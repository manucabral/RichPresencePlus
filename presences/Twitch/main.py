import re
import time
import rpp

TWITCH_ICON = "https://assets.twitch.tv/assets/favicon-32-e29e246c157142c94346.png"


@rpp.extension
class Twitch(rpp.Presence):
    def __init__(self):
        super().__init__(metadata_file=True)
        self.activity_type: rpp.ActivityType = rpp.ActivityType.WATCHING
        self.tab: rpp.Tab = None

    def on_load(self):
        self.state = "No watching"
        self.details = "Initializing"
        self.start = int(time.time())
        self.small_image = TWITCH_ICON
        self.log.info("Loaded")

    def extract_stream_avatar(self):
        element = self.tab.execute(
            "document.querySelector('div[aria-label=\"Channel Avatar Picture\"] img')"
        )
        if not element.objectId:
            self.log.warning("Stream avatar not found")
            return None
        return self.tab.getProperties(element.objectId)

    def extract_stream_title(self):
        element = self.tab.execute(
            "document.querySelector('h2[data-a-target=\"stream-title\"]')"
        )
        if not element.objectId:
            self.log.warning("Stream title not found")
            return "No title"
        return self.tab.getProperties(element.objectId)

    def on_update(self, runtime: rpp.Runtime, **context):

        tabs = runtime.filter_tabs("www.twitch.tv")
        if not tabs:
            return

        last_tab = tabs[0]
        if self.tab is None or self.tab.url != last_tab.url:
            self.tab = last_tab
            self.log.info(f"Tab switched to {self.tab.url}")
            self.start = int(time.time())

        if not self.tab.connected:
            self.tab.connect()

        if not re.match(r"^https://www.twitch.tv/\w+$", self.tab.url):
            self.state = "Browsing Twitch"
            self.details = None
            self.large_image = TWITCH_ICON
            return

        title = self.extract_stream_title().textContent.value
        avatar = self.extract_stream_avatar().src.value
        channel_name = self.tab.url.split("/")[-1]
        self.state = title
        self.details = "Watching " + channel_name
        self.large_image = avatar
        self.large_text = channel_name
        self.small_text = "Twitch"
        self.buttons = [
            {
                "label": "Watch Stream",
                "url": self.tab.url,
            },
        ]

    def on_close(self):
        self.tab = None
        self.log.info("Closed")
