use std::time::Duration;

use clap::{Parser, Subcommand};
use open_edifier::{
    Device, DeviceEvent, DeviceEvents, DeviceStatus, DiscoveredDevice, Error, ModelId,
    PlaybackAction, Result, Source, connect, connect_events, connect_events_host, connect_host,
    discover, supports_model,
};

#[derive(Debug, Parser)]
#[command(
    name = "edifier",
    about = "Discover and control EDIFIER speakers locally",
    version
)]
struct Args {
    /// Connect directly instead of using mDNS discovery.
    #[arg(
        long,
        env = "EDIFIER_HOST",
        requires = "model",
        conflicts_with = "device"
    )]
    host: Option<String>,
    /// Driver model to use with --host, for example s260.
    #[arg(long, env = "EDIFIER_MODEL", requires = "host")]
    model: Option<String>,
    /// Override the selected model driver's default port.
    #[arg(long, requires = "host")]
    port: Option<u16>,
    /// Select a discovered speaker by ID, name, or hostname.
    #[arg(long, env = "EDIFIER_DEVICE", conflicts_with = "host")]
    device: Option<String>,
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u64).range(1..))]
    timeout: u64,
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u64).range(1..))]
    discovery_timeout: u64,
    /// Emit machine-readable JSON.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Find recognizable EDIFIER speakers on the local network.
    Discover,
    /// Show the selected speaker's current state.
    Status,
    /// Select an input source such as aux, usb, bluetooth, or airplay.
    Source { name: String },
    /// Set volume within the range reported by the speaker.
    Volume { level: u8 },
    /// Read or select an equalizer preset.
    Eq { preset: Option<u8> },
    /// Start or resume playback on a media-capable input.
    Play,
    /// Pause playback on a media-capable input.
    Pause,
    /// Skip to the next track on a media-capable input.
    Next,
    /// Return to the previous track on a media-capable input.
    Prev,
    /// Stream state changes from the model's push channel.
    Listen {
        /// Exit after this many events; omit to continue until interrupted.
        #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
        count: Option<u64>,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("edifier: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    if matches!(&args.command, Command::Discover) {
        return print_devices(
            &discover(Duration::from_secs(args.discovery_timeout))?,
            args.json,
        );
    }

    let timeout = Duration::from_secs(args.timeout);
    if let Command::Listen { count } = &args.command {
        return listen(resolve_events(&args, timeout)?, *count, args.json);
    }

    let mut device = resolve_device(&args, timeout)?;
    let status = match &args.command {
        Command::Status | Command::Eq { preset: None } => Some(device.status()?),
        Command::Source { name } => Some(device.set_source(Source::new(name))?),
        Command::Volume { level } => Some(device.set_volume(*level)?),
        Command::Eq {
            preset: Some(preset),
        } => Some(device.set_eq_preset(*preset)?),
        Command::Play => {
            device.playback(PlaybackAction::Play)?;
            print_action("play", args.json)?;
            None
        }
        Command::Pause => {
            device.playback(PlaybackAction::Pause)?;
            print_action("pause", args.json)?;
            None
        }
        Command::Next => {
            device.playback(PlaybackAction::Next)?;
            print_action("next", args.json)?;
            None
        }
        Command::Prev => {
            device.playback(PlaybackAction::Previous)?;
            print_action("previous", args.json)?;
            None
        }
        Command::Discover | Command::Listen { .. } => unreachable!(),
    };
    if let Some(status) = status {
        print_status(&status, args.json)?;
    }
    Ok(())
}

fn resolve_device(args: &Args, timeout: Duration) -> Result<Box<dyn Device>> {
    if let Some(host) = &args.host {
        return connect_host(&explicit_model(args)?, host, args.port, timeout);
    }
    connect(&discover_selected(args)?, timeout)
}

fn resolve_events(args: &Args, timeout: Duration) -> Result<Box<dyn DeviceEvents>> {
    if let Some(host) = &args.host {
        return connect_events_host(&explicit_model(args)?, host, args.port, timeout);
    }
    connect_events(&discover_selected(args)?, timeout)
}

fn explicit_model(args: &Args) -> Result<ModelId> {
    Ok(ModelId::new(args.model.as_deref().ok_or_else(|| {
        Error::Protocol("--model is required when --host is used".into())
    })?))
}

fn discover_selected(args: &Args) -> Result<DiscoveredDevice> {
    let devices = discover(Duration::from_secs(args.discovery_timeout))?;
    select_device(&devices, args.device.as_deref()).cloned()
}

