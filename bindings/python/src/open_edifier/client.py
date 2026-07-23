"""Async S260 control and event clients."""

from __future__ import annotations

import asyncio
import itertools
import math
import time
from collections import deque
from contextlib import suppress
from typing import Any, AsyncIterator, Callable

from .errors import (
    InvalidEqualizerPresetError,
    InvalidVolumeError,
    NetworkError,
    NotConnectedError,
    ProtocolError,
    RejectedError,
    RequestTimeoutError,
    VerificationTimeoutError,
)
from .models import DeviceEvent, DeviceStatus
from .protocol import (
    BinaryFrameDecoder,
    JsonFrameDecoder,
    ParsedStatus,
    decode_event,
    encode_request,
    normalize_source,
    parse_status,
    source_index,
)

DEFAULT_PORT = 8080
_REQUEST_SEQUENCE = itertools.count()


class S260Client:
    """One serialized async control connection to an S260 speaker."""

    def __init__(
        self,
        host: str,
        port: int = DEFAULT_PORT,
        *,
        connect_timeout: float = 5.0,
        request_timeout: float = 5.0,
        verification_timeout: float = 1.0,
        verification_interval: float = 0.05,
    ) -> None:
        _validate_endpoint(host, port)
        _validate_timeouts(
            connect_timeout,
            request_timeout,
            verification_timeout,
            verification_interval,
        )
        self.host = host
        self.port = port
        self.connect_timeout = connect_timeout
        self.request_timeout = request_timeout
        self.verification_timeout = verification_timeout
        self.verification_interval = verification_interval
        self._reader: asyncio.StreamReader | None = None
        self._writer: asyncio.StreamWriter | None = None
        self._decoder = JsonFrameDecoder()
        self._lock = asyncio.Lock()

    async def __aenter__(self) -> S260Client:
        await self.connect()
        return self

    async def __aexit__(self, *_: object) -> None:
        await self.close()

    async def connect(self) -> None:
        """Open the control connection within the configured timeout."""
        async with self._lock:
            if self._writer is not None:
                return
            try:
                async with asyncio.timeout(self.connect_timeout):
                    self._reader, self._writer = await asyncio.open_connection(
                        self.host, self.port
                    )
            except TimeoutError as error:
                raise RequestTimeoutError("connect", self.connect_timeout) from error
            except OSError as error:
                raise NetworkError("connect", str(error)) from error
            self._decoder = JsonFrameDecoder()

    async def close(self) -> None:
        """Close the control connection."""
        async with self._lock:
            await self._close_locked()

    async def status(self) -> DeviceStatus:
        """Return the current privacy-safe speaker state."""
        async with self._lock:
            return (await self._status_locked()).public

    async def set_source(self, source: str) -> DeviceStatus:
        """Select an input source and verify the resulting state."""
        selected_source = normalize_source(source)
        async with self._lock:
            current = await self._status_locked()
            await self._request_locked(
                "settings",
                {
                    "inputSource": {
                        "inputIndex": current.input_index,
                        "selectedIndex": source_index(selected_source),
                    }
                },
            )
            return await self._verify_locked(
                "source",
                selected_source,
                lambda status: status.source == selected_source,
                lambda status: status.source,
            )

    async def set_volume(self, volume: int) -> DeviceStatus:
        """Set a device-bounded volume and verify the resulting state."""
        if type(volume) is not int:
            raise TypeError("volume must be an integer")
        async with self._lock:
            current = await self._status_locked()
            if (
                not current.public.volume.minimum
                <= volume
                <= current.public.volume.maximum
            ):
                raise InvalidVolumeError(
                    volume,
                    current.public.volume.minimum,
                    current.public.volume.maximum,
                )
            await self._request_locked("settings", {"player": {"volume": volume}})
            return await self._verify_locked(
                "volume",
                str(volume),
                lambda status: status.volume.current == volume,
                lambda status: str(status.volume.current),
            )

    async def set_equalizer(self, preset: int) -> DeviceStatus:
        """Select an equalizer preset and verify the resulting state."""
        if type(preset) is not int:
            raise TypeError("equalizer preset must be an integer")
        async with self._lock:
            current = await self._status_locked()
            equalizer = current.public.equalizer
            if equalizer is None:
                raise ProtocolError("status did not contain equalizer state")
            if not 0 <= preset < equalizer.preset_count:
                raise InvalidEqualizerPresetError(preset, equalizer.preset_count)
            await self._request_locked(
                "settings", {"soundEffect": {"selectedIndex": preset}}
            )
            return await self._verify_locked(
                "equalizer",
                str(preset),
                lambda status: (
                    status.equalizer is not None and status.equalizer.preset == preset
                ),
                lambda status: (
                    "missing"
                    if status.equalizer is None
                    else str(status.equalizer.preset)
                ),
            )

    async def playback(self, action: str) -> None:
        """Send a playback command acknowledged by the speaker."""
        player = {
            "play": {"playerStatus": 1},
            "pause": {"playerStatus": 0},
            "next": {"next": 1},
            "previous": {"previous": 1},
        }.get(action)
        if player is None:
            raise ValueError("playback action must be play, pause, next, or previous")
        async with self._lock:
            await self._request_locked("settings", {"player": player})

    async def _status_locked(self, timeout: float | None = None) -> ParsedStatus:
        return parse_status(await self._request_locked("status_query", {}, timeout))

    async def _request_locked(
        self,
        payload: str,
        settings: dict[str, Any],
        timeout: float | None = None,
    ) -> dict[str, Any]:
        if self._reader is None or self._writer is None:
            raise NotConnectedError()
        bounded_timeout = self.request_timeout if timeout is None else timeout
        request_id = _request_id()
        request = {"id": request_id, "payload": payload, **settings}
        try:
            async with asyncio.timeout(bounded_timeout):
                self._writer.write(encode_request(request))
                await self._writer.drain()
                while True:
                    chunk = await self._reader.read(8192)
                    if not chunk:
                        raise NetworkError("read", "speaker closed the connection")
                    malformed: ProtocolError | None = None
                    for response in self._decoder.feed(chunk):
                        if isinstance(response, ProtocolError):
                            malformed = malformed or response
                            continue
                        if not isinstance(response, dict):
                            malformed = malformed or ProtocolError(
                                "response must be a JSON object"
                            )
                            continue
                        if response.get("id") != request_id:
                            continue
                        _check_response_result(response)
                        return response
                    if malformed is not None:
                        raise malformed
        except TimeoutError as error:
            await self._close_locked()
            raise RequestTimeoutError("request", bounded_timeout) from error
        except OSError as error:
            await self._close_locked()
            raise NetworkError("request", str(error)) from error
        except NetworkError:
            await self._close_locked()
            raise

    async def _verify_locked(
        self,
        field: str,
        expected: str,
        matches: Callable[[DeviceStatus], bool],
        actual: Callable[[DeviceStatus], str],
    ) -> DeviceStatus:
        loop = asyncio.get_running_loop()
        started = loop.time()
        deadline = started + self.verification_timeout
        attempts = 0
        last_actual = "unobserved"
        while (remaining := deadline - loop.time()) > 0:
            attempts += 1
            try:
                status = (
                    await self._status_locked(min(remaining, self.request_timeout))
                ).public
            except RequestTimeoutError:
                if loop.time() >= deadline:
                    break
                raise
            if matches(status):
                return status
            last_actual = actual(status)
            remaining = deadline - loop.time()
            if remaining > 0:
                await asyncio.sleep(min(remaining, self.verification_interval))
        raise VerificationTimeoutError(
            field,
            expected,
            last_actual,
            attempts,
            int((loop.time() - started) * 1000),
        )

    async def _close_locked(self) -> None:
        writer, self._writer = self._writer, None
        self._reader = None
        self._decoder = JsonFrameDecoder()
        if writer is not None:
            writer.close()
            with suppress(OSError):
                await writer.wait_closed()


