"""
Page representation and CDP WebSocket connection management.
"""

import threading
import json
import time
from typing import Any, Optional
from websockets.sync.client import connect as ws_connect
from .logger import logger


class _WSClient:
    """
    Wrapper around a WebSocket client to provide thread-safe send/recv/close.
    """

    def __init__(self, ws: Any) -> None:
        """
        Initializes the WebSocket client wrapper.
        """
        self._ws = ws
        self._lock = threading.Lock()

    def send(self, data: str) -> None:
        """
        Sends data through the WebSocket.

        Args:
            data (str): The data to send.
        """
        with self._lock:
            if self._ws is None:
                raise ValueError("WebSocket is closed")
            self._ws.send(data)

    def recv(self) -> Any:
        """
        Receives data from the WebSocket.

        Returns:
            Any: The received data.
        """
        with self._lock:
            if self._ws is None:
                raise ValueError("WebSocket is closed")
            return self._ws.recv()

    def close(self) -> None:
        """
        Closes the WebSocket connection.
        """
        try:
            with self._lock:
                try:
                    if self._ws is not None:
                        self._ws.close()
                except Exception as exc:
                    logger.debug("Error closing WebSocket: %s", exc)
        finally:
            self._ws = None

    def is_alive(self) -> bool:
        """
        Checks if the WebSocket connection is alive.

        Returns:
            bool: True if the connection is alive, False otherwise.
        """
        try:
            closed = getattr(self._ws, "closed", None)
            if closed is not None:
                return not bool(closed)
        except Exception:
            return False
        return self._ws is not None


