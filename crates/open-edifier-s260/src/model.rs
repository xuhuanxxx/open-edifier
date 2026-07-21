use open_edifier_core::{
    DeviceCapabilities, DeviceStatus, Equalizer, ModelId, PlaybackState, Source, Volume,
};
use serde::Deserialize;
use serde_json::Value;

use crate::{Error, MODEL_ID, Result};

pub(crate) fn source_from_index(value: u64) -> Result<Source> {
    match value {
        0 => Ok(Source::new(Source::BLUETOOTH)),
        1 => Ok(Source::new(Source::AUX)),
        2 => Ok(Source::new(Source::USB)),
        3 => Ok(Source::new(Source::AIRPLAY)),
        other => Err(Error::InvalidSource(other)),
    }
}

pub(crate) fn source_index(source: &Source) -> Result<u8> {
    match source.as_str() {
        Source::BLUETOOTH => Ok(0),
        Source::AUX => Ok(1),
        Source::USB => Ok(2),
        Source::AIRPLAY => Ok(3),
        other => Err(Error::UnsupportedSource(other.to_owned())),
    }
}

pub(crate) fn playback_state(state: u64) -> PlaybackState {
    PlaybackState::new(match state {
        0 => PlaybackState::STOPPED.to_owned(),
        1 => PlaybackState::PLAYING.to_owned(),
        2 => PlaybackState::PAUSED.to_owned(),
        other => format!("unknown_{other}"),
    })
}

/// Parsed, privacy-safe S260 state.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct S260Status {
    /// Current Bluetooth/device name.
    pub name: String,
    /// Device firmware version.
    pub firmware: String,
    /// Selected input source.
    pub source: Source,
    /// Vendor input group index required for source mutations.
    pub input_index: u64,
    /// Current volume.
    pub volume: u8,
    /// Device-reported minimum volume.
    pub min_volume: u8,
    /// Device-reported maximum volume.
    pub max_volume: u8,
    /// Equalizer state when reported by the device.
    pub equalizer: Option<Equalizer>,
    /// Playback state when reported by the current source.
    pub playback: Option<PlaybackState>,
}

impl S260Status {
    pub(crate) fn from_value(raw: Value) -> Result<Self> {
        let wire: StatusResponse = serde_json::from_value(raw)?;
        let source = source_from_index(wire.input_source.selected_index)?;
        let volume = checked_u8(wire.player.volume, "volume")?;
        let min_volume = checked_u8(wire.player.min_volume.unwrap_or(0), "minVolume")?;
        let max_volume = checked_u8(wire.player.max_volume, "maxVolume")?;
        if min_volume > max_volume || !(min_volume..=max_volume).contains(&volume) {
            return Err(Error::Protocol(format!(
                "invalid volume range: min={min_volume}, current={volume}, max={max_volume}"
            )));
        }
        let equalizer = match wire.sound_effect {
            Some(SoundEffect {
                selected_index: Some(preset),
                sound_index: Some(preset_count),
            }) => {
                let preset = checked_u8(preset, "EQ preset")?;
                let preset_count = checked_u8(preset_count, "EQ preset count")?;
                if preset_count == 0 || preset >= preset_count {
                    return Err(Error::Protocol(format!(
                        "invalid EQ range: preset={preset}, preset_count={preset_count}"
                    )));
                }
                Some(Equalizer {
                    preset,
                    preset_count,
                })
            }
            Some(SoundEffect {
                selected_index: None,
                sound_index: None,
            })
            | None => None,
            Some(_) => return Err(Error::MissingField("complete soundEffect range")),
        };
        let playback = wire.player.player_status.map(playback_state);

        Ok(Self {
            name: wire
                .device_info
                .bluetooth_name
                .unwrap_or_else(|| "EDIFIER S260".to_owned()),
            firmware: wire
                .device_info
                .firmware_version
                .unwrap_or_else(|| "unknown".to_owned()),
            source,
            input_index: wire.input_source.input_index,
            volume,
            min_volume,
            max_volume,
            equalizer,
            playback,
        })
    }
}

