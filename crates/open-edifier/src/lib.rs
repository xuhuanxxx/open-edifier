//! High-level OpenEdifier SDK with discovery and driver selection.
#![warn(missing_docs)]

use std::time::Duration;

pub use open_edifier_core::{
    Device, DeviceCapabilities, DeviceEvent, DeviceEvents, DeviceStatus, DiscoveredDevice,
    Equalizer, Error, ModelId, PlaybackAction, PlaybackState, Result, Source, Volume,
};
pub use open_edifier_discovery::discover;

type ConnectDevice = fn(String, u16, Duration) -> Result<Box<dyn Device>>;
type ConnectEvents = fn(String, u16, Duration) -> Result<Box<dyn DeviceEvents>>;

struct DriverRegistration {
    model: &'static str,
    default_port: u16,
    connect: ConnectDevice,
    connect_events: Option<ConnectEvents>,
}

static DRIVERS: &[DriverRegistration] = &[DriverRegistration {
    model: open_edifier_s260::MODEL_ID,
    default_port: open_edifier_s260::DEFAULT_PORT,
    connect: connect_s260,
    connect_events: Some(connect_s260_events),
}];

/// Returns whether this build contains a driver for the model.
pub fn supports_model(model: &ModelId) -> bool {
    driver(model).is_some()
}

/// Connects to a discovered speaker using its registered model driver.
pub fn connect(device: &DiscoveredDevice, timeout: Duration) -> Result<Box<dyn Device>> {
    let driver = required_driver(&device.model)?;
    (driver.connect)(preferred_host(device), driver.default_port, timeout)
}

/// Opens a model's state-event channel for a discovered speaker.
pub fn connect_events(
    device: &DiscoveredDevice,
    timeout: Duration,
) -> Result<Box<dyn DeviceEvents>> {
    let driver = required_driver(&device.model)?;
    let connect = driver
        .connect_events
        .ok_or(Error::UnsupportedCapability("events"))?;
    connect(preferred_host(device), driver.default_port, timeout)
}

/// Connects directly to a host using an explicitly selected model driver.
pub fn connect_host(
    model: &ModelId,
    host: impl Into<String>,
    port: Option<u16>,
    timeout: Duration,
) -> Result<Box<dyn Device>> {
    let driver = required_driver(model)?;
    (driver.connect)(host.into(), port.unwrap_or(driver.default_port), timeout)
}

/// Opens a model's state-event channel using an explicit host and model.
pub fn connect_events_host(
    model: &ModelId,
    host: impl Into<String>,
    port: Option<u16>,
    timeout: Duration,
) -> Result<Box<dyn DeviceEvents>> {
    let driver = required_driver(model)?;
    let connect = driver
        .connect_events
        .ok_or(Error::UnsupportedCapability("events"))?;
    connect(host.into(), port.unwrap_or(driver.default_port), timeout)
}

fn driver(model: &ModelId) -> Option<&'static DriverRegistration> {
    driver_in(DRIVERS, model)
}

fn driver_in<'a>(
    drivers: &'a [DriverRegistration],
    model: &ModelId,
) -> Option<&'a DriverRegistration> {
    drivers.iter().find(|driver| driver.model == model.as_str())
}

fn required_driver(model: &ModelId) -> Result<&'static DriverRegistration> {
    driver(model).ok_or_else(|| Error::UnsupportedModel(model.clone()))
}

fn connect_s260(host: String, port: u16, timeout: Duration) -> Result<Box<dyn Device>> {
    Ok(Box::new(
        open_edifier_s260::Client::connect(open_edifier_s260::ClientConfig {
            host,
            port,
            timeout,
        })
        .map_err(open_edifier_core::Error::from)?,
    ))
}

fn connect_s260_events(
    host: String,
    port: u16,
    timeout: Duration,
) -> Result<Box<dyn DeviceEvents>> {
    Ok(Box::new(
        open_edifier_s260::EventStream::connect(open_edifier_s260::ClientConfig {
            host,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_distinguishes_installed_and_unknown_models() {
        assert!(supports_model(&ModelId::new(open_edifier_s260::MODEL_ID)));
        assert!(!supports_model(&ModelId::new("unknown")));
        assert_eq!(
            required_driver(&ModelId::new(open_edifier_s260::MODEL_ID))
                .unwrap()
                .default_port,
            open_edifier_s260::DEFAULT_PORT
        );
    }

    #[test]
    fn registry_can_select_a_second_model_without_changing_dispatch() {
        let drivers = [
            DriverRegistration {
                model: "first",
                default_port: 1001,
                connect: connect_s260,
                connect_events: None,
            },
            DriverRegistration {
                model: "second",
                default_port: 1002,
                connect: connect_s260,
                connect_events: Some(connect_s260_events),
            },
        ];

        let selected = driver_in(&drivers, &ModelId::new("second")).unwrap();
        assert_eq!(selected.default_port, 1002);
        assert!(selected.connect_events.is_some());
    }
}