class Page:
    """
    Represents a browser page with CDP WebSocket connection.
    """

    def __init__(
        self,
        id_: Optional[str] = None,
        url: Optional[str] = None,
        title: Optional[str] = None,
        ws_url: Optional[str] = None,
        chromium: bool = True,
    ) -> None:
        """
        Initializes the Page instance.
        """
        self.id: Optional[str] = id_
        self.url: Optional[str] = url
        self.title: Optional[str] = title
        self.minititle: Optional[str] = (
            (title[:10] + "...") if title and len(title) > 13 else title
        )
        self.ws_url: Optional[str] = ws_url
        self.connected: bool = False
        self._ws: Optional[Any] = None
        self.chromium: bool = chromium
        self._lock = threading.Lock()
        self._next_id: int = 0

    def __eq__(self, other: Any) -> bool:
        """Compares pages by id and url, not by object identity."""
        if not isinstance(other, Page):
            return False
        return self.id == other.id and self.url == other.url

    def __hash__(self) -> int:
        """Hash by id and url for use in sets/dicts."""
        return hash((self.id, self.url))

    def is_ws_alive(self) -> bool:
        """Checks if the WebSocket connection is alive."""
        if self._ws is None:
            return False
        if isinstance(self._ws, _WSClient):
            return self._ws.is_alive()
        # fallback for raw websocket objects
        try:
            closed = getattr(self._ws, "closed", None)
            if closed is not None:
                return not bool(closed)
        except Exception:
            pass
        return False

    def connect_if_needed(self, timeout: float = 5.0) -> Any:
        """Connects only if there is no active connection."""
        if self.is_ws_alive() and self.connected:
            logger.debug("Reusing WebSocket connection for %s", self.id)
            return self._ws
        logger.debug("Connection not available for %s, connecting...", self.id)
        return self.connect(timeout=timeout)

    def connect(self, timeout: float = 5.0) -> Any:
        """Establishes a WebSocket connection to the page's CDP endpoint."""
        if not self.ws_url:
            logger.error(
                "No WebSocket URL available for page %s", self.minititle or self.url
            )
            raise ValueError("Page has no webSocketDebuggerUrl")
        with self._lock:
            if self._ws is not None:
                try:
                    if isinstance(self._ws, _WSClient) and self._ws.is_alive():
                        self.connected = True
                        logger.debug("WebSocket already open for %s", self.id)
                        return self._ws
                except Exception:
                    logger.debug("Trying reconnect to %s", self.id)
            logger.info("Trying to connect to %s", self.id)
            try:
                raw_ws = ws_connect(self.ws_url, open_timeout=timeout)
            except Exception as exc:
                self.connected = False
                logger.debug("WebSocket connection failed for %s: %s", self.ws_url, exc)
                raise

            ws = _WSClient(raw_ws)
            self._ws = ws
            self.connected = True
            logger.info("Success: %s", self.ws_url)
            logger.info(
                "Connected to %s (%s)",
                self.id,
                self.minititle,
            )
            return ws

    def close(self) -> None:
        """Closes the WebSocket connection."""
        with self._lock:
            if self._ws is None:
                return
            try:
                try:
                    if isinstance(self._ws, _WSClient):
                        self._ws.close()
                    else:
                        try:
                            self._ws.close()
                        except Exception:
                            pass
                finally:
                    self._ws = None
                    self.connected = False
                logger.debug("Closed for id=%s title=%s", self.id, self.minititle)
            except Exception:
                logger.warning(
                    "Error closing WebSocket for id=%s title=%s",
                    self.id,
                    self.minititle,
                    exc_info=True,
                )

    def _next_message_id(self) -> int:
        """Generates the next message ID in a thread-safe manner."""
        with self._lock:
            self._next_id += 1
            return self._next_id

    def send(
        self, method: str, params: Optional[dict] = None, timeout: float = 5.0
    ) -> dict:
        """Sends a CDP message and waits for the response with the same id (blocking)."""
        if self._ws is None:
            raise ValueError("WebSocket not connected. Call connect() first.")
        msg_id = self._next_message_id()
        payload = {"id": msg_id, "method": method}
        if params:
            payload["params"] = params
        try:
            self._ws.send(json.dumps(payload))
        except Exception:
            try:
                self.connected = False
            except Exception:
                pass
            raise
        deadline = time.time() + timeout
        while time.time() < deadline:
            raw = self._ws.recv()
            try:
                msg = json.loads(raw)
            except Exception:
                logger.debug("Error parsing WebSocket message: %s", raw)
                continue
            if isinstance(msg, dict) and msg.get("id") == msg_id:
                return msg
        raise TimeoutError(f"Timeout waiting for response to {method} (id={msg_id})")

    def evaluate(
        self, expression: str, return_by_value: bool = True, timeout: float = 5.0
    ) -> Any:
        """Runtime.evaluate wrapper. Returns the deserialized value if return_by_value=True."""
        params = {"expression": expression, "awaitPromise": True}
        if return_by_value:
            params["returnByValue"] = True
        resp = self.send("Runtime.evaluate", params=params, timeout=timeout)
        if "error" in resp:
            raise RuntimeError(f"CDP error: {resp['error']}")
        inner = resp.get("result", {}).get("result")
        if inner is None:
            return None
        if "value" in inner:
            return inner["value"]
        return inner

    def get_media_session(self, timeout: float = 5.0) -> Optional[dict]:
        """Retrieves the Media Session metadata from the page."""
        expression = """
        (function(){
          try {
            const ms = navigator.mediaSession;
            if(!ms) return null;
            const md = ms.metadata || {};
            return {
              title: md.title || null,
              artist: md.artist || null,
              album: md.album || null,
              artwork: (md.artwork && md.artwork[0]) ? (md.artwork[0].src || null) : null,
              playbackState: navigator.mediaSession.playbackState || null
            };
          } catch(e) {
            return {__error: e && e.message};
          }
        })()
        """
        return self.evaluate(expression, return_by_value=True, timeout=timeout)

    def __enter__(self) -> "Page":
        """Context manager enter: returns self."""
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        logger.debug("Exiting %s for id=%s title=%s", exc_type, self.id, self.minititle)
        try:
            self.close()
        except Exception:
            logger.debug("Error during __exit__ close", exc_info=True)

    def __del__(self) -> None:
        try:
            self.close()
        except Exception:
            pass
