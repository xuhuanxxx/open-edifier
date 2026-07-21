//! High-level OpenEdifier SDK with discovery and driver selection.
#![warn(missing_docs)]

use std::time::Duration;

pub use open_edifier_core::{
    Device, DeviceCapabilities, DeviceEvent, DeviceEvents, DeviceStatus, DiscoveredDevice,
    Equalizer, Error, ModelId, PlaybackAction, PlaybackState, Result, Source, Volume,
};

/// Discovers only devices supported by a driver in this build.
pub fn discover(timeout: Duration) -> Result<Vec<DiscoveredDevice>> {
    let mut devices = open_edifier_discovery::discover_candidates(timeout)?;
    devices.retain(|device| supports_model(&device.model));
    Ok(devices)
}

/// Returns whether this build contains a driver for the model.
pub fn supports_model(model: &ModelId) -> bool {
    model.as_str() == open_edifier_s260::MODEL_ID
}

/// Connects to a discovered speaker using its registered model driver.
pub fn connect(device: &DiscoveredDevice, timeout: Duration) -> Result<Box<dyn Device>> {
    connect_host(&device.model, preferred_host(device), None, timeout)
}

/// Opens a model's state-event channel for a discovered speaker.
pub fn connect_events(
    device: &DiscoveredDevice,
    timeout: Duration,
) -> Result<Box<dyn DeviceEvents>> {
    connect_events_host(&device.model, preferred_host(device), None, timeout)
}

/// Connects directly to a host using an explicitly selected model driver.
pub fn connect_host(
    model: &ModelId,
    host: impl Into<String>,
    port: Option<u16>,
    timeout: Duration,
) -> Result<Box<dyn Device>> {
    match model.as_str() {
        open_edifier_s260::MODEL_ID => connect_s260(
            host.into(),
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
        open_edifier_s260::MODEL_ID => connect_s260_events(
            host.into(),
            port.unwrap_or(open_edifier_s260::DEFAULT_PORT),
            timeout,
        ),
        _ => Err(Error::UnsupportedModel(model.clone())),
    }
}

fn connect_s260(host: String, port: u16, timeout: Duration) -> Result<Box<dyn Device>> {
    let mut config = open_edifier_s260::ClientConfig::new(host);
    config.port = port;
    config.connect_timeout = timeout;
    config.request_timeout = timeout;
    Ok(Box::new(
        open_edifier_s260::Client::connect(config).map_err(open_edifier_core::Error::from)?,
    ))
}

fn connect_s260_events(
    host: String,
    port: u16,
    timeout: Duration,
) -> Result<Box<dyn DeviceEvents>> {
    let mut config = open_edifier_s260::ClientConfig::new(host);
    config.port = port;
    config.connect_timeout = timeout;
    config.request_timeout = timeout;
    Ok(Box::new(
        open_edifier_s260::EventStream::connect(config).map_err(open_edifier_core::Error::from)?,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_dispatch_distinguishes_supported_and_unknown_models() {
        assert!(supports_model(&ModelId::new(open_edifier_s260::MODEL_ID)));
        assert!(!supports_model(&ModelId::new("unknown")));
    }
}