class S260EventStream(AsyncIterator[DeviceEvent]):
    """Cancellable event connection with bounded exponential reconnect backoff."""

    def __init__(
        self,
        host: str,
        port: int = DEFAULT_PORT,
        *,
        connect_timeout: float = 5.0,
    ) -> None:
        _validate_endpoint(host, port)
        if not _is_positive_timeout(connect_timeout):
            raise ValueError("connect_timeout must be a finite positive number")
        self.host = host
        self.port = port
        self.connect_timeout = connect_timeout
        self._reader: asyncio.StreamReader | None = None
        self._writer: asyncio.StreamWriter | None = None
        self._decoder = BinaryFrameDecoder()
        self._pending: deque[DeviceEvent] = deque()
        self._closed = True
        self._reconnect_delay = 0.2
        self._read_lock = asyncio.Lock()

    async def __aenter__(self) -> S260EventStream:
        await self.connect()
        return self

    async def __aexit__(self, *_: object) -> None:
        await self.close()

    def __aiter__(self) -> S260EventStream:
        return self

    async def __anext__(self) -> DeviceEvent:
        if self._closed:
            raise StopAsyncIteration
        return await self.next_event()

    async def connect(self) -> None:
        """Open the initial event connection."""
        if not self._closed:
            return
        await self._open()
        self._closed = False

    async def close(self) -> None:
        """Close the event connection; callers should cancel any pending read first."""
        self._closed = True
        await self._drop_connection()

    async def next_event(self) -> DeviceEvent:
        """Wait for the next event; cancellation stops the wait immediately."""
        if self._closed:
            raise NotConnectedError()
        async with self._read_lock:
            while not self._closed:
                if self._pending:
                    return self._pending.popleft()
                if self._reader is None:
                    await asyncio.sleep(self._reconnect_delay)
                    try:
                        await self._open()
                    except NetworkError:
                        self._reconnect_delay = min(self._reconnect_delay * 2, 5.0)
                    continue
                try:
                    chunk = await self._reader.read(4096)
                except OSError:
                    await self._drop_connection()
                    continue
                if not chunk:
                    await self._drop_connection()
                    continue
                for command, payload in self._decoder.feed(chunk):
                    event = decode_event(command, payload)
                    if event is not None:
                        self._pending.append(event)
            raise NotConnectedError()

    async def _open(self) -> None:
        try:
            async with asyncio.timeout(self.connect_timeout):
                self._reader, self._writer = await asyncio.open_connection(
                    self.host, self.port
                )
        except TimeoutError as error:
            raise RequestTimeoutError("connect events", self.connect_timeout) from error
        except OSError as error:
            raise NetworkError("connect events", str(error)) from error
        self._decoder = BinaryFrameDecoder()
        self._reconnect_delay = 0.2

    async def _drop_connection(self) -> None:
        writer, self._writer = self._writer, None
        self._reader = None
        self._decoder = BinaryFrameDecoder()
        if writer is not None:
            writer.close()
            with suppress(OSError):
                await writer.wait_closed()


