use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde_json::{Map, Value, json};

use open_edifier_core::{Device, DeviceStatus, PlaybackAction, Source};

use crate::{
    Error, Result,
    model::{S260Status, source_index},
    protocol::{FrameDecoder, encode_request},
};

/// Verified default TCP control port for the S260.
pub const DEFAULT_PORT: u16 = 8080;
static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const READ_SLICE: Duration = Duration::from_millis(500);

/// Connection and protocol settings for an S260 client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Hostname or IP address of the speaker.
    pub host: String,
    /// TCP control port.
    pub port: u16,
    /// Maximum connection duration.
    pub connect_timeout: Duration,
    /// Maximum duration for one request and response.
    pub request_timeout: Duration,
    /// Maximum duration for write-after-read verification.
    pub verification_timeout: Duration,
    /// Delay between verification reads.
    pub verification_interval: Duration,
}

impl ClientConfig {
    /// Creates a configuration using verified S260 defaults.
    pub fn new(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: DEFAULT_PORT,
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(5),
            verification_timeout: Duration::from_secs(1),
            verification_interval: Duration::from_millis(50),
        }
    }
}

/// Stateful TCP client for one S260 speaker.
pub struct Client {
    stream: TcpStream,
    decoder: FrameDecoder,
    config: ClientConfig,
}

impl Client {
    /// Connects to an S260 using the supplied bounded timeout.
    pub fn connect(config: ClientConfig) -> Result<Self> {
        validate_config(&config)?;
        let stream = connect_socket(&config)?;
        Ok(Self {
            stream,
            decoder: FrameDecoder::new(),
            config,
        })
    }

    /// Reads and parses the current speaker state.
    pub fn status(&mut self) -> Result<DeviceStatus> {
        self.status_wire().map(Into::into)
    }

    /// Selects an input and verifies the resulting speaker state.
    pub fn set_source(&mut self, source: Source) -> Result<DeviceStatus> {
        let current = self.status_wire()?;
        let settings = Map::from_iter([(
            "inputSource".to_owned(),
            json!({
                "inputIndex": current.input_index,
                "selectedIndex": source_index(&source)?,
            }),
        )]);
        self.request("settings", settings)?;
        self.verify_state(
            "source",
            source.to_string(),
            |status| status.source == source,
            |status| status.source.to_string(),
        )
        .map(Into::into)
    }

    /// Sets a device-bounded volume and verifies the resulting state.
    pub fn set_volume(&mut self, volume: u8) -> Result<DeviceStatus> {
        let current = self.status_wire()?;
        if !(current.min_volume..=current.max_volume).contains(&volume) {
            return Err(Error::InvalidVolume {
                value: volume,
                min: current.min_volume,
                max: current.max_volume,
            });
        }
        self.request(
            "settings",
            Map::from_iter([("player".to_owned(), json!({"volume": volume}))]),
        )?;
        self.verify_state(
            "volume",
            volume.to_string(),
            |status| status.volume == volume,
            |status| status.volume.to_string(),
        )
        .map(Into::into)
    }

    /// Selects an equalizer preset and verifies the resulting state.
    pub fn set_eq_preset(&mut self, preset: u8) -> Result<DeviceStatus> {
        let current = self.status_wire()?;
        let equalizer = current
            .equalizer
            .ok_or(Error::MissingField("soundEffect"))?;
        if preset >= equalizer.preset_count {
            return Err(Error::InvalidEqPreset {
                value: preset,
                preset_count: equalizer.preset_count,
            });
        }
        self.request(
            "settings",
            Map::from_iter([("soundEffect".to_owned(), json!({"selectedIndex": preset}))]),
        )?;
        self.verify_state(
            "equalizer",
            preset.to_string(),
            |status| {
                status
                    .equalizer
                    .as_ref()
                    .is_some_and(|eq| eq.preset == preset)
            },
            |status| {
                status
                    .equalizer
                    .as_ref()
                    .map(|eq| eq.preset.to_string())
                    .unwrap_or_else(|| "missing".to_owned())
            },
        )
        .map(Into::into)
    }

