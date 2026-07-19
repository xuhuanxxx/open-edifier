use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde_json::{Map, Value, json};

use open_edifier_core::{Device, DeviceStatus, PlaybackAction, Source};

use crate::{
    Error, FrameDecoder, Result, SpeakerStatus, model::source_index, protocol::encode_request,
};

/// Verified default TCP control port for the S260.
pub const DEFAULT_PORT: u16 = 8080;
static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Connection and protocol settings for an S260 client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Hostname or IP address of the speaker.
    pub host: String,
    /// TCP control port.
    pub port: u16,
    /// Maximum connection and request duration.
    pub timeout: Duration,
}

impl ClientConfig {
    /// Creates a configuration using verified S260 defaults.
    pub fn new(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: DEFAULT_PORT,
            timeout: Duration::from_secs(5),
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
        let stream = connect_socket(&config)?;
        Ok(Self {
            stream,
            decoder: FrameDecoder::new(),
            config,
        })
    }

    /// Reads and parses the current speaker state.
    pub fn status(&mut self) -> Result<SpeakerStatus> {
        SpeakerStatus::from_value(self.request("status_query", Map::new())?)
    }

    /// Selects an input and verifies the resulting speaker state.
    pub fn set_source(&mut self, source: Source) -> Result<SpeakerStatus> {
        let current = self.status()?;
        let settings = Map::from_iter([(
            "inputSource".to_owned(),
            json!({
                "inputIndex": current.input_index,
                "selectedIndex": source_index(&source)?,
            }),
        )]);
        self.request("settings", settings)?;
        let updated = self.status()?;
        if updated.source != source {
            return Err(Error::Verification {
                expected: source.to_string(),
                actual: updated.source.to_string(),
            });
        }
        Ok(updated)
    }

    /// Sets a device-bounded volume and verifies the resulting state.
    pub fn set_volume(&mut self, volume: u8) -> Result<SpeakerStatus> {
        let current = self.status()?;
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
        let updated = self.status()?;
        if updated.volume != volume {
            return Err(Error::Verification {
                expected: volume.to_string(),
                actual: updated.volume.to_string(),
            });
        }
        Ok(updated)
    }

    /// Selects an equalizer preset and verifies the resulting state.
    pub fn set_eq_preset(&mut self, preset: u8) -> Result<SpeakerStatus> {
        let current = self.status()?;
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
        let updated = self.status()?;
        let actual = updated
            .equalizer
            .as_ref()
            .ok_or(Error::MissingField("soundEffect.selectedIndex"))?
            .preset;
        if actual != preset {
            return Err(Error::Verification {
                expected: preset.to_string(),
                actual: actual.to_string(),
            });
        }
        Ok(updated)
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
        let request_id = request_id();
        let mut request = Map::from_iter([
            ("id".to_owned(), Value::String(request_id.clone())),
            ("payload".to_owned(), Value::String(payload.to_owned())),
        ]);
        request.extend(settings);
        let bytes = encode_request(&Value::Object(request))?;
        self.stream.write_all(&bytes)?;

        let deadline = std::time::Instant::now() + self.config.timeout;
        let mut buffer = [0_u8; 8192];
        while std::time::Instant::now() < deadline {
            match self.stream.read(&mut buffer) {
                Ok(0) => return Err(Error::Protocol("speaker closed the connection".into())),
                Ok(size) => {
                    for response in self.decoder.feed(&buffer[..size])? {
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
                            return Err(Error::Rejected(response.to_string()));
                        }
                        return Ok(response);
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
            self.config.timeout
        )))
    }
}

pub(crate) fn connect_socket(config: &ClientConfig) -> Result<TcpStream> {
    if config.timeout.is_zero() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "timeout must be greater than zero",
        )
        .into());
    }
    let deadline = Instant::now() + config.timeout;
    let addresses = (config.host.as_str(), config.port).to_socket_addrs()?;
    let mut last_error = None;
    let mut stream = None;
    for address in addresses {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
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
                format!("could not connect within {:?}", config.timeout),
            )
        })
    })?;
    stream.set_read_timeout(Some(Duration::from_millis(500)))?;
    stream.set_write_timeout(Some(config.timeout))?;
    Ok(stream)
}

impl Device for Client {
    fn status(&mut self) -> open_edifier_core::Result<DeviceStatus> {
        Client::status(self).map(Into::into).map_err(core_error)
    }

    fn set_source(&mut self, source: Source) -> open_edifier_core::Result<DeviceStatus> {
        Client::set_source(self, source)
            .map(Into::into)
            .map_err(core_error)
    }

    fn set_volume(&mut self, volume: u8) -> open_edifier_core::Result<DeviceStatus> {
        Client::set_volume(self, volume)
            .map(Into::into)
            .map_err(core_error)
    }

    fn set_eq_preset(&mut self, preset: u8) -> open_edifier_core::Result<DeviceStatus> {
        Client::set_eq_preset(self, preset)
            .map(Into::into)
            .map_err(core_error)
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