def _validate_timeouts(
    connect_timeout: float,
    request_timeout: float,
    verification_timeout: float,
    verification_interval: float,
) -> None:
    values = (
        connect_timeout,
        request_timeout,
        verification_timeout,
        verification_interval,
    )
    if any(not _is_positive_timeout(value) for value in values) or (
        verification_interval > verification_timeout
    ):
        raise ValueError(
            "timeouts must be positive and verification_interval must not exceed "
            "verification_timeout"
        )


def _is_positive_timeout(value: object) -> bool:
    return (
        not isinstance(value, bool)
        and isinstance(value, (int, float))
        and math.isfinite(value)
        and value > 0
    )


def _validate_endpoint(host: str, port: int) -> None:
    if not isinstance(host, str) or not host:
        raise ValueError("host must be a non-empty string")
    if type(port) is not int or not 1 <= port <= 65535:
        raise ValueError("port must be an integer between 1 and 65535")


def _request_id() -> str:
    millis = time.time_ns() // 1_000_000
    sequence = next(_REQUEST_SEQUENCE) % 1000
    return f"{millis}{sequence:03}"


def _check_response_result(response: dict[str, Any]) -> None:
    code = response.get("code")
    message = response.get("message")
    if type(code) is not int:
        raise ProtocolError("response did not contain an integer code")
    if not isinstance(message, str):
        raise ProtocolError("response did not contain a message")
    if code != 0 or message != "success":
        sanitized = "".join(
            character for character in message if character.isprintable()
        )[:160]
        raise RejectedError(code, sanitized)