    /// Sends a playback command acknowledged by the speaker.
    pub fn playback(&mut self, action: PlaybackAction) -> Result<()> {
        let player = match action {
            PlaybackAction::Play => json!({"playerStatus": 1}),
            PlaybackAction::Pause => json!({"playerStatus": 0}),
            PlaybackAction::Next => json!({"next": 1}),
            PlaybackAction::Previous => json!({"previous": 1}),
        };
        self.request("settings", Map::from_iter([("player".to_owned(), player)]))?;
        Ok(())
    }

    fn request(&mut self, payload: &str, settings: Map<String, Value>) -> Result<Value> {
        self.request_with_timeout(payload, settings, self.config.request_timeout)
    }

    fn request_with_timeout(
        &mut self,
        payload: &str,
        settings: Map<String, Value>,
        timeout: Duration,
    ) -> Result<Value> {
        let deadline = checked_deadline(timeout, "request timeout")?;
        let request_id = request_id();
        let mut request = Map::from_iter([
            ("id".to_owned(), Value::String(request_id.clone())),
            ("payload".to_owned(), Value::String(payload.to_owned())),
        ]);
        request.extend(settings);
        let bytes = encode_request(&Value::Object(request))?;
        self.stream.set_write_timeout(Some(timeout))?;
        self.stream.write_all(&bytes)?;

        let mut buffer = [0_u8; 8192];
        while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
            if remaining.is_zero() {
                break;
            }
            self.stream
                .set_read_timeout(Some(remaining.min(READ_SLICE)))?;
            match self.stream.read(&mut buffer) {
                Ok(0) => return Err(Error::Protocol("speaker closed the connection".into())),
                Ok(size) => {
                    let mut malformed = None;
                    for response in self.decoder.feed(&buffer[..size]) {
                        let response = match response {
                            Ok(response) => response,
                            Err(error) => {
                                malformed.get_or_insert(error);
                                continue;
                            }
                        };
                        if response["id"].as_str() != Some(&request_id) {
                            continue;
                        }
                        let code = response["code"].as_i64().ok_or_else(|| {
                            Error::Protocol("response did not contain an integer code".into())
                        })?;
                        let message = response["message"].as_str().ok_or_else(|| {
                            Error::Protocol("response did not contain a message".into())
                        })?;
                        if code != 0 || message != "success" {
                            return Err(Error::Rejected {
                                code,
                                message: sanitize_message(message),
                            });
                        }
                        return Ok(response);
                    }
                    if let Some(error) = malformed {
                        return Err(error);
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(error) => return Err(error.into()),
            }
        }
        Err(Error::Protocol(format!(
            "speaker did not answer request {request_id} within {:?}",
            timeout
        )))
    }

    fn status_wire(&mut self) -> Result<S260Status> {
        S260Status::from_value(self.request("status_query", Map::new())?)
    }

    fn status_wire_with_timeout(&mut self, timeout: Duration) -> Result<S260Status> {
        S260Status::from_value(self.request_with_timeout("status_query", Map::new(), timeout)?)
    }

    fn verify_state(
        &mut self,
        field: &'static str,
        expected: String,
        matches: impl Fn(&S260Status) -> bool,
        actual: impl Fn(&S260Status) -> String,
    ) -> Result<S260Status> {
        let started = Instant::now();
        let deadline = checked_deadline(self.config.verification_timeout, "verification timeout")?;
        let mut attempts = 0_u32;
        let mut last_actual = "unobserved".to_owned();
        loop {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return Err(Error::VerificationTimeout {
                    field,
                    expected,
                    actual: last_actual,
                    attempts,
                    elapsed_ms: elapsed_millis(started),
                });
            };
            if remaining.is_zero() {
                return Err(Error::VerificationTimeout {
                    field,
                    expected,
                    actual: last_actual,
                    attempts,
                    elapsed_ms: elapsed_millis(started),
                });
            }
            attempts = attempts.saturating_add(1);
            let status =
                match self.status_wire_with_timeout(remaining.min(self.config.request_timeout)) {
                    Ok(status) => status,
                    Err(_)
                        if deadline
                            .checked_duration_since(Instant::now())
                            .is_none_or(|remaining| remaining.is_zero()) =>
                    {
                        return Err(Error::VerificationTimeout {
                            field,
                            expected,
                            actual: last_actual,
                            attempts,
                            elapsed_ms: elapsed_millis(started),
                        });
                    }
                    Err(error) => return Err(error),
                };
            if matches(&status) {
                return Ok(status);
            }
            last_actual = actual(&status);
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return Err(Error::VerificationTimeout {
                    field,
                    expected,
                    actual: last_actual,
                    attempts,
                    elapsed_ms: elapsed_millis(started),
                });
            };
            thread::sleep(remaining.min(self.config.verification_interval));
        }
    }
}

