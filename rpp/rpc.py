"""
Module for the RPC client.
"""

import typing
import json
import struct
import uuid
import os
from enum import Enum
from .logger import get_logger, RPPLogger
from .utils import remove_none


class OperationCode(Enum):
    """
    Operation code for the RPC.
    """

    HANDSHAKE = 0
    FRAME = 1
    CLOSE = 2


class ActivityType(Enum):
    """
    Available activity types for the RPC.
    """

    PLAYING = 0
    LISTENING = 2
    WATCHING = 3
    COMPETING = 5


class ClientRPC:
    """
    ClientRPC core class for Discord RPC.
    """

    def __init__(self, client_id: typing.Union[str, int] = None, debug: bool = False):
        """
        Initialize the RPC client.
        """
        self.log: RPPLogger = get_logger(self.__class__.__name__)
        self.__client_id: str = str(client_id)
        self.__socket: typing.BinaryIO = None
        self.__connected: bool = False
        self._pipe_connected: bool = True
        self.debug: bool = debug

        path = R"\\?\pipe\discord-ipc-{}"
        for index in range(10):
            path = path.format(index)
            try:
                # pylint: disable=consider-using-with
                self.__socket = open(path, "w+b")
            except FileNotFoundError:
                continue
            except OSError as e:
                self.log.error(e)
            else:
                if self.debug:
                    self.log.info("Connected to: %s", path)
                break
        else:
            self.log.error("Failed to connect. Possibly Discord is not running.")
            self._pipe_connected = False

    def __send(self, payload: dict, operation_code: OperationCode) -> None:
        """
        Send a payload to the RPC.

        Args:
            payload (dict): The payload to send.
            operation_code (OperationCode): The operation code.
        """
        payload = json.dumps(payload).encode("utf-8")
        new_payload = struct.pack("<ii", operation_code.value, len(payload)) + payload
        self.__socket.write(new_payload)
        self.__socket.flush()

    def __recv(self) -> dict:
        """
        Receive a payload from the response.
        """
        enc_header = b""
        header_size = 8

        while len(enc_header) < header_size:
            enc_header += self.__socket.read(header_size - len(enc_header))

        if len(enc_header) < header_size:
            self.log.error("Incomplete header received.")
            return {}

        dec_header = struct.unpack("<ii", enc_header)
        enc_data = b""
        remain_packet_size = dec_header[1]

        while len(enc_data) < remain_packet_size:
            enc_data += self.__socket.read(remain_packet_size - len(enc_data))

        if len(enc_data) < remain_packet_size:
            self.log.error("Incomplete data received.")
            return {}

        output = json.loads(enc_data.decode("UTF-8"))
        if self.debug:
            self.log.debug("Received: %s", output)
        return output

    def __handshake(self) -> None:
        """
        Handshake with the RPC.
        """
        if self.__connected:
            self.log.warning("Client already connected.")
            return
        self.__send({"v": 1, "client_id": self.__client_id}, OperationCode.HANDSHAKE)
        data = self.__recv()

        if "code" in data and data["code"] == 4000:
            raise ValueError(data["message"])

        if data["cmd"] == "DISPATCH" and data["evt"] == "READY":
            self.log.info("Handshake successful.")
            self.log.info("User: %s", data["data"]["user"]["username"])
            self.__connected = True
        else:
            self.log.error("Failed to handshake: %s", data)
            raise ValueError("Handshake failed.")

    # pylint: disable=R0913, R0917
    def update(
        self,
        state: str = None,
        details: str = None,
        activity_type: ActivityType = 0,
        start_time: int = None,
        end_time: int = None,
        large_image: str = None,
        large_text: str = None,
        small_image: str = None,
        small_text: str = None,
        buttons: list[dict] = None,
    ) -> None:
        """
        Update the activity.
        """
        if not self._pipe_connected:
            self.log.warning("Pipe not connected.")
            return
        if not self.__connected:
            self.log.warning("Client not connected.")
            return
        if not isinstance(activity_type, ActivityType):
            raise ValueError("Invalid activity type.")
        activity = {
            "state": state,
            "details": details,
            "type": activity_type.value,
            "timestamps": {"start": start_time, "end": end_time},
            "assets": {
                "large_image": large_image,
                "large_text": large_text,
                "small_image": small_image,
                "small_text": small_text,
            },
            "buttons": buttons,
        }
        pid = os.getpid()
        unique_id = str(uuid.uuid4())
        activity = remove_none(activity)
        payload = {
            "cmd": "SET_ACTIVITY",
            "args": {"pid": pid, "activity": activity},
            "nonce": unique_id,
        }
        self.__send(payload, OperationCode.FRAME)

    def close(self) -> None:
        """
        Close the connection.
        """
        if not self._pipe_connected:
            self.log.warning("Pipe not connected for closing.")
            return
        if not self.__connected:
            self.log.warning("Client not connected.")
            return
        self.__send({}, OperationCode.CLOSE)
        self.__socket.close()
        self.log.info("Connection closed.")
        self.__connected = False

    def connect(self) -> None:
        """
        Connect to the RPC.
        """
        if not self._pipe_connected:
            self.log.warning("Pipe not connected.")
            return
        self.__handshake()
