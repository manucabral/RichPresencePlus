"""
Context (Page) - Unified execution context for browser tabs/pages.
"""

import json
import time
import threading
from dataclasses import dataclass, field
from typing import Optional, Dict, Any

from websockets.sync.client import connect

from .ws_client import WSClient
from ..logger import logger


@dataclass
class Context:
    """
    Unified execution context (CDP page or BiDi browsing context).
    Supports WebSocket connection for CDP protocol.
    """

    id: str
    url: str
    title: str
    protocol: str  # "cdp", "bidi", or "shim"
    ws_url: Optional[str] = None

    _ws: Optional[Any] = field(default=None, repr=False, compare=False)
    _connected: bool = field(default=False, repr=False, compare=False)
    _lock: threading.Lock = field(
        default_factory=threading.Lock, repr=False, compare=False
    )
    _next_id: int = field(default=0, repr=False, compare=False)
    _bidi_adapter: Optional[Any] = field(
        default=None, repr=False, compare=False
    )  # DISABLED
    _prefetched_media_session: Optional[Dict[str, Any]] = field(
        default=None, repr=False, compare=False
    )

    def __post_init__(self):
        self.minititle = (
            (self.title[:10] + "...")
            if self.title and len(self.title) > 13
            else self.title
        )

    def __repr__(self):
        id_short = self.id[:8] if self.id else ""
        url_short = self.url[:40] if self.url else ""
        return (
            f"Context(id={id_short}..., " f"url={url_short}, protocol={self.protocol})"
        )

    def __hash__(self):
        return hash((self.id, self.url))

    @property
    def connected(self) -> bool:
        """Return connection status."""
        return self._connected

    def is_ws_alive(self) -> bool:
        """Check if WebSocket connection is alive (CDP only)."""
        if self.protocol != "cdp":
            return self._bidi_adapter is not None
        if self._ws is None:
            return False
        if isinstance(self._ws, WSClient):
            return self._ws.is_alive()
        try:
            closed = getattr(self._ws, "closed", None)
            if closed is not None:
                return not bool(closed)
        except Exception:
            pass
        return False

    def connect_if_needed(self, timeout: float = 5.0) -> Any:
        """Connect only if there is no active connection."""
        if self.is_ws_alive() and self._connected:
            logger.debug("Reusing connection for %s", self.id)
            return self._ws if self.protocol == "cdp" else self._bidi_adapter
        return self.connect(timeout=timeout)

    def connect(self, timeout: float = 5.0) -> Any:
        """Establish connection to the context."""
        if self.protocol == "bidi":
            # BiDi uses adapter's persistent connection
            logger.debug("BiDi context %s uses adapter connection", self.id)
            self._connected = True
            return self._bidi_adapter

        if self.protocol == "shim":
            logger.debug("Shim context %s - no connection needed", self.id)
            self._connected = True
            return None

        # CDP: WebSocket per page
        if not self.ws_url:
            logger.error("No WebSocket URL for context %s", self.minititle or self.url)
            raise ValueError("Context has no webSocketDebuggerUrl")

        with self._lock:
            if (
                self._ws is not None
                and isinstance(self._ws, WSClient)
                and self._ws.is_alive()
            ):
                self._connected = True
                return self._ws

            logger.info("Connecting to CDP context %s", self.id)
            try:
                raw_ws = connect(self.ws_url, open_timeout=timeout)
            except Exception as exc:
                self._connected = False
                logger.debug("WebSocket connection failed for %s: %s", self.ws_url, exc)
                raise

            ws = WSClient(raw_ws)
            self._ws = ws
            self._connected = True
            logger.info("Connected to %s (%s)", self.id, self.minititle)
            return ws

    def close(self) -> None:
        """Close connection."""
        with self._lock:
            if self.protocol == "cdp" and self._ws is not None:
                try:
                    if isinstance(self._ws, WSClient):
                        self._ws.close()
                    else:
                        self._ws.close()
                except Exception:
                    pass
                finally:
                    self._ws = None
                logger.debug(
                    "Closed CDP connection for id=%s title=%s", self.id, self.minititle
                )
            self._connected = False

    def _next_message_id(self) -> int:
        with self._lock:
            self._next_id += 1
            return self._next_id

    def send(
        self, method: str, params: Optional[dict] = None, timeout: float = 5.0
    ) -> dict:
        """Send CDP/BiDi message and wait for response."""
        if self.protocol == "bidi":
            return self._send_bidi(method, params)
        return self._send_cdp(method, params, timeout)

    def _send_cdp(
        self, method: str, params: Optional[dict] = None, timeout: float = 5.0
    ) -> dict:
        """Send CDP message."""
        if self._ws is None:
            raise ValueError("WebSocket not connected. Call connect() first.")

        msg_id = self._next_message_id()
        payload = {"id": msg_id, "method": method}
        if params:
            payload["params"] = params

        try:
            self._ws.send(json.dumps(payload))
        except Exception:
            self._connected = False
            raise

        deadline = time.time() + timeout
        while time.time() < deadline:
            raw = self._ws.recv()
            try:
                msg = json.loads(raw)
            except Exception:
                continue
            if isinstance(msg, dict) and msg.get("id") == msg_id:
                return msg
        raise TimeoutError(f"Timeout waiting for response to {method} (id={msg_id})")

    def _send_bidi(self, method: str, params: Optional[dict] = None) -> dict:
        """Send BiDi message through adapter."""
        if self._bidi_adapter is None:
            raise ValueError("BiDi adapter not available")

        # Map CDP methods to BiDi equivalents
        if method == "Runtime.evaluate":
            expression = params.get("expression", "") if params else ""
            await_promise = params.get("awaitPromise", False) if params else False
            result = self._bidi_adapter.evaluate_script(
                self.id, expression, await_promise
            )
            return {"result": {"result": result}}

        raise NotImplementedError(f"BiDi method mapping for {method} not implemented")

    def evaluate(
        self, expression: str, return_by_value: bool = True, timeout: float = 5.0
    ) -> Any:
        """Execute JavaScript and return result."""
        # Shim protocol cannot execute JS
        if self.protocol == "shim":
            raise ValueError("Shim context cannot execute JS directly")

        if self.protocol == "bidi":
            if self._bidi_adapter is None:
                raise ValueError("BiDi adapter not available")
            result = self._bidi_adapter.evaluate_script(
                self.id, expression, await_promise=True
            )
            if result.get("error"):
                raise RuntimeError(f"BiDi error: {result['error']}")
            return result.get("value")

        # CDP
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
        """Retrieve Media Session metadata from the page."""
        # fr shim protocol in worker processes, return pre-fetched data
        if self.protocol == "shim" and self._prefetched_media_session is not None:
            logger.debug("Returning pre-fetched media session for %s", self.url)
            return self._prefetched_media_session

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

    def __enter__(self) -> "Context":
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        try:
            self.close()
        except Exception:
            logger.debug("Error during __exit__ close", exc_info=True)

    def __del__(self) -> None:
        try:
            self.close()
        except Exception:
            pass


# for backward compatibility
Page = Context
