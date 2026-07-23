"""S260 framing and privacy-safe protocol projection."""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any

from .errors import ProtocolError, UnsupportedSourceError
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

JSON_FRAME_HEADER = b"\xee\xdd\xff\xee"
BINARY_FRAME_HEADER = b"\xbb\xec"
HEARTBEAT_COMMAND = 0x003F

_INPUT_EVENT = 0x0061
_VOLUME_EVENT = 0x0066
_PLAYBACK_EVENT = 0x0068
_TRACK_INFO_EVENT = 0x0050
_EQ_EVENTS = {0x00D5, 0x00C4}
_INPUT_SUBCOMMAND = 0x1E
_SOURCES = ("bluetooth", "aux", "usb", "airplay")
_CAPABILITIES = DeviceCapabilities(_SOURCES, True, True, True, True)


class JsonFrameDecoder:
    """Incrementally decodes framed JSON responses and skips leading noise."""

    def __init__(self) -> None:
        self._buffer = bytearray()

    def feed(self, chunk: bytes) -> list[Any | ProtocolError]:
        self._buffer.extend(chunk)
        frames: list[Any | ProtocolError] = []
        while True:
            marker = self._buffer.find(JSON_FRAME_HEADER)
            if marker < 0:
                keep = min(len(self._buffer), len(JSON_FRAME_HEADER) - 1)
                del self._buffer[: len(self._buffer) - keep]
                return frames
            if marker:
                del self._buffer[:marker]
            if len(self._buffer) < 6:
                return frames
            payload_length = int.from_bytes(self._buffer[4:6], "big")
            frame_end = 6 + payload_length
            if len(self._buffer) < frame_end:
                return frames
            payload = bytes(self._buffer[6:frame_end])
            del self._buffer[:frame_end]
            try:
                frames.append(json.loads(payload))
            except (UnicodeDecodeError, json.JSONDecodeError) as error:
                frames.append(ProtocolError(f"invalid JSON response: {error}"))


class BinaryFrameDecoder:
    """Incrementally decodes checksum-valid BB EC frames."""

    def __init__(self) -> None:
        self._buffer = bytearray()

    def feed(self, chunk: bytes) -> list[tuple[int, bytes]]:
        self._buffer.extend(chunk)
        frames: list[tuple[int, bytes]] = []
        while True:
            marker = self._buffer.find(BINARY_FRAME_HEADER)
            if marker < 0:
                keep = min(len(self._buffer), len(BINARY_FRAME_HEADER) - 1)
                del self._buffer[: len(self._buffer) - keep]
                return frames
            if marker:
                del self._buffer[:marker]
            if len(self._buffer) < 6:
                return frames
            payload_length = self._buffer[4]
            frame_end = 6 + payload_length
            if len(self._buffer) < frame_end:
                return frames
            expected = sum(self._buffer[: frame_end - 1]) & 0xFF
            if self._buffer[frame_end - 1] != expected:
                del self._buffer[0]
                continue
            command = int.from_bytes(self._buffer[2:4], "little")
            payload = bytes(self._buffer[5 : 5 + payload_length])
            del self._buffer[:frame_end]
            frames.append((command, payload))


@dataclass(frozen=True, slots=True)
class ParsedStatus:
    public: DeviceStatus
    input_index: int


def encode_request(value: dict[str, Any]) -> bytes:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode()


