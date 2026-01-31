"""
Module for the RPC client.
"""

import typing
import json
import struct
import uuid
import os
import enum
import inspect
import time

from .logger import logger
from .utils import remove_none


class OperationCode(enum.Enum):
    """
    Discord IPC operation codes.
    """

    HANDSHAKE = 0
    FRAME = 1
    CLOSE = 2


class ActivityType(enum.Enum):
    """
    Discord activity types.
    """

    PLAYING = 0
    LISTENING = 2
    WATCHING = 3
    COMPETING = 5


class ClientRPC:
    """
    ClientRPC core class for Discord RPC.
    """

    def __init__(
        self,
        client_id: typing.Optional[typing.Union[str, int]] = None,
        debug: bool = False,
    ):
        self.debug = debug
        self.__client_id = "" if client_id is None else str(client_id)

        self.__socket: typing.Optional[typing.BinaryIO] = None
        self.__connected: bool = False
        self._pipe_connected: bool = True

        self._rpc_pid: int = os.getpid()

        pipe_template = r"\\.\pipe\discord-ipc-{}"
        for index in range(10):
            path = pipe_template.format(index)
            try:
                self.__socket = open(path, "w+b")
                if self.debug:
                    logger.info("Connected to Discord IPC: %s", path)
                break
            except FileNotFoundError:
                continue
            except OSError as e:
                logger.error("OS error opening pipe %s: %s", path, e)
        else:
            logger.error("Failed to connect to Discord IPC (is Discord running?)")
            self._pipe_connected = False

    def __send(self, payload: dict, operation_code: OperationCode) -> None:
        """
        Send a payload to the Discord IPC.
        """
        if not self.__socket:
            return
        try:
            if self.debug:
                logger.debug("IPC SEND op=%s payload=%s", operation_code.name, payload)
            raw = json.dumps(payload).encode("utf-8")
            packet = struct.pack("<ii", operation_code.value, len(raw)) + raw
            self.__socket.write(packet)
            self.__socket.flush()
        except OSError as e:
            logger.error("IPC send failed: %s", e)

    def __recv(self) -> dict:
        """
        Receive a payload from the Discord IPC.
        """
        if not self.__socket:
            return {}

        try:
            header = self.__socket.read(8)
            if not header or len(header) < 8:
                return {}

            _, size = struct.unpack("<ii", header)
            data = self.__socket.read(size)
            if not data or len(data) < size:
                return {}

            payload = json.loads(data.decode("utf-8"))
            if self.debug:
                logger.debug("IPC RECV payload=%s", payload)
            return payload
        except Exception as e:
            if self.debug:
                logger.debug("IPC recv failed: %s", e)
            return {}

    def connect(self) -> None:
        """
        Establish the RPC connection with Discord.
        """
        if not self._pipe_connected:
            logger.warning("Pipe not connected")
            return
        if self.__connected:
            return
        self.__handshake()

    def __handshake(self) -> None:
        self.__send(
            {"v": 1, "client_id": self.__client_id},
            OperationCode.HANDSHAKE,
        )

        data = self.__recv()

        if data.get("code") == 4000:
            raise ValueError(data.get("message", "Handshake error"))

        if data.get("evt") == "READY":
            self.__connected = True
            if self.debug:
                user = data.get("data", {}).get("user", {}).get("username")
                logger.info("Discord RPC ready (user=%s)", user)
        else:
            raise ValueError(f"Handshake failed: {data}")

    # pylint: disable=too-many-arguments
    def update(
        self,
        state: typing.Optional[str],
        details: typing.Optional[str],
        activity_type: typing.Optional[ActivityType],
        start_time: typing.Optional[int],
        end_time: typing.Optional[int],
        large_image: typing.Optional[str],
        large_text: typing.Optional[str],
        small_image: typing.Optional[str],
        small_text: typing.Optional[str],
        buttons: typing.Optional[list[dict]],
    ) -> None:
        """
        Update the Discord Rich Presence activity.
        """
        if not self._pipe_connected or not self.__connected:
            return

        if activity_type is not None and not isinstance(activity_type, ActivityType):
            raise ValueError("Invalid activity type")

        activity: dict[str, typing.Any] = {}

        if state is not None:
            activity["state"] = state
        if details is not None:
            activity["details"] = details
        if activity_type is not None:
            activity["type"] = activity_type.value

        timestamps = {}
        if start_time is not None:
            timestamps["start"] = start_time
        if end_time is not None:
            timestamps["end"] = end_time
        if timestamps:
            activity["timestamps"] = timestamps

        assets = {}
        if large_image is not None:
            assets["large_image"] = large_image
        if large_text is not None:
            assets["large_text"] = large_text
        if small_image is not None:
            assets["small_image"] = small_image
        if small_text is not None:
            assets["small_text"] = small_text
        if assets:
            activity["assets"] = assets

        if buttons:
            activity["buttons"] = buttons[:2]

        activity = remove_none(activity)

        payload = {
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": self._rpc_pid,
                "activity": activity,
            },
            "nonce": str(uuid.uuid4()),
        }

        # debug: who triggered update
        if self.debug:
            try:
                frm = inspect.stack()[1]
                mod = inspect.getmodule(frm[0])
                caller = mod.__name__ if mod else frm.filename
            except Exception:
                caller = "unknown"
            logger.debug("SET_ACTIVITY pid=%s caller=%s", self._rpc_pid, caller)

        self.__send(payload, OperationCode.FRAME)
        self.__recv()

    def clear_activity(self) -> None:
        """
        Clear the current Discord Rich Presence activity.
        """
        if not self._pipe_connected or not self.__connected:
            return

        payload = {
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": self._rpc_pid,
                "activity": None,  # IMPORTANT
            },
            "nonce": str(uuid.uuid4()),
        }

        if self.debug:
            logger.debug("Clearing activity pid=%s", self._rpc_pid)

        self.__send(payload, OperationCode.FRAME)
        self.__recv()

    def close(self) -> None:
        """
        Close the RPC connection cleanly.
        """
        if not self._pipe_connected or not self.__connected:
            return

        try:
            self.clear_activity()
            time.sleep(0.25)
        except Exception:
            pass

        # close IPC
        try:
            self.__send({}, OperationCode.CLOSE)
        except Exception:
            pass

        try:
            if self.__socket:
                self.__socket.close()
        except Exception as e:
            logger.error("Error closing IPC socket: %s", e)

        self.__connected = False
        if self.debug:
            logger.info("Discord RPC closed cleanly")
