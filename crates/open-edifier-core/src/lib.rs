//! Stable, model-independent contracts shared by OpenEdifier drivers and apps.
#![warn(missing_docs)]

use std::{fmt, net::IpAddr, str::FromStr, time::Duration};

use serde::{Deserialize, Serialize};

/// Stable, string-backed identifier for a speaker model.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModelId(String);

impl ModelId {
    /// Creates a normalized lowercase model identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().to_ascii_lowercase())
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ModelId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Extensible, string-backed input source identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Source(String);

impl Source {
    /// Bluetooth input source identifier.
    pub const BLUETOOTH: &'static str = "bluetooth";
    /// Analog auxiliary input source identifier.
    pub const AUX: &'static str = "aux";
    /// USB audio input source identifier.
    pub const USB: &'static str = "usb";
    /// AirPlay input source identifier.
    pub const AIRPLAY: &'static str = "airplay";

    /// Creates a normalized source identifier and resolves common aliases.
    pub fn new(value: impl AsRef<str>) -> Self {
        let normalized = value.as_ref().to_ascii_lowercase().replace(['-', '_'], "");
        let canonical = match normalized.as_str() {
            "bt" | "bluetooth" => Self::BLUETOOTH,
            "aux" | "linein" => Self::AUX,
            "usb" => Self::USB,
            "airplay" | "airplay2" => Self::AIRPLAY,
            _ => value.as_ref(),
        };
        Self(canonical.to_ascii_lowercase())
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Source {
    type Err = std::convert::Infallible;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self::new(value))
    }
}

impl fmt::Display for Source {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A local-network speaker candidate returned by discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    /// Stable device identifier advertised by the speaker.
    pub id: String,
    /// Human-readable advertised device name.
    pub name: String,
    /// Canonical model or advertised hardware identifier.
    pub model: ModelId,
    /// Advertised local hostname.
    pub host: String,
    /// Advertised IP addresses.
    pub addresses: Vec<IpAddr>,
}

/// Current volume and the device-reported valid range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Volume {
    /// Current volume level.
    pub current: u8,
    /// Minimum accepted volume level.
    pub min: u8,
    /// Maximum accepted volume level.
    pub max: u8,
}

/// Current equalizer preset and the number of reported presets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Equalizer {
    /// Selected zero-based preset index.
    pub preset: u8,
    /// Number of presets reported by the device.
    pub preset_count: u8,
}

/// Model-independent capabilities reported by a driver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    /// Input sources accepted by [`Device::set_source`].
    pub sources: Vec<Source>,
    /// Whether the device supports volume mutations.
    pub volume: bool,
    /// Whether the device supports equalizer preset mutations.
    pub equalizer: bool,
    /// Whether the device accepts playback commands.
    pub playback: bool,
    /// Whether the driver provides a state-event stream.
    pub events: bool,
}

/// Playback command accepted by media-capable drivers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackAction {
    /// Start or resume playback.
    Play,
    /// Pause playback.
    Pause,
    /// Skip to the next track.
    Next,
    /// Return to the previous track.
    Previous,
}

/// Extensible playback state reported by a device or event stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlaybackState(String);

impl PlaybackState {
    /// Stopped playback state.
    pub const STOPPED: &'static str = "stopped";
    /// Active playback state.
    pub const PLAYING: &'static str = "playing";
    /// Paused playback state.
    pub const PAUSED: &'static str = "paused";

    /// Creates a normalized playback state.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().to_ascii_lowercase())
    }

    /// Returns the state as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlaybackState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Model-independent speaker state returned by every driver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceStatus {
    /// Human-readable speaker name.
    pub name: String,
    /// Canonical model identifier.
    pub model: ModelId,
    /// Device firmware version.
    pub firmware: String,
    /// Current input source when supported and reported.
    pub source: Option<Source>,
    /// Current volume state when supported and reported.
    pub volume: Option<Volume>,
    /// Equalizer state when supported and reported.
    pub equalizer: Option<Equalizer>,
    /// Playback state when reported by the current source.
    pub playback: Option<PlaybackState>,
    /// Stable capabilities projected by the selected model driver.
    pub capabilities: DeviceCapabilities,
}

/// Synchronous control contract implemented by every model driver.
pub trait Device: Send {
    /// Reads current speaker state.
    fn status(&mut self) -> Result<DeviceStatus>;
    /// Selects an input source and returns verified state.
    fn set_source(&mut self, _source: Source) -> Result<DeviceStatus> {
        Err(Error::UnsupportedCapability("source"))
    }
    /// Sets volume and returns verified state.
    fn set_volume(&mut self, _volume: u8) -> Result<DeviceStatus> {
        Err(Error::UnsupportedCapability("volume"))
    }
    /// Selects an equalizer preset and returns verified state.
    fn set_eq_preset(&mut self, _preset: u8) -> Result<DeviceStatus> {
        Err(Error::UnsupportedCapability("equalizer"))
    }
    /// Sends a media playback command.
    fn playback(&mut self, _action: PlaybackAction) -> Result<()> {
        Err(Error::UnsupportedCapability("playback"))
    }
}

