"""
Containts the Tab class which is used to interact with runtime.
"""
import json
import httpx_ws
from .media_session import MediaSession

# pylint: disable=broad-except


class Tab:
    """
    A tab object.
    """

    __slots__ = (
        "__id",
        "__url",
        "__title",
        "__ws_debug_url",
        "__fav_icon_url",
    )

    def __init__(self, **kwargs: dict) -> None:
        """
        Create a new Tab object.
        """
        self.__id = kwargs.get("id", None)
        self.__url = kwargs.get("url", "about:blank")
        self.__title = kwargs.get("title", "Unknown")
        self.__ws_debug_url = kwargs.get("webSocketDebuggerUrl", None)
        self.__fav_icon_url = kwargs.get("favIconUrl", None)

    def __eq__(self, other: "Tab") -> bool:
        if not isinstance(other, Tab):
            return False
        return self.__url == other.url

    def __str__(self):
        return f"<Tab url={self.url}>"

    def __repr__(self):
        return self.__str__()

    def __parse(self, data: str) -> str:
        """
        Parse the data received from the websocket.
        """
        result_type = data["result"]["result"]["type"]
        if result_type == "undefined":
            return None
        if result_type == "object":
            return data["result"]["result"]["objectId"]
        if result_type == "string":
            return data["result"]["result"]["value"]
        return None

    def __dump(self, exp: str) -> str:
        """
        Dump the expression to a json string.
        """
        return json.dumps(
            {
                "id": 1,
                "method": "Runtime.evaluate",
                "params": {"expression": exp},
            }
        )

    @property
    def id(self) -> str:
        """
        Return the tab id.
        """
        return self.__id

    @property
    def url(self) -> str:
        """
        Return the tab url.
        """
        return self.__url

    @property
    def title(self) -> str:
        """
        Return the tab title.
        """
        return self.__title

    @property
    def favicon(self) -> str:
        """
        Return the tab favicon url.
        """
        return self.__fav_icon_url

    def exec(self, code: str):
        """
        Execute a code in the tab.
        """
        try:
            with httpx_ws.connect_ws(self.__ws_debug_url) as ws:
                dumped = self.__dump(code)
                ws.send_text(dumped)
                data = ws.receive_text()
                return self.__parse(json.loads(data))
        except Exception as exc:
            raise exc

    def media_session(self) -> MediaSession:
        """
        Get the media session data from the tab.
        """
        # self.exec(f"navigator.mediaSession.{action}()")
        data = self.exec(
            "[navigator.mediaSession.playbackState,\
                    navigator.mediaSession.metadata.album || 'unknown',\
                    navigator.mediaSession.metadata.artist || 'unknown',\
                    navigator.mediaSession.metadata.artwork[0].src,\
                    navigator.mediaSession.metadata.title].join('@')"
        )
        if data is None:
            return None
        data = data.split("@")
        return MediaSession(
            **{
                "state": data[0],
                "album": data[1],
                "artist": data[2],
                "artwork": data[3],
                "title": data[4],
            }
        )
