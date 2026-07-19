use std::{
    collections::VecDeque,
    io::Read,
    net::TcpStream,
    time::{Duration, Instant},
};

use open_edifier_aaec::{Frame, FrameDecoder};
use open_edifier_core::{DeviceEvent, DeviceEvents, Source};

use crate::{ClientConfig, Result, client::connect_socket, model::playback_state};

const INPUT_EVENT: u16 = 0x0061;
const VOLUME_EVENT: u16 = 0x0066;
const PLAYBACK_EVENT: u16 = 0x0068;
const TRACK_INFO_EVENT: u16 = 0x0050;
const EQ_EVENT: u16 = 0x00d5;
const EQ_ACK_EVENT: u16 = 0x00c4;
const INPUT_SUBCOMMAND: u8 = 0x1e;
const INITIAL_RECONNECT_DELAY: Duration = Duration::from_millis(200);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(5);

/// Blocking S260 state-event stream backed by the `BB EC` push channel.
///
/// Once the initial connection succeeds, transient disconnects are retried
/// with bounded exponential backoff. Read timeouts and reconnect waits return
/// `None`, allowing callers to check their own cancellation condition.
pub struct EventStream {
    config: ClientConfig,
    stream: Option<TcpStream>,
    decoder: FrameDecoder,
    pending: VecDeque<DeviceEvent>,
    reconnect_at: Instant,
    reconnect_delay: Duration,
}

impl EventStream {
    /// Opens the S260 binary push channel using the same host and timeout settings.
    pub fn connect(config: ClientConfig) -> Result<Self> {
        let stream = connect_socket(&config)?;
        Ok(Self {
            config,
            stream: Some(stream),
            decoder: FrameDecoder::new(),
            pending: VecDeque::new(),
            reconnect_at: Instant::now(),
            reconnect_delay: INITIAL_RECONNECT_DELAY,
        })
    }

    /// Returns the next decoded state event, or `None` after a read interval.
    pub fn next_event(&mut self) -> Result<Option<DeviceEvent>> {
        if let Some(event) = self.pending.pop_front() {
            return Ok(Some(event));
        }

        if !self.reconnect_if_due() {
            return Ok(None);
        }

        let mut buffer = [0_u8; 4096];
        loop {
            let result = self
                .stream
                .as_mut()
                .expect("event stream is connected")
                .read(&mut buffer);
            match result {
                Ok(0) => {
                    self.schedule_reconnect();
                    return Ok(None);
                }
                Ok(size) => {
                    for frame in self.decoder.feed(&buffer[..size]) {
                        if frame.is_heartbeat() {
                            continue;
                        }
                        self.pending.push_back(decode_event(frame));
                    }
                    if let Some(event) = self.pending.pop_front() {
                        return Ok(Some(event));
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    return Ok(None);
                }
                Err(_) => {
                    self.schedule_reconnect();
                    return Ok(None);
                }
            }
        }
    }

    fn reconnect_if_due(&mut self) -> bool {
        if self.stream.is_some() {
            return true;
        }
        if Instant::now() < self.reconnect_at {
            return false;
        }
        match connect_socket(&self.config) {
            Ok(stream) => {
                self.stream = Some(stream);
                self.decoder = FrameDecoder::new();
                self.reconnect_delay = INITIAL_RECONNECT_DELAY;
                true
            }
            Err(_) => {
                self.schedule_reconnect();
                false
            }
        }
    }

    fn schedule_reconnect(&mut self) {
        self.stream = None;
        self.decoder = FrameDecoder::new();
        self.reconnect_at = Instant::now() + self.reconnect_delay;
        self.reconnect_delay = self
            .reconnect_delay
            .saturating_mul(2)
            .min(MAX_RECONNECT_DELAY);
    }
}

impl DeviceEvents for EventStream {
    fn next_event(&mut self) -> open_edifier_core::Result<Option<DeviceEvent>> {
        EventStream::next_event(self).map_err(Into::into)
    }
}

fn decode_event(frame: Frame) -> DeviceEvent {
    match (frame.command, frame.payload.as_slice()) {
        (INPUT_EVENT, [INPUT_SUBCOMMAND, source, ..]) => DeviceEvent::Source {
            source: binary_source(*source),
        },
        (VOLUME_EVENT, [max, current, ..]) => DeviceEvent::Volume {
            current: *current,
            max: *max,
        },
        (PLAYBACK_EVENT, [state, ..]) => DeviceEvent::Playback {
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
    }
}

fn binary_source(index: u8) -> Source {
    Source::new(match index {
        1 => Source::BLUETOOTH,
        2 => Source::AUX,
        3 => Source::USB,
        4 => Source::AIRPLAY,
        other => return Source::new(format!("unknown_{other}")),
    })
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
            }),
            DeviceEvent::Source {
                source: Source::new(Source::AIRPLAY),
            }
        );
        assert_eq!(
            decode_event(Frame {
                command: VOLUME_EVENT,
                payload: vec![30, 18],
            }),
            DeviceEvent::Volume {
                current: 18,
                max: 30,
            }
        );
        assert!(matches!(
            decode_event(Frame {
                command: 0x9999,
                payload: vec![1, 2],
            }),
            DeviceEvent::Unknown {
                command: 0x9999,
                ..
            }
        ));
    }
}
