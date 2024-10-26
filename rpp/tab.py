"""
Tab module. 
Contains each element to interact with a tab from the browser.
"""

import json
from .client import Client
from .logger import get_logger

# pylint: disable=C0103


class Attribute:
    """
    Attribute object.

    Args:
        name (str): The name of the attribute.
        type (str): The type of the attribute.
        value (str): The value of the attribute.
    """

    def __init__(self, **kwargs: dict):
        """
        Initialize the attribute object.
        """
        self.name = kwargs.get("name", None)
        self.type = kwargs.get("type", None)
        self.value = kwargs.get("value", None)

    def __str__(self) -> str:
        return f"Attribute<name={self.name}, type={self.type}, value={self.value}>"

    def __repr__(self) -> str:
        return self.__str__()


class PropertiesResponse:
    """
    Dynamic properties response object.

    All attributes are a instance of Attribute class.
    """

    def __init__(self, **kwargs: dict) -> None:
        """
        Initialize the properties response object.
        """
        self.__dict__.update(kwargs)

    def __str__(self) -> str:
        return f"PropertiesResponse<{self.__dict__}>"

    def __repr__(self) -> str:
        return self.__str__()

    def __getitem__(self, key: str) -> Attribute:
        return getattr(self, key)

    def keys(self) -> list:
        """
        Get the keys of the properties.
        """
        return self.__dict__.keys()

    def __contains__(self, key: str) -> bool:
        return key in self.__dict__

    # ON GET KEY
    def __getattr__(self, key: str) -> Attribute:
        return getattr(self, key)


class RemoteObject:
    """
    Remote object class.
    See: https://chromedevtools.github.io/devtools-protocol/tot/Runtime#type-RemoteObject
    """

    def __init__(self, **kwargs: dict):
        """
        Initialize the remote object.
        """
        self.type = kwargs.get("type", None)
        self.subtype = kwargs.get("subtype", None)
        self.className = kwargs.get("className", None)
        self.value = kwargs.get("value", None)
        self.unserializableValue = kwargs.get("unserializableValue", None)
        self.description = kwargs.get("description", None)
        self.objectId = kwargs.get("objectId", None)

    def __str__(self) -> str:
        return f"RemoteObject<{self.type}, {self.objectId}, {self.value}>"

    def __repr__(self) -> str:
        return self.__str__()

    def __eq__(self, other) -> bool:
        return self.objectId == other.objectId

    def __ne__(self, other) -> bool:
        return self.objectId != other.objectId


class Tab:
    """
    Tab class.
    Interacts with a tab from the browser.
    """

    def __init__(self, **kwargs: dict) -> None:
        self.__id = kwargs.get("id", None)
        self.__url = kwargs.get("url", "about:blank")
        self.__title = kwargs.get("title", "Unknown")
        self.__ws_debug_url = kwargs.get("webSocketDebuggerUrl", None)
        self.__client = Client(self.__ws_debug_url)
        self.__connected = False
        self.log = get_logger(f"Tab<{self.__id}>")

    def __str__(self) -> str:
        return f"Tab<{self.__id}, {self.__title}>"

    def __repr__(self) -> str:
        return self.__str__()

    def __eq__(self, other) -> bool:
        return self.__id == other.id

    def __ne__(self, other) -> bool:
        return self.__id != other.id

    @property
    def id(self) -> str:
        """
        Get the tab id.
        """
        return self.__id

    @property
    def url(self) -> str:
        """
        Get the tab url.
        """
        return self.__url

    @property
    def title(self) -> str:
        """
        Get the tab title.
        """
        return self.__title

    @property
    def connected(self) -> bool:
        """
        Get the connection status.
        """
        return self.__connected

    def getProperties(self, objectId: str) -> PropertiesResponse:
        """
        Get the properties of an remote object.

        Args:
            objectId (str): The object id.

        Returns:
            PropertiesResponse: The properties of the object.
        """
        if self.__connected is False:
            raise RuntimeError("Please connect to the tab before executing this method")
        self.__client.send(
            {
                "id": 2,
                "method": "Runtime.getProperties",
                "params": {"objectId": objectId},
            }
        )
        data = self.__client.receive()
        jsonData = json.loads(data)
        if "error" in jsonData:
            self.log.error("On get properties: %s", jsonData["error"]["message"])
            return PropertiesResponse()
        result = jsonData["result"]["result"]
        properties = {
            item["name"]: Attribute(name=item["name"], **item["value"])
            for item in result
            if "value" in item
        }
        return PropertiesResponse(**properties)

    def connect(self) -> None:
        """
        Connect to the tab.
        """
        if self.__connected:
            self.log.info("Already connected")
            return
        if self.__client.ws is None:
            self.__client.connect()
            self.__connected = True

    def close(self) -> None:
        """
        Close the tab.
        """
        if self.connected is True:
            self.__client.close()
            self.__connected = False
            self.log.info("Closed the websocket connection.")

    def execute(self, code: str) -> RemoteObject:
        """
        Execute javascript code in the tab.

        Args:
            code (str): The code to execute.

        Returns:
            str: The result of the code execution.
        """
        if self.__connected is False:
            raise RuntimeError("Please connect to the tab before executing code")
        try:
            self.__client.send(
                {
                    "id": 1,
                    "method": "Runtime.evaluate",
                    "params": {"expression": code},
                }
            )
            data = self.__client.receive()
            jsonData = json.loads(data)
            if "error" in jsonData:
                self.log.error("On execute: %s", jsonData["error"]["message"])
                return RemoteObject()
            return RemoteObject(**jsonData["result"]["result"])
        except Exception as exc:
            raise exc