/// Typed state change emitted by a model event channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeviceEvent {
    /// Input source changed.
    Source {
        /// Newly selected source.
        source: Source,
    },
    /// Volume changed.
    Volume {
        /// Current volume.
        current: u8,
        /// Device-reported maximum.
        max: u8,
    },
    /// Playback state changed.
    Playback {
        /// Newly reported playback state.
        state: PlaybackState,
    },
    /// Equalizer preset changed.
    Equalizer {
        /// Newly selected preset.
        preset: u8,
    },
    /// Track metadata event whose vendor payload is not decoded yet.
    TrackInfo {
        /// Undecoded vendor metadata bytes.
        payload: Vec<u8>,
    },
    /// Valid but currently unknown command.
    Unknown {
        /// Vendor command identifier.
        command: u16,
        /// Undecoded vendor payload.
        payload: Vec<u8>,
    },
}

/// Blocking event stream implemented by devices with a push channel.
pub trait DeviceEvents: Send {
    /// Returns the next state event, or `None` after waiting up to `max_wait`.
    fn next_event(&mut self, max_wait: Duration) -> Result<Option<DeviceEvent>>;
}

/// Model-independent failures suitable for CLI and future language bindings.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Local discovery could not start or complete.
    #[error("device discovery failed: {0}")]
    Discovery(String),
    /// No driver is installed for the discovered model.
    #[error("model {0} does not have an installed driver")]
    UnsupportedModel(ModelId),
    /// No supported speaker matched discovery or selection.
    #[error("no supported EDIFIER speaker was found")]
    DeviceNotFound,
    /// More than one supported speaker requires explicit selection.
    #[error("multiple supported speakers were found: {candidates:?}")]
    AmbiguousDevice {
        /// Human-readable candidate labels.
        candidates: Vec<String>,
    },
    /// A network operation failed.
    #[error("network error during {operation}: {message}")]
    Network {
        /// Operation that failed.
        operation: &'static str,
        /// Sanitized failure description.
        message: String,
    },
    /// The device returned malformed or unexpected protocol data.
    #[error("protocol error: {message}")]
    Protocol {
        /// Sanitized protocol failure description.
        message: String,
    },
    /// The device explicitly rejected a command.
    #[error("speaker rejected command with code {code}: {message}")]
    Rejected {
        /// Device result code.
        code: i64,
        /// Sanitized device message.
        message: String,
    },
    /// The requested source is not supported by the selected model.
    #[error("source {0:?} is not supported by this speaker")]
    UnsupportedSource(Source),
    /// A requested volume is outside the reported valid range.
    #[error("volume {value} is outside the supported range {min}..={max}")]
    InvalidVolume {
        /// Requested volume.
        value: u8,
        /// Device-reported minimum.
        min: u8,
        /// Device-reported maximum.
        max: u8,
    },
    /// A mutation was acknowledged but did not reach the requested state in time.
    #[error(
        "{field} did not reach {expected} within {elapsed_ms} ms after {attempts} checks; last observed {actual}"
    )]
    VerificationTimeout {
        /// State field being verified.
        field: &'static str,
        /// Requested value.
        expected: String,
        /// Last value reported during verification.
        actual: String,
        /// Number of state queries made.
        attempts: u32,
        /// Elapsed verification time in milliseconds.
        elapsed_ms: u64,
    },
    /// Machine-readable output could not be serialized.
    #[error("serialization failed: {0}")]
    Serialization(String),
    /// Selected device does not implement the requested capability.
    #[error("speaker does not support {0}")]
    UnsupportedCapability(&'static str),
    /// Equalizer preset is outside the reported valid range.
    #[error("equalizer preset {value} is outside 0..{preset_count}")]
    InvalidEqPreset {
        /// Requested preset index.
        value: u8,
        /// Number of presets reported by the device.
        preset_count: u8,
    },
}

/// Convenience result type shared by OpenEdifier crates.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    struct StatusOnlyDevice;

    impl Device for StatusOnlyDevice {
        fn status(&mut self) -> Result<DeviceStatus> {
            Ok(DeviceStatus {
                name: "Status only".to_owned(),
                model: ModelId::new("test"),
                firmware: "test".to_owned(),
                source: None,
                volume: None,
                equalizer: None,
                playback: None,
                capabilities: DeviceCapabilities {
                    sources: Vec::new(),
                    volume: false,
                    equalizer: false,
                    playback: false,
                    events: false,
                },
            })
        }
    }

    #[test]
    fn source_aliases_are_canonical() {
        assert_eq!(Source::new("BT").as_str(), Source::BLUETOOTH);
        assert_eq!(Source::new("air-play2").as_str(), Source::AIRPLAY);
        assert_eq!(Source::new("optical").as_str(), "optical");
    }

    #[test]
    fn optional_mutations_default_to_unsupported() {
        let mut device = StatusOnlyDevice;
        assert!(matches!(
            device.set_source(Source::new("test")),
            Err(Error::UnsupportedCapability("source"))
        ));
        assert!(matches!(
            device.set_volume(1),
            Err(Error::UnsupportedCapability("volume"))
        ));
    }
}
