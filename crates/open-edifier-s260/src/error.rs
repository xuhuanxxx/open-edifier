use std::io;

/// Failures reported by the S260 protocol driver.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Socket or address resolution failure.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    /// Malformed JSON frame.
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// Malformed or unexpected protocol data.
    #[error("invalid protocol frame: {0}")]
    Protocol(String),
    /// Explicit command rejection from the speaker.
    #[error("speaker rejected command: {0}")]
    Rejected(String),
    /// Required response field was absent.
    #[error("speaker response did not contain {0}")]
    MissingField(&'static str),
    /// Device reported an unknown numeric source.
    #[error("invalid source index {0}")]
    InvalidSource(u64),
    /// Caller requested a source unsupported by S260.
    #[error("source {0:?} is not supported by the S260")]
    UnsupportedSource(String),
    /// Requested volume was outside the device-reported range.
    #[error("volume {value} is outside the supported range {min}..={max}")]
    InvalidVolume {
        /// Requested volume.
        value: u8,
        /// Device-reported minimum.
        min: u8,
        /// Device-reported maximum.
        max: u8,
    },
    /// Device state did not reflect an acknowledged mutation.
    #[error("speaker acknowledged the command but reported {actual} instead of {expected}")]
    Verification {
        /// Requested value.
        expected: String,
        /// Value reported after the mutation.
        actual: String,
    },
    /// Requested equalizer preset was outside the reported range.
    #[error("equalizer preset {value} is outside 0..{preset_count}")]
    InvalidEqPreset {
        /// Requested preset.
        value: u8,
        /// Number of presets reported by the device.
        preset_count: u8,
    },
}

/// Convenience result type for the S260 driver.
pub type Result<T> = std::result::Result<T, Error>;

impl From<Error> for open_edifier_core::Error {
    fn from(error: Error) -> Self {
        use open_edifier_core::{Error as CoreError, Source};

        match error {
            Error::Io(error) => CoreError::Network(error.to_string()),
            Error::Json(error) => CoreError::Protocol(error.to_string()),
            Error::Protocol(message) => CoreError::Protocol(message),
            Error::Rejected(message) => CoreError::Rejected(message),
            Error::MissingField(field) => CoreError::Protocol(format!("missing field {field}")),
            Error::InvalidSource(index) => {
                CoreError::Protocol(format!("invalid source index {index}"))
            }
            Error::UnsupportedSource(source) => CoreError::UnsupportedSource(Source::new(source)),
            Error::InvalidVolume { value, min, max } => {
                CoreError::InvalidVolume { value, min, max }
            }
            Error::Verification { expected, actual } => {
                CoreError::Verification { expected, actual }
            }
            Error::InvalidEqPreset {
                value,
                preset_count,
            } => CoreError::InvalidEqPreset {
                value,
                preset_count,
            },
        }
    }
}
