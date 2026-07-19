//! Local network discovery for EDIFIER speakers.
#![warn(missing_docs)]

use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent};
use open_edifier_core::{DiscoveredDevice, Error, MODEL_S260, ModelId, Result};

/// DNS-SD service used to identify the verified S260.
pub const AIRPLAY_SERVICE: &str = "_airplay._tcp.local.";
const S260_AIRPLAY_MODEL: &str = "EDF100122";

/// Discovers EDIFIER candidates for the full requested duration.
pub fn discover(timeout: Duration) -> Result<Vec<DiscoveredDevice>> {
    let daemon = ServiceDaemon::new().map_err(discovery_error)?;
    let receiver = daemon.browse(AIRPLAY_SERVICE).map_err(discovery_error)?;
    let deadline = Instant::now() + timeout;
    let mut devices = Vec::new();
    let mut seen = HashSet::new();

    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(service)) => {
                if let Some(device) = classify(&service) {
                    if seen.insert(device.id.clone()) {
                        devices.push(device);
                    }
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    let _ = daemon.stop_browse(AIRPLAY_SERVICE);
    let _ = daemon.shutdown();
    devices.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(devices)
}

fn classify(service: &ResolvedService) -> Option<DiscoveredDevice> {
    let manufacturer = service
        .get_property_val_str("manufacturer")
        .unwrap_or_default();
    let fullname = service.get_fullname();
    if !manufacturer.to_ascii_lowercase().contains("edifier")
        && !fullname.to_ascii_lowercase().contains("edifier")
    {
        return None;
    }

    let name = fullname
        .strip_suffix(AIRPLAY_SERVICE)
        .unwrap_or(fullname)
        .trim_end_matches('.')
        .to_owned();
    let advertised_model = service.get_property_val_str("model").unwrap_or("unknown");
    let model = classify_model(&name, advertised_model);
    let id = service
        .get_property_val_str("deviceid")
        .filter(|value| !value.is_empty())
        .unwrap_or(service.get_hostname())
        .to_owned();
    let mut addresses: Vec<_> = service
        .get_addresses()
        .iter()
        .map(|address| address.to_ip_addr())
        .collect();
    addresses.sort();

    Some(DiscoveredDevice {
        id,
        name,
        model,
        host: service.get_hostname().trim_end_matches('.').to_owned(),
        addresses,
    })
}

fn discovery_error(error: impl std::fmt::Display) -> Error {
    Error::Discovery(error.to_string())
}

fn classify_model(name: &str, advertised_model: &str) -> ModelId {
    if advertised_model.eq_ignore_ascii_case(S260_AIRPLAY_MODEL)
        || name.to_ascii_lowercase().contains("s260")
    {
        ModelId::new(MODEL_S260)
    } else {
        ModelId::new(advertised_model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_s260_after_user_renames_airplay_service() {
        assert_eq!(
            classify_model("Living Room", S260_AIRPLAY_MODEL).as_str(),
            MODEL_S260
        );
    }

    #[test]
    fn preserves_unknown_advertised_model() {
        assert_eq!(
            classify_model("EDIFIER Studio", "EDF999999").as_str(),
            "edf999999"
        );
    }
}