fn checked_u8(value: u64, field: &str) -> Result<u8> {
    u8::try_from(value).map_err(|_| Error::Protocol(format!("{field} does not fit in u8")))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatusResponse {
    #[serde(default)]
    device_info: DeviceInfo,
    input_source: InputSource,
    player: Player,
    #[serde(default)]
    sound_effect: Option<SoundEffect>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceInfo {
    bluetooth_name: Option<String>,
    firmware_version: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InputSource {
    #[serde(default = "default_input_index")]
    input_index: u64,
    selected_index: u64,
}

fn default_input_index() -> u64 {
    1
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Player {
    volume: u64,
    min_volume: Option<u64>,
    max_volume: u64,
    player_status: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SoundEffect {
    selected_index: Option<u64>,
    sound_index: Option<u64>,
}

impl From<S260Status> for DeviceStatus {
    fn from(status: S260Status) -> Self {
        Self {
            name: status.name,
            model: ModelId::new(MODEL_ID),
            firmware: status.firmware,
            source: Some(status.source),
            volume: Some(Volume {
                current: status.volume,
                min: status.min_volume,
                max: status.max_volume,
            }),
            equalizer: status.equalizer,
            playback: status.playback,
            capabilities: DeviceCapabilities {
                sources: [Source::BLUETOOTH, Source::AUX, Source::USB, Source::AIRPLAY]
                    .into_iter()
                    .map(Source::new)
                    .collect(),
                volume: true,
                equalizer: true,
                playback: true,
                events: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn rejects_volume_ranges_that_do_not_fit_the_public_type() {
        let value = json!({
            "inputSource": {"selectedIndex": 2},
            "player": {"volume": 18, "minVolume": 256, "maxVolume": 30}
        });
        assert!(matches!(
            S260Status::from_value(value),
            Err(Error::Protocol(message)) if message.contains("minVolume")
        ));
    }

    #[test]
    fn public_status_does_not_serialize_private_vendor_fields() {
        let value = json!({
            "supportedFeatures": ["deviceInfo", "player"],
            "deviceInfo": {
                "bluetoothName": "EDIFIER S260",
                "firmwareVersion": "01.00.00",
                "wifiName": "private-network",
                "wifiMac": "private-value"
            },
            "bluetoothPairingRecord": [{"name": "private-device"}],
            "inputSource": {"inputIndex": 1, "selectedIndex": 2},
            "player": {"volume": 18, "minVolume": 0, "maxVolume": 30}
        });
        let public = DeviceStatus::from(S260Status::from_value(value).unwrap());
        let serialized = serde_json::to_string(&public).unwrap();
        assert!(!serialized.contains("private-network"));
        assert!(!serialized.contains("private-device"));
        assert!(!serialized.contains("wifiMac"));
        assert!(!serialized.contains("supportedFeatures"));
        assert!(!serialized.contains("deviceInfo"));
        assert!(serialized.contains(r#""sources":["bluetooth","aux","usb","airplay"]"#));
    }

    #[test]
    fn rejects_inconsistent_public_state_ranges() {
        let volume = json!({
            "inputSource": {"selectedIndex": 2},
            "player": {"volume": 31, "minVolume": 0, "maxVolume": 30}
        });
        assert!(matches!(
            S260Status::from_value(volume),
            Err(Error::Protocol(message)) if message.contains("volume range")
        ));

        let equalizer = json!({
            "inputSource": {"selectedIndex": 2},
            "player": {"volume": 18, "minVolume": 0, "maxVolume": 30},
            "soundEffect": {"selectedIndex": 3, "soundIndex": 3}
        });
        assert!(matches!(
            S260Status::from_value(equalizer),
            Err(Error::Protocol(message)) if message.contains("EQ range")
        ));
    }
}
