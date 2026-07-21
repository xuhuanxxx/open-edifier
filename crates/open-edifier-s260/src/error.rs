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
    #[error("speaker rejected command with code {code}: {message}")]
    Rejected {
        /// Device result code.
        code: i64,
        /// Sanitized device message.
        message: String,
    },
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
    /// Device state did not reflect an acknowledged mutation before the deadline.
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
    /// Event connection could not be restored within the caller's wait budget.
    #[error("event stream reconnect failed: {0}")]
    Reconnect(String),
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
            Error::Io(error) => CoreError::Network {
                operation: "S260 socket operation",
                message: error.to_string(),
            },
            Error::Json(error) => CoreError::Protocol {
                message: error.to_string(),
            },
            Error::Protocol(message) => CoreError::Protocol { message },
            Error::Rejected { code, message } => CoreError::Rejected { code, message },
            Error::MissingField(field) => CoreError::Protocol {
                message: format!("missing field {field}"),
            },
            Error::InvalidSource(index) => CoreError::Protocol {
                message: format!("invalid source index {index}"),
            },
            Error::UnsupportedSource(source) => CoreError::UnsupportedSource(Source::new(source)),
            Error::InvalidVolume { value, min, max } => {
                CoreError::InvalidVolume { value, min, max }
            }
            Error::VerificationTimeout {
                field,
                expected,
                actual,
                attempts,
                elapsed_ms,
            } => CoreError::VerificationTimeout {
                field,
                expected,
                actual,
                attempts,
                elapsed_ms,
            },
            Error::Reconnect(message) => CoreError::Network {
                operation: "event stream reconnect",
                message,
            },
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
