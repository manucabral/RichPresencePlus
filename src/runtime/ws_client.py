"""
WebSocket client wrapper with thread-safe operations.
"""

import threading
from typing import Any

from ..logger import logger


class WSClient:
    """
    Wrapper around a WebSocket client to provide thread-safe send/recv/close.
    """

    def __init__(self, ws: Any) -> None:
        self._ws = ws
        self._lock = threading.Lock()

    def send(self, data: str) -> None:
        """Send data through the WebSocket."""
        with self._lock:
            if self._ws is None:
                raise ValueError("WebSocket is closed")
            self._ws.send(data)

    def recv(self) -> Any:
        """Receive data from the WebSocket."""
        with self._lock:
            if self._ws is None:
                raise ValueError("WebSocket is closed")
            return self._ws.recv()

    def close(self) -> None:
        """Close the WebSocket connection."""
        try:
            with self._lock:
                if self._ws is not None:
                    self._ws.close()
        except Exception as exc:
            logger.debug("Error closing WebSocket: %s", exc)
        finally:
            self._ws = None

    def is_alive(self) -> bool:
        """Check if WebSocket connection is alive."""
        try:
            if self._ws is None:
                return False
            closed = getattr(self._ws, "closed", None)
            if closed is not None:
                return not bool(closed)
        except Exception:
            return False
        return self._ws is not None
