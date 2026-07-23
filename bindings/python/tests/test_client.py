import asyncio
import json
import unittest
from collections.abc import Awaitable, Callable

from open_edifier import (
    RejectedError,
    S260Client,
    S260EventStream,
    VerificationTimeoutError,
    VolumeEvent,
)
from open_edifier.protocol import BINARY_FRAME_HEADER, JSON_FRAME_HEADER


Handler = Callable[[asyncio.StreamReader, asyncio.StreamWriter], Awaitable[None]]


class ClientTests(unittest.IsolatedAsyncioTestCase):
    def test_rejects_invalid_endpoint_and_timeout_configuration(self) -> None:
        with self.assertRaises(ValueError):
            S260Client("", 8080)
        with self.assertRaises(ValueError):
            S260Client("127.0.0.1", 0)
        with self.assertRaises(ValueError):
            S260Client("127.0.0.1", verification_timeout=float("nan"))

    async def test_volume_verification_retries_until_target_is_visible(self) -> None:
        status_reads = 0

        async def handler(
            reader: asyncio.StreamReader, writer: asyncio.StreamWriter
        ) -> None:
            nonlocal status_reads
            try:
                for _ in range(4):
                    request = await read_request(reader)
                    request_id = request["id"]
                    if request["payload"] == "settings":
                        self.assertEqual(request["player"]["volume"], 19)
                        response = ack(request_id)
                    else:
                        status_reads += 1
                        response = status(request_id, 18 if status_reads < 3 else 19)
                    writer.write(json_frame(response))
                    await writer.drain()
            finally:
                writer.close()
                await writer.wait_closed()

        server, port = await start_server(handler)
        try:
            async with S260Client("127.0.0.1", port) as client:
                updated = await client.set_volume(19)
            self.assertEqual(updated.volume.current, 19)
            self.assertEqual(status_reads, 3)
        finally:
            server.close()
            await server.wait_closed()

    async def test_rejected_error_does_not_expose_other_response_fields(self) -> None:
        async def handler(
            reader: asyncio.StreamReader, writer: asyncio.StreamWriter
        ) -> None:
            try:
                request = await read_request(reader)
                writer.write(
                    json_frame(
                        {
                            "id": request["id"],
                            "code": 7,
                            "message": "rejected",
                            "wifiName": "private-network",
                            "bluetoothPairingRecord": [{"name": "private-device"}],
                        }
                    )
                )
                await writer.drain()
            finally:
                writer.close()
                await writer.wait_closed()

        server, port = await start_server(handler)
        try:
            async with S260Client("127.0.0.1", port) as client:
                with self.assertRaises(RejectedError) as caught:
                    await client.status()
            self.assertEqual(caught.exception.code, 7)
            self.assertNotIn("private-network", str(caught.exception))
            self.assertNotIn("private-device", str(caught.exception))
        finally:
            server.close()
            await server.wait_closed()

    async def test_source_equalizer_and_playback_use_verified_fields(self) -> None:
        source = 3
        equalizer = 0

        async def handler(
            reader: asyncio.StreamReader, writer: asyncio.StreamWriter
        ) -> None:
            nonlocal source, equalizer
            try:
                for _ in range(7):
                    request = await read_request(reader)
                    request_id = request["id"]
                    if request["payload"] == "status_query":
                        response = status(
                            request_id, 18, source=source, equalizer=equalizer
                        )
                    else:
                        if "inputSource" in request:
                            self.assertEqual(request["inputSource"]["inputIndex"], 1)
                            source = request["inputSource"]["selectedIndex"]
                        elif "soundEffect" in request:
                            equalizer = request["soundEffect"]["selectedIndex"]
                        else:
                            self.assertEqual(request["player"], {"previous": 1})
                        response = ack(request_id)
                    writer.write(json_frame(response))
                    await writer.drain()
            finally:
                writer.close()
                await writer.wait_closed()

        server, port = await start_server(handler)
        try:
            async with S260Client("127.0.0.1", port) as client:
                changed_source = await client.set_source("aux")
                changed_equalizer = await client.set_equalizer(1)
                await client.playback("previous")
            self.assertEqual(changed_source.source, "aux")
            self.assertEqual(changed_equalizer.equalizer.preset, 1)
        finally:
            server.close()
            await server.wait_closed()

    async def test_volume_verification_failure_is_bounded_and_structured(self) -> None:
        async def handler(
            reader: asyncio.StreamReader, writer: asyncio.StreamWriter
        ) -> None:
            try:
                while request_data := await reader.read(4096):
                    request = json.loads(request_data)
                    response = (
                        ack(request["id"])
                        if request["payload"] == "settings"
                        else status(request["id"], 18)
                    )
                    writer.write(json_frame(response))
                    await writer.drain()
            finally:
                writer.close()
                await writer.wait_closed()

        server, port = await start_server(handler)
        try:
            async with S260Client(
                "127.0.0.1",
                port,
                verification_timeout=0.06,
                verification_interval=0.01,
            ) as client:
                with self.assertRaises(VerificationTimeoutError) as caught:
                    await client.set_volume(19)
            self.assertEqual(caught.exception.field, "volume")
            self.assertEqual(caught.exception.expected, "19")
            self.assertEqual(caught.exception.actual, "18")
            self.assertGreaterEqual(caught.exception.attempts, 2)
        finally:
            server.close()
            await server.wait_closed()

    async def test_event_stream_ignores_heartbeat_and_decodes_volume(self) -> None:
        async def handler(
            _reader: asyncio.StreamReader, writer: asyncio.StreamWriter
        ) -> None:
            try:
                writer.write(binary_frame(0x003F, bytes(9)))
                writer.write(binary_frame(0x0066, bytes([30, 18])))
                await writer.drain()
            finally:
                writer.close()
                await writer.wait_closed()

        server, port = await start_server(handler)
        try:
            async with S260EventStream("127.0.0.1", port) as events:
                async with asyncio.timeout(2):
                    event = await events.next_event()
            self.assertEqual(event, VolumeEvent(18, 30))
        finally:
            server.close()
            await server.wait_closed()

    async def test_event_stream_reconnects_after_disconnect(self) -> None:
        connections = 0

        async def handler(
            _reader: asyncio.StreamReader, writer: asyncio.StreamWriter
        ) -> None:
            nonlocal connections
            connections += 1
            try:
                if connections == 1:
                    writer.write(binary_frame(0x003F, bytes(9)))
                else:
                    writer.write(binary_frame(0x0066, bytes([30, 12])))
                await writer.drain()
            finally:
                writer.close()
                await writer.wait_closed()

        server, port = await start_server(handler)
        try:
            async with S260EventStream("127.0.0.1", port) as events:
                async with asyncio.timeout(2):
                    event = await events.next_event()
            self.assertEqual(event, VolumeEvent(12, 30))
            self.assertEqual(connections, 2)
        finally:
            server.close()
            await server.wait_closed()


