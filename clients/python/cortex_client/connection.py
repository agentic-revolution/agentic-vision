# Copyright 2026 Cortex Contributors
# SPDX-License-Identifier: Apache-2.0
"""Unix socket connection to the Cortex runtime."""

from __future__ import annotations

import json
import socket
import time
from typing import Any

from .errors import CortexConnectionError, CortexTimeoutError

DEFAULT_SOCKET_PATH = "/tmp/cortex.sock"
DEFAULT_TIMEOUT = 60.0


class Connection:
    """Low-level connection to the Cortex runtime via Unix domain socket."""

    def __init__(
        self,
        socket_path: str = DEFAULT_SOCKET_PATH,
        timeout: float = DEFAULT_TIMEOUT,
    ) -> None:
        self._socket_path = socket_path
        self._timeout = timeout
        self._sock: socket.socket | None = None
        self._buffer = b""

    def connect(self) -> None:
        """Connect to the Cortex runtime socket."""
        try:
            self._sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            self._sock.settimeout(self._timeout)
            self._sock.connect(self._socket_path)
        except FileNotFoundError:
            raise CortexConnectionError(
                f"Cortex is not running (socket not found: {self._socket_path})"
            )
        except ConnectionRefusedError:
            raise CortexConnectionError(
                f"Cortex refused connection at {self._socket_path}"
            )
        except OSError as e:
            raise CortexConnectionError(f"Cannot connect to Cortex: {e}")

    def close(self) -> None:
        """Close the connection."""
        if self._sock:
            try:
                self._sock.close()
            except OSError:
                pass
            self._sock = None
        self._buffer = b""

    def send(self, method: str, params: dict[str, Any] | None = None) -> dict[str, Any]:
        """Send a request and return the response.

        Args:
            method: Protocol method name.
            params: Method parameters.

        Returns:
            The response dict with 'result' or 'error' key.

        Raises:
            CortexConnectionError: If not connected or connection broken.
            CortexTimeoutError: If the operation times out.
        """
        if self._sock is None:
            self.connect()

        request = {
            "id": f"req-{time.monotonic_ns()}",
            "method": method,
            "params": params or {},
        }

        request_bytes = json.dumps(request).encode("utf-8") + b"\n"

        try:
            assert self._sock is not None
            self._sock.sendall(request_bytes)
        except BrokenPipeError:
            # Try to reconnect once
            self.close()
            self.connect()
            assert self._sock is not None
            self._sock.sendall(request_bytes)
        except socket.timeout:
            raise CortexTimeoutError(f"Timeout sending {method} request")

        return self._read_response()

    def _read_response(self) -> dict[str, Any]:
        """Read a newline-delimited JSON response."""
        assert self._sock is not None
        while b"\n" not in self._buffer:
            try:
                chunk = self._sock.recv(65536)
            except socket.timeout:
                raise CortexTimeoutError("Timeout waiting for response")
            if not chunk:
                raise CortexConnectionError("Connection closed by server")
            self._buffer += chunk

        line, self._buffer = self._buffer.split(b"\n", 1)
        result: dict[str, Any] = json.loads(line.decode("utf-8"))
        return result

    @property
    def is_connected(self) -> bool:
        return self._sock is not None

    def __enter__(self) -> Connection:
        self.connect()
        return self

    def __exit__(self, *args: object) -> None:
        self.close()
