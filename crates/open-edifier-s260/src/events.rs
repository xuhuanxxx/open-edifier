use std::{
    collections::VecDeque,
    io::Read,
    net::TcpStream,
    thread,
    time::{Duration, Instant},
};

use open_edifier_aaec::{Frame, FrameDecoder};
use open_edifier_core::{DeviceEvent, DeviceEvents, Source};

use crate::{
    ClientConfig, Error, Result,
    client::{connect_socket, connect_socket_with_timeout, validate_config},
    model::playback_state,
};

const INPUT_EVENT: u16 = 0x0061;
const VOLUME_EVENT: u16 = 0x0066;
const PLAYBACK_EVENT: u16 = 0x0068;
const TRACK_INFO_EVENT: u16 = 0x0050;
const EQ_EVENT: u16 = 0x00d5;
const EQ_ACK_EVENT: u16 = 0x00c4;
const INPUT_SUBCOMMAND: u8 = 0x1e;
const INITIAL_RECONNECT_DELAY: Duration = Duration::from_millis(200);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(5);
const READ_SLICE: Duration = Duration::from_millis(500);

/// Blocking S260 state-event stream backed by the `BB EC` push channel.
///
/// Once the initial connection succeeds, transient disconnects are retried
/// with bounded exponential backoff. The caller supplies a maximum wait so it
/// can check cancellation without busy-looping; failed reconnects preserve the
/// last network error.
pub struct EventStream {
    config: ClientConfig,
    stream: Option<TcpStream>,
    decoder: FrameDecoder,
    pending: VecDeque<DeviceEvent>,
    reconnect_at: Instant,
    reconnect_delay: Duration,
    last_error: Option<String>,
}

impl EventStream {
    /// Opens the S260 binary push channel using the same host and timeout settings.
    pub fn connect(config: ClientConfig) -> Result<Self> {
        validate_config(&config)?;
        let stream = connect_socket(&config)?;
        Ok(Self {
            config,
            stream: Some(stream),
            decoder: FrameDecoder::new(),
            pending: VecDeque::new(),
            reconnect_at: Instant::now(),
            reconnect_delay: INITIAL_RECONNECT_DELAY,
            last_error: None,
        })
    }

