"""Structured public errors for the OpenEdifier Python client."""


class OpenEdifierError(Exception):
    """Base class for all client failures."""


class NotConnectedError(OpenEdifierError):
    """Raised when an operation requires an open connection."""

    def __init__(self) -> None:
        super().__init__("client is not connected")


class NetworkError(OpenEdifierError):
    """A network operation failed."""

    def __init__(self, operation: str, message: str) -> None:
        self.operation = operation
        self.message = message
        super().__init__(f"{operation}: {message}")


class RequestTimeoutError(NetworkError):
    """A bounded network operation timed out."""

    def __init__(self, operation: str, timeout: float) -> None:
        self.timeout = timeout
        super().__init__(operation, f"timed out after {timeout:g}s")


class ProtocolError(OpenEdifierError):
    """The speaker returned malformed or inconsistent protocol data."""


class RejectedError(OpenEdifierError):
    """The speaker explicitly rejected a request."""

    def __init__(self, code: int, message: str) -> None:
        self.code = code
        self.message = message
        super().__init__(f"speaker rejected request with code {code}: {message}")


class UnsupportedSourceError(OpenEdifierError):
    """The requested input source is not supported by the S260 driver."""

    def __init__(self, source: str) -> None:
        self.source = source
        super().__init__(f"unsupported input source: {source}")


class InvalidVolumeError(OpenEdifierError):
    """The requested volume is outside the device-reported range."""

    def __init__(self, value: int, minimum: int, maximum: int) -> None:
        self.value = value
        self.minimum = minimum
        self.maximum = maximum
        super().__init__(f"volume {value} is outside {minimum}..={maximum}")


class InvalidEqualizerPresetError(OpenEdifierError):
    """The requested EQ preset is outside the device-reported range."""

    def __init__(self, value: int, preset_count: int) -> None:
        self.value = value
        self.preset_count = preset_count
        super().__init__(f"equalizer preset {value} is outside 0..{preset_count - 1}")


class VerificationTimeoutError(OpenEdifierError):
    """The device acknowledged a mutation but did not report the target state."""

    def __init__(
        self,
        field: str,
        expected: str,
        actual: str,
        attempts: int,
        elapsed_ms: int,
    ) -> None:
        self.field = field
        self.expected = expected
        self.actual = actual
        self.attempts = attempts
        self.elapsed_ms = elapsed_ms
        super().__init__(
            f"{field} verification timed out: expected {expected}, observed {actual} "
            f"after {attempts} attempts and {elapsed_ms}ms"
        )