fn select_device<'a>(
    devices: &'a [DiscoveredDevice],
    selector: Option<&str>,
) -> Result<&'a DiscoveredDevice> {
    if let Some(selector) = selector {
        return devices
            .iter()
            .find(|device| {
                [&device.id, &device.name, &device.host]
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(selector))
            })
            .ok_or(Error::DeviceNotFound);
    }
    let supported: Vec<_> = devices
        .iter()
        .filter(|device| supports_model(&device.model))
        .collect();
    match supported.as_slice() {
        [] => Err(Error::DeviceNotFound),
        [device] => Ok(device),
        candidates => Err(Error::AmbiguousDevice {
            candidates: candidates
                .iter()
                .map(|device| format!("{} ({})", device.name, device.id))
                .collect(),
        }),
    }
}

fn print_devices(devices: &[DiscoveredDevice], json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(devices)
                .map_err(|error| Error::Serialization(error.to_string()))?
        );
        return Ok(());
    }
    if devices.is_empty() {
        println!("No EDIFIER speakers found.");
    }
    for device in devices {
        let addresses = device
            .addresses
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{} | model={} | host={} | address={} | id={}",
            device.name, device.model, device.host, addresses, device.id
        );
    }
    Ok(())
}

fn print_status(status: &DeviceStatus, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(status)
                .map_err(|error| Error::Serialization(error.to_string()))?
        );
    } else {
        let source = status
            .source
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "n/a".to_owned());
        let volume = status
            .volume
            .as_ref()
            .map(|volume| format!("{}/{}", volume.current, volume.max))
            .unwrap_or_else(|| "n/a".to_owned());
        let equalizer = status
            .equalizer
            .as_ref()
            .map(|eq| format!("{}/{}", eq.preset, eq.preset_count))
            .unwrap_or_else(|| "n/a".to_owned());
        let playback = status
            .playback
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "unknown".to_owned());
        println!(
            "{} | model={} | source={} | volume={} | eq={} | playback={} | firmware={}",
            status.name, status.model, source, volume, equalizer, playback, status.firmware
        );
    }
    Ok(())
}

fn print_action(action: &str, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"action": action, "accepted": true}))
                .map_err(|error| Error::Serialization(error.to_string()))?
        );
    } else {
        println!("{action} accepted");
    }
    Ok(())
}

fn listen(mut events: Box<dyn DeviceEvents>, count: Option<u64>, json: bool) -> Result<()> {
    if !json {
        eprintln!("listening for speaker events; press Ctrl-C to stop...");
    }
    let mut emitted = 0_u64;
    loop {
        let Some(event) = events.next_event()? else {
            continue;
        };
        if json {
            println!(
                "{}",
                serde_json::to_string(&event)
                    .map_err(|error| Error::Serialization(error.to_string()))?
            );
        } else {
            println!("{}", event_line(&event));
        }
        emitted += 1;
        if count.is_some_and(|limit| emitted >= limit) {
            return Ok(());
        }
    }
}

fn event_line(event: &DeviceEvent) -> String {
    match event {
        DeviceEvent::Source { source } => format!("source={source}"),
        DeviceEvent::Volume { current, max } => format!("volume={current}/{max}"),
        DeviceEvent::Playback { state } => format!("playback={state}"),
        DeviceEvent::Equalizer { preset } => format!("eq={preset}"),
        DeviceEvent::TrackInfo { payload } => format!("track_info={}", hex(payload)),
        DeviceEvent::Unknown { command, payload } => {
            format!("event=0x{command:04x} payload={}", hex(payload))
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use open_edifier::{DeviceEvent, DiscoveredDevice, ModelId, Source};

    use super::{event_line, select_device};

    fn device(id: &str, name: &str, model: &str) -> DiscoveredDevice {
        DiscoveredDevice {
            id: id.to_owned(),
            name: name.to_owned(),
            model: ModelId::new(model),
            host: format!("{id}.local"),
            addresses: vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
        }
    }

    #[test]
    fn selection_requires_a_selector_when_multiple_devices_exist() {
        let devices = [
            device("one", "Office", "s260"),
            device("two", "Studio", "s260"),
        ];
        assert!(select_device(&devices, None).is_err());
        assert_eq!(select_device(&devices, Some("Studio")).unwrap().id, "two");
    }

    #[test]
    fn automatic_selection_ignores_devices_without_a_driver() {
        let devices = [
            device("one", "Office", "s260"),
            device("two", "Unknown", "edf999999"),
        ];
        assert_eq!(select_device(&devices, None).unwrap().id, "one");
    }

    #[test]
    fn formats_typed_events_for_humans() {
        assert_eq!(
            event_line(&DeviceEvent::Source {
                source: Source::new(Source::USB),
            }),
            "source=usb"
        );
    }
}
