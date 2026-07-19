//! High-level OpenEdifier SDK with discovery and driver selection.
#![warn(missing_docs)]

use std::time::Duration;

pub use open_edifier_core::{
    Device, DeviceEvent, DeviceEvents, DeviceStatus, DiscoveredDevice, Equalizer, Error, ModelId,
    PlaybackAction, PlaybackState, Result, Source, Volume,
};
pub use open_edifier_discovery::discover;

/// Returns whether this build contains a driver for the model.
pub fn supports_model(model: &ModelId) -> bool {
    model.as_str() == open_edifier_core::MODEL_S260
}

/// Connects to a discovered speaker using its registered model driver.
pub fn connect(device: &DiscoveredDevice, timeout: Duration) -> Result<Box<dyn Device>> {
    match device.model.as_str() {
        open_edifier_core::MODEL_S260 => connect_s260(
            preferred_host(device),
            open_edifier_s260::DEFAULT_PORT,
            timeout,
        ),
        _ => Err(Error::UnsupportedModel(device.model.clone())),
    }
}

/// Opens a model's state-event channel for a discovered speaker.
pub fn connect_events(
    device: &DiscoveredDevice,
    timeout: Duration,
) -> Result<Box<dyn DeviceEvents>> {
    match device.model.as_str() {
        open_edifier_core::MODEL_S260 => connect_s260_events(
            preferred_host(device),
            open_edifier_s260::DEFAULT_PORT,
            timeout,
        ),
        _ => Err(Error::UnsupportedModel(device.model.clone())),
    }
}

/// Connects directly to a host using an explicitly selected model driver.
pub fn connect_host(
    model: &ModelId,
    host: impl Into<String>,
    port: Option<u16>,
    timeout: Duration,
) -> Result<Box<dyn Device>> {
    match model.as_str() {
        open_edifier_core::MODEL_S260 => connect_s260(
            host,
            port.unwrap_or(open_edifier_s260::DEFAULT_PORT),
            timeout,
        ),
        _ => Err(Error::UnsupportedModel(model.clone())),
    }
}

/// Opens a model's state-event channel using an explicit host and model.
pub fn connect_events_host(
    model: &ModelId,
    host: impl Into<String>,
    port: Option<u16>,
    timeout: Duration,
) -> Result<Box<dyn DeviceEvents>> {
    match model.as_str() {
        open_edifier_core::MODEL_S260 => connect_s260_events(
            host,
            port.unwrap_or(open_edifier_s260::DEFAULT_PORT),
            timeout,
        ),
        _ => Err(Error::UnsupportedModel(model.clone())),
    }
}

fn connect_s260(host: impl Into<String>, port: u16, timeout: Duration) -> Result<Box<dyn Device>> {
    Ok(Box::new(
        open_edifier_s260::Client::connect(open_edifier_s260::ClientConfig {
            host: host.into(),
            port,
            timeout,
        })
        .map_err(open_edifier_core::Error::from)?,
    ))
}

fn connect_s260_events(
    host: impl Into<String>,
    port: u16,
    timeout: Duration,
) -> Result<Box<dyn DeviceEvents>> {
    Ok(Box::new(
        open_edifier_s260::EventStream::connect(open_edifier_s260::ClientConfig {
            host: host.into(),
            port,
            timeout,
        })
        .map_err(open_edifier_core::Error::from)?,
    ))
}

fn preferred_host(device: &DiscoveredDevice) -> String {
    device
        .addresses
        .iter()
        .find(|address| address.is_ipv4())
        .map(ToString::to_string)
        .unwrap_or_else(|| device.host.clone())
}