def parse_status(raw: Any) -> ParsedStatus:
    response = _object(raw, "response")
    device_info = _object(response.get("deviceInfo", {}), "deviceInfo")
    input_source = _object(response.get("inputSource"), "inputSource")
    player = _object(response.get("player"), "player")

    selected_index = _integer(input_source.get("selectedIndex"), "selectedIndex")
    try:
        source = _SOURCES[selected_index]
    except IndexError as error:
        raise ProtocolError(f"invalid input source index: {selected_index}") from error
    input_index = _integer(input_source.get("inputIndex", 1), "inputIndex")

    current = _byte(player.get("volume"), "volume")
    minimum = _byte(player.get("minVolume", 0), "minVolume")
    maximum = _byte(player.get("maxVolume"), "maxVolume")
    if minimum > maximum or not minimum <= current <= maximum:
        raise ProtocolError(
            f"invalid volume range: min={minimum}, current={current}, max={maximum}"
        )

    equalizer = _parse_equalizer(response.get("soundEffect"))
    playback_value = player.get("playerStatus")
    playback = (
        None
        if playback_value is None
        else _playback_state(_integer(playback_value, "playerStatus"))
    )

    name = _optional_string(device_info, "bluetoothName", "EDIFIER S260")
    firmware = _optional_string(device_info, "firmwareVersion", "unknown")
    return ParsedStatus(
        public=DeviceStatus(
            name=name,
            model="s260",
            firmware=firmware,
            source=source,
            volume=Volume(current, minimum, maximum),
            equalizer=equalizer,
            playback=playback,
            capabilities=_CAPABILITIES,
        ),
        input_index=input_index,
    )


def normalize_source(source: str) -> str:
    if not isinstance(source, str):
        raise TypeError("source must be a string")
    normalized = source.lower().replace("-", "").replace("_", "")
    aliases = {
        "bt": "bluetooth",
        "bluetooth": "bluetooth",
        "aux": "aux",
        "linein": "aux",
        "usb": "usb",
        "airplay": "airplay",
        "airplay2": "airplay",
    }
    try:
        return aliases[normalized]
    except KeyError as error:
        raise UnsupportedSourceError(source) from error


def source_index(source: str) -> int:
    return _SOURCES.index(normalize_source(source))


def decode_event(command: int, payload: bytes) -> DeviceEvent | None:
    if command == HEARTBEAT_COMMAND:
        return None
    if (
        command == _INPUT_EVENT
        and len(payload) >= 2
        and payload[0] == _INPUT_SUBCOMMAND
    ):
        source_index_value = payload[1]
        if 1 <= source_index_value <= len(_SOURCES):
            return SourceEvent(_SOURCES[source_index_value - 1])
    elif command == _VOLUME_EVENT and len(payload) >= 2:
        maximum, current = payload[:2]
        if current > maximum:
            raise ProtocolError(
                f"invalid volume event: current={current}, max={maximum}"
            )
        return VolumeEvent(current, maximum)
    elif command == _PLAYBACK_EVENT and payload and payload[0] <= 2:
        return PlaybackEvent(_playback_state(payload[0]))
    elif command in _EQ_EVENTS and payload:
        return EqualizerEvent(payload[0])
    elif command == _TRACK_INFO_EVENT:
        return TrackInfoEvent(payload)
    return UnknownEvent(command, payload)


def _parse_equalizer(raw: Any) -> Equalizer | None:
    if raw is None:
        return None
    value = _object(raw, "soundEffect")
    preset = value.get("selectedIndex")
    preset_count = value.get("soundIndex")
    if preset is None and preset_count is None:
        return None
    if preset is None or preset_count is None:
        raise ProtocolError("soundEffect must contain selectedIndex and soundIndex")
    checked_preset = _byte(preset, "EQ preset")
    checked_count = _byte(preset_count, "EQ preset count")
    if checked_count == 0 or checked_preset >= checked_count:
        raise ProtocolError(
            f"invalid EQ range: preset={checked_preset}, preset_count={checked_count}"
        )
    return Equalizer(checked_preset, checked_count)


def _playback_state(value: int) -> str:
    return {0: "stopped", 1: "playing", 2: "paused"}.get(value, f"unknown_{value}")


def _object(value: Any, field: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ProtocolError(f"{field} must be an object")
    return value


def _integer(value: Any, field: str) -> int:
    if type(value) is not int or value < 0:
        raise ProtocolError(f"{field} must be a non-negative integer")
    return value


def _byte(value: Any, field: str) -> int:
    checked = _integer(value, field)
    if checked > 255:
        raise ProtocolError(f"{field} does not fit in u8")
    return checked


def _optional_string(value: dict[str, Any], field: str, default: str) -> str:
    result = value.get(field)
    if result is None:
        return default
    if not isinstance(result, str):
        raise ProtocolError(f"{field} must be a string")
    return result
