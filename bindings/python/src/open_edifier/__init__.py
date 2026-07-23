"""Async Python client for supported EDIFIER speakers."""

from .client import S260Client, S260EventStream
from .errors import (
    InvalidEqualizerPresetError,
    InvalidVolumeError,
    NetworkError,
    NotConnectedError,
    OpenEdifierError,
    ProtocolError,
    RejectedError,
    RequestTimeoutError,
    UnsupportedSourceError,
    VerificationTimeoutError,
)
from .models import (
    DeviceCapabilities,
    DeviceEvent,
    DeviceStatus,
    Equalizer,
    EqualizerEvent,
    PlaybackEvent,
    SourceEvent,
    TrackInfoEvent,
    UnknownEvent,
    Volume,
    VolumeEvent,
)

__all__ = [
    "DeviceCapabilities",
    "DeviceEvent",
    "DeviceStatus",
    "Equalizer",
    "EqualizerEvent",
    "InvalidEqualizerPresetError",
    "InvalidVolumeError",
    "NetworkError",
    "NotConnectedError",
    "OpenEdifierError",
    "PlaybackEvent",
    "ProtocolError",
    "RejectedError",
    "RequestTimeoutError",
    "S260Client",
    "S260EventStream",
    "SourceEvent",
    "TrackInfoEvent",
    "UnknownEvent",
    "UnsupportedSourceError",
    "VerificationTimeoutError",
    "Volume",
    "VolumeEvent",
]
