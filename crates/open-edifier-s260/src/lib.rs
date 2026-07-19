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
pub use model::SpeakerStatus;
pub use open_edifier_core::Source;
pub use protocol::{FRAME_HEADER, FrameDecoder};
