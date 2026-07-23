import json
import unittest

from open_edifier import ProtocolError, UnknownEvent, VolumeEvent
from open_edifier.protocol import (
    BINARY_FRAME_HEADER,
    JSON_FRAME_HEADER,
    BinaryFrameDecoder,
    JsonFrameDecoder,
    decode_event,
    parse_status,
)


def json_frame(value: object) -> bytes:
    payload = json.dumps(value, separators=(",", ":")).encode()
    return JSON_FRAME_HEADER + len(payload).to_bytes(2, "big") + payload


def binary_frame(command: int, payload: bytes) -> bytes:
    frame = (
        BINARY_FRAME_HEADER
        + command.to_bytes(2, "little")
        + bytes([len(payload)])
        + payload
    )
    return frame + bytes([sum(frame) & 0xFF])


class ProtocolTests(unittest.TestCase):
    def test_json_decoder_recovers_after_noise_and_malformed_frame(self) -> None:
        expected = {"id": "2", "code": 0, "message": "success"}
        wire = b"noise" + JSON_FRAME_HEADER + b"\x00\x01{" + json_frame(expected)
        decoder = JsonFrameDecoder()

        self.assertEqual(decoder.feed(wire[:8]), [])
        decoded = decoder.feed(wire[8:])

        self.assertIsInstance(decoded[0], ProtocolError)
        self.assertEqual(decoded[1], expected)

    def test_binary_decoder_recovers_after_bad_checksum(self) -> None:
        damaged = bytearray(binary_frame(0x0066, bytes([30, 17])))
        damaged[-1] ^= 0xFF
        expected = binary_frame(0x0066, bytes([30, 18]))

        frames = BinaryFrameDecoder().feed(bytes(damaged) + expected)

        self.assertEqual(frames, [(0x0066, bytes([30, 18]))])
        self.assertEqual(decode_event(*frames[0]), VolumeEvent(18, 30))

    def test_status_projection_validates_ranges_and_drops_private_fields(self) -> None:
        raw = status("request", volume=18)
        raw["deviceInfo"]["wifiName"] = "private-network"
        raw["bluetoothPairingRecord"] = [{"name": "private-device"}]

        parsed = parse_status(raw).public

        self.assertEqual(parsed.volume.current, 18)
        self.assertNotIn("private-network", repr(parsed))
        self.assertNotIn("private-device", repr(parsed))
        raw["player"]["volume"] = 31
        with self.assertRaises(ProtocolError):
            parse_status(raw)

    def test_unknown_valid_event_remains_observable(self) -> None:
        self.assertEqual(
            decode_event(0x9999, b"\x01\x02"), UnknownEvent(0x9999, b"\x01\x02")
        )


def status(request_id: str, *, volume: int) -> dict[str, object]:
    return {
        "code": 0,
        "id": request_id,
        "payload": "status_query",
        "message": "success",
        "deviceInfo": {
            "bluetoothName": "EDIFIER S260",
            "firmwareVersion": "01.00.00",
        },
        "inputSource": {"inputIndex": 1, "selectedIndex": 3},
        "player": {
            "volume": volume,
            "minVolume": 0,
            "maxVolume": 30,
            "playerStatus": 0,
        },
        "soundEffect": {"selectedIndex": 0, "soundIndex": 3},
    }


if __name__ == "__main__":
    unittest.main()
