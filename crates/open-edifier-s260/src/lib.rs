//! EDIFIER S260 local network driver for OpenEdifier.
#![warn(missing_docs)]

mod client;
mod error;
mod events;
mod model;
mod protocol;

pub use client::{Client, ClientConfig, DEFAULT_PORT};
pub use error::{Error, Result};
pub use events::EventStream;
pub use open_edifier_core::Source;

/// Canonical driver identifier for the EDIFIER S260.
pub const MODEL_ID: &str = "s260";