async def start_server(handler: Handler) -> tuple[asyncio.Server, int]:
    server = await asyncio.start_server(handler, "127.0.0.1", 0)
    return server, server.sockets[0].getsockname()[1]


async def read_request(reader: asyncio.StreamReader) -> dict[str, object]:
    data = await reader.read(4096)
    if not data:
        raise AssertionError("client closed before sending a request")
    value = json.loads(data)
    if not isinstance(value, dict):
        raise AssertionError("request is not an object")
    return value


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


def ack(request_id: object) -> dict[str, object]:
    return {"code": 0, "id": request_id, "payload": "settings", "message": "success"}


def status(
    request_id: object, volume: int, *, source: int = 3, equalizer: int = 0
) -> dict[str, object]:
    return {
        "code": 0,
        "id": request_id,
        "payload": "status_query",
        "message": "success",
        "deviceInfo": {
            "bluetoothName": "EDIFIER S260",
            "firmwareVersion": "01.00.00",
        },
        "inputSource": {"inputIndex": 1, "selectedIndex": source},
        "player": {
            "volume": volume,
            "minVolume": 0,
            "maxVolume": 30,
            "playerStatus": 0,
        },
        "soundEffect": {"selectedIndex": equalizer, "soundIndex": 3},
    }


if __name__ == "__main__":
    unittest.main()
