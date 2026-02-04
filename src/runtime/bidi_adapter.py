"""
BiDi Adapter - Firefox WebDriver BiDi implementation (DISABLED).
"""

import json
import time
import threading
from typing import List, Dict, Any, Optional

from websockets.sync.client import connect, ClientConnection
from websockets.exceptions import WebSocketException
from websockets.protocol import State

from .protocol_adapter import ProtocolAdapter
from .context import Context
from ..logger import logger


class BiDiAdapter(ProtocolAdapter):
    """
    Firefox BiDi WebDriver adapter.
    NOTE: BiDi is currently disabled. Use Chromium-based browsers instead.
    """

    _instance_counter = 0  # Class variable to count instances

    def __init__(self, host: str, port: int):
        BiDiAdapter._instance_counter += 1
        self._instance_id = BiDiAdapter._instance_counter
        self.host = host
        self.port = port
        self.connection: Optional[ClientConnection] = None
        self.session_id: Optional[str] = None
        self._next_id = 0
        self._contexts_cache: List[Context] = []
        self._lock = threading.RLock()  # Use RLock to allow reentrant locking
        logger.debug("BiDiAdapter instance #%d created", self._instance_id)

    def _ws_url(self) -> str:
        return f"ws://{self.host}:{self.port}/session"

    def _get_next_id(self) -> int:
        self._next_id += 1
        return self._next_id

    def _send_command(
        self, method: str, params: Optional[Dict] = None, timeout: float = 5.0
    ) -> Dict[str, Any]:
        """Send BiDi command and wait for response."""
        if not self.connection:
            raise RuntimeError("BiDi not connected")

        msg_id = self._get_next_id()
        payload = {"id": msg_id, "method": method, "params": params or {}}

        self.connection.send(json.dumps(payload))

        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                raw = self.connection.recv()
                msg = json.loads(raw)

                if isinstance(msg, dict) and msg.get("id") == msg_id:
                    return msg

            except json.JSONDecodeError:
                continue

        raise TimeoutError(f"Timeout waiting for {method}")

    def connect(self) -> bool:
        """Establish BiDi WebSocket connection and reuse or create session."""
        with self._lock:
            # dont reconnect if already connected with valid session
            if self.is_connected():
                logger.debug(
                    "BiDiAdapter #%d: Already connected with session %s",
                    self._instance_id,
                    self.session_id,
                )
                return True

            try:
                logger.debug(
                    "BiDiAdapter #%d: Connecting to BiDi at %s",
                    self._instance_id,
                    self._ws_url(),
                )

                # close existing connection if any (its invalid at this point)
                if self.connection:
                    try:
                        self.connection.close()
                    except Exception:
                        pass
                    self.connection = None
                    self.session_id = None  # session is tied to connection

                self.connection = connect(self._ws_url(), open_timeout=3.0)

                # create a new session for this connection
                logger.debug("Creating new BiDi session for this connection")
                response = self._send_command(
                    "session.new", {"capabilities": {"alwaysMatch": {}}}
                )

                if response.get("type") == "error":
                    error_msg = response.get("error", "Unknown error")
                    logger.error("BiDi session creation failed: %s", error_msg)
                    # cclean up, session creation failed, connection is useless
                    self.session_id = None
                    return False

                self.session_id = response.get("result", {}).get("sessionId")
                logger.info(
                    "BiDiAdapter #%d: BiDi session created: %s",
                    self._instance_id,
                    self.session_id,
                )
                return True

            except WebSocketException as exc:
                logger.error("BiDi connection failed: %s", exc)
                self.session_id = None
                return False
            except Exception as exc:
                logger.error("BiDi unexpected error: %s", exc)
            self.session_id = None
            return False

    def get_contexts(self) -> List[Context]:
        """Get BiDi browsing contexts."""
        # ensure we have an active connection
        logger.debug(
            "BiDiAdapter #%d: get_contexts: connection=%s, session_id=%s",
            self._instance_id,
            self.connection is not None,
            self.session_id,
        )
        if not self.connection or not self.is_connected():
            logger.debug("No BiDi connection, attempting to connect")
            if not self.connect():
                logger.error("Failed to connect to BiDi")
                return []

        try:
            response = self._send_command("browsingContext.getTree")

            if response.get("type") == "error":
                logger.error("Failed to get BiDi contexts")
                return []

            raw_contexts = response.get("result", {}).get("contexts", [])
            contexts = []

            for item in raw_contexts:
                context = Context(
                    id=item.get("context", ""),
                    url=item.get("url", ""),
                    title=item.get("title", ""),
                    protocol="bidi",
                    ws_url=None,
                    _bidi_adapter=self,
                )
                contexts.append(context)

            with self._lock:
                self._contexts_cache = contexts

            return contexts

        except Exception as exc:
            logger.error("BiDi get contexts failed: %s", exc)
            return []

    def evaluate_script(
        self, context_id: str, expression: str, await_promise: bool = False
    ) -> Dict[str, Any]:
        """Execute JavaScript using BiDi script.evaluate."""
        if not self.connection:
            raise RuntimeError("BiDi not connected")

        try:
            params = {
                "expression": expression,
                "target": {"context": context_id},
                "awaitPromise": await_promise,
                "resultOwnership": "none",
            }

            response = self._send_command("script.evaluate", params)

            if response.get("type") == "error":
                error = response.get("error", "Unknown error")
                return {"type": "error", "value": None, "error": error}

            result = response.get("result", {})
            return {
                "type": result.get("type", "undefined"),
                "value": result.get("value"),
                "error": None,
            }

        except Exception as exc:
            logger.error("BiDi evaluate failed: %s", exc)
            return {"type": "error", "value": None, "error": str(exc)}

    def close(self):
        """Close BiDi connection."""
        if self.connection:
            try:
                self.connection.close()
                logger.debug("BiDi connection closed")
            except Exception:
                pass
            finally:
                self.connection = None
                self.session_id = None

        with self._lock:
            self._contexts_cache = []

    def is_connected(self) -> bool:
        """Check if BiDi connection is active."""
        if self.connection is None:
            logger.debug("is_connected: connection is None")
            return False
        if self.session_id is None:
            logger.debug("is_connected: session_id is None")
            return False
        try:
            is_open = self.connection.state == State.OPEN
            logger.debug(
                "is_connected: connection.state=%s, is_open=%s",
                self.connection.state,
                is_open,
            )
            return is_open
        except Exception as exc:
            logger.debug("BiDi is_connected check failed: %s", exc)
            return False
