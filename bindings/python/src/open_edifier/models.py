"""Privacy-safe public state and event models."""

from dataclasses import dataclass
from typing import TypeAlias


@dataclass(frozen=True, slots=True)
class Volume:
    """Current volume and its device-reported range."""

    current: int
    minimum: int
    maximum: int


@dataclass(frozen=True, slots=True)
class Equalizer:
    """Current equalizer preset and the number of presets."""

    preset: int
    preset_count: int


@dataclass(frozen=True, slots=True)
class DeviceCapabilities:
    """Capabilities exposed by the verified S260 driver."""

    sources: tuple[str, ...]
    volume: bool
    equalizer: bool
    playback: bool
    events: bool


@dataclass(frozen=True, slots=True)
class DeviceStatus:
    """Stable public state without private vendor response fields."""

    name: str
    model: str
    firmware: str
    source: str
    volume: Volume
    equalizer: Equalizer | None
    playback: str | None
    capabilities: DeviceCapabilities


@dataclass(frozen=True, slots=True)
class SourceEvent:
    """The active input source changed."""

    source: str


@dataclass(frozen=True, slots=True)
class VolumeEvent:
    """The speaker reported a volume change."""

    current: int
    maximum: int


@dataclass(frozen=True, slots=True)
class PlaybackEvent:
    """The speaker reported a playback-state change."""

    state: str


@dataclass(frozen=True, slots=True)
class EqualizerEvent:
    """The speaker reported an equalizer change."""

    preset: int


@dataclass(frozen=True, slots=True)
class TrackInfoEvent:
    """Opaque, verified track-information event payload."""

    payload: bytes


@dataclass(frozen=True, slots=True)
class UnknownEvent:
    """A checksum-valid event whose command semantics are unknown."""

    command: int
    payload: bytes


DeviceEvent: TypeAlias = (
    SourceEvent
    | VolumeEvent
    | PlaybackEvent
    | EqualizerEvent
    | TrackInfoEvent
    | UnknownEvent
)