    /// Returns the next decoded state event, or `None` after waiting up to `max_wait`.
    pub fn next_event(&mut self, max_wait: Duration) -> Result<Option<DeviceEvent>> {
        if max_wait.is_zero() {
            return Err(Error::Protocol(
                "event wait must be greater than zero".into(),
            ));
        }
        let deadline = Instant::now()
            .checked_add(max_wait)
            .ok_or_else(|| Error::Protocol("event wait is too large".into()))?;

        loop {
            if let Some(event) = self.pending.pop_front() {
                return Ok(Some(event));
            }

            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return self.wait_expired();
            };
            if remaining.is_zero() {
                return self.wait_expired();
            }

            if self.stream.is_none() {
                if Instant::now() < self.reconnect_at {
                    let until_reconnect = self.reconnect_at.duration_since(Instant::now());
                    thread::sleep(remaining.min(until_reconnect));
                    continue;
                }
                match connect_socket_with_timeout(
                    &self.config,
                    remaining.min(self.config.connect_timeout),
                ) {
                    Ok(stream) => {
                        self.stream = Some(stream);
                        self.decoder = FrameDecoder::new();
                        self.reconnect_delay = INITIAL_RECONNECT_DELAY;
                        self.last_error = None;
                    }
                    Err(error) => {
                        self.schedule_reconnect(error.to_string());
                    }
                }
                continue;
            }

            self.stream
                .as_ref()
                .expect("event stream is connected")
                .set_read_timeout(Some(remaining.min(READ_SLICE)))?;
            let mut buffer = [0_u8; 4096];
            let result = self
                .stream
                .as_mut()
                .expect("event stream is connected")
                .read(&mut buffer);
            match result {
                Ok(0) => {
                    self.schedule_reconnect("speaker closed the connection".into());
                }
                Ok(size) => {
                    for frame in self.decoder.feed(&buffer[..size]) {
                        if frame.is_heartbeat() {
                            continue;
                        }
                        self.pending.push_back(decode_event(frame)?);
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(error) => {
                    self.schedule_reconnect(error.to_string());
                }
            }
        }
    }

    fn wait_expired(&self) -> Result<Option<DeviceEvent>> {
        match (&self.stream, &self.last_error) {
            (None, Some(error)) => Err(Error::Reconnect(error.clone())),
            _ => Ok(None),
        }
    }

    fn schedule_reconnect(&mut self, error: String) {
        self.stream = None;
        self.decoder = FrameDecoder::new();
        self.last_error = Some(error);
        self.reconnect_at = Instant::now()
            .checked_add(self.reconnect_delay)
            .unwrap_or_else(Instant::now);
        self.reconnect_delay = self
            .reconnect_delay
            .saturating_mul(2)
            .min(MAX_RECONNECT_DELAY);
    }
}

impl DeviceEvents for EventStream {
    fn next_event(&mut self, max_wait: Duration) -> open_edifier_core::Result<Option<DeviceEvent>> {
        EventStream::next_event(self, max_wait).map_err(Into::into)
    }
}

fn decode_event(frame: Frame) -> Result<DeviceEvent> {
    let event = match (frame.command, frame.payload.as_slice()) {
        (INPUT_EVENT, [INPUT_SUBCOMMAND, source, ..]) => match binary_source(*source) {
            Some(source) => DeviceEvent::Source { source },
            None => DeviceEvent::Unknown {
                command: frame.command,
                payload: frame.payload,
            },
        },
        (VOLUME_EVENT, [max, current, ..]) if current <= max => DeviceEvent::Volume {
            current: *current,
            max: *max,
        },
        (VOLUME_EVENT, [max, current, ..]) => {
            return Err(Error::Protocol(format!(
                "invalid volume event: current={current}, max={max}"
            )));
        }
        (PLAYBACK_EVENT, [state @ 0..=2, ..]) => DeviceEvent::Playback {
            state: playback_state(u64::from(*state)),
        },
        (EQ_EVENT | EQ_ACK_EVENT, [preset, ..]) => DeviceEvent::Equalizer { preset: *preset },
        (TRACK_INFO_EVENT, payload) => DeviceEvent::TrackInfo {
            payload: payload.to_vec(),
        },
        (command, payload) => DeviceEvent::Unknown {
            command,
            payload: payload.to_vec(),
        },
    };
    Ok(event)
}

fn binary_source(index: u8) -> Option<Source> {
    Some(Source::new(match index {
        1 => Source::BLUETOOTH,
        2 => Source::AUX,
        3 => Source::USB,
        4 => Source::AIRPLAY,
        _ => return None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_verified_source_volume_and_unknown_events() {
        assert_eq!(
            decode_event(Frame {
                command: INPUT_EVENT,
                payload: vec![INPUT_SUBCOMMAND, 4],
            })
            .unwrap(),
            DeviceEvent::Source {
                source: Source::new(Source::AIRPLAY),
            }
        );
        assert_eq!(
            decode_event(Frame {
                command: VOLUME_EVENT,
                payload: vec![30, 18],
            })
            .unwrap(),
            DeviceEvent::Volume {
                current: 18,
                max: 30,
            }
        );
        assert!(matches!(
            decode_event(Frame {
                command: 0x9999,
                payload: vec![1, 2],
            })
            .unwrap(),
            DeviceEvent::Unknown {
                command: 0x9999,
                ..
            }
        ));
    }

    #[test]
    fn rejects_impossible_volume_events() {
        assert!(matches!(
            decode_event(Frame {
                command: VOLUME_EVENT,
                payload: vec![30, 31],
            }),
            Err(Error::Protocol(message)) if message.contains("volume event")
        ));
    }
}