pub(crate) fn connect_socket(config: &ClientConfig) -> Result<TcpStream> {
    connect_socket_with_timeout(config, config.connect_timeout)
}

pub(crate) fn connect_socket_with_timeout(
    config: &ClientConfig,
    timeout: Duration,
) -> Result<TcpStream> {
    let deadline = checked_deadline(timeout, "connection timeout")?;
    let addresses = (config.host.as_str(), config.port).to_socket_addrs()?;
    let mut last_error = None;
    let mut stream = None;
    for address in addresses {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        if remaining.is_zero() {
            break;
        }
        match TcpStream::connect_timeout(&address, remaining) {
            Ok(connected) => {
                stream = Some(connected);
                break;
            }
            Err(error) => last_error = Some(error),
        }
    }
    let stream = stream.ok_or_else(|| {
        last_error.unwrap_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("could not connect within {timeout:?}"),
            )
        })
    })?;
    stream.set_read_timeout(Some(config.request_timeout.min(READ_SLICE)))?;
    stream.set_write_timeout(Some(config.request_timeout))?;
    Ok(stream)
}

impl Device for Client {
    fn status(&mut self) -> open_edifier_core::Result<DeviceStatus> {
        Client::status(self).map_err(core_error)
    }

    fn set_source(&mut self, source: Source) -> open_edifier_core::Result<DeviceStatus> {
        Client::set_source(self, source).map_err(core_error)
    }

    fn set_volume(&mut self, volume: u8) -> open_edifier_core::Result<DeviceStatus> {
        Client::set_volume(self, volume).map_err(core_error)
    }

    fn set_eq_preset(&mut self, preset: u8) -> open_edifier_core::Result<DeviceStatus> {
        Client::set_eq_preset(self, preset).map_err(core_error)
    }

    fn playback(&mut self, action: PlaybackAction) -> open_edifier_core::Result<()> {
        Client::playback(self, action).map_err(core_error)
    }
}

fn core_error(error: Error) -> open_edifier_core::Error {
    error.into()
}

fn request_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let sequence = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed) % 1000;
    format!("{millis}{sequence:03}")
}

pub(crate) fn validate_config(config: &ClientConfig) -> Result<()> {
    let invalid = config.connect_timeout.is_zero()
        || config.request_timeout.is_zero()
        || config.verification_timeout.is_zero()
        || config.verification_interval.is_zero()
        || config.verification_interval > config.verification_timeout;
    if invalid {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "timeouts must be positive and verification_interval must not exceed verification_timeout",
        )
        .into());
    }
    Ok(())
}

fn checked_deadline(timeout: Duration, field: &str) -> Result<Instant> {
    Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| Error::Protocol(format!("{field} is too large")))
}

fn elapsed_millis(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn sanitize_message(message: &str) -> String {
    message
        .chars()
        .filter(|character| !character.is_control())
        .take(160)
        .collect()
}
