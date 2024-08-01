import time
import rpp


@rpp.extension
class Youtube(rpp.Presence):

    def __init__(self):
        self.name = "Youtube"
        self.version = "0.0.1"
        self.clientId = 1113646725408772176
        self.usingWeb = True

    def on_load(self):
        self.state = "Idle"
        self.details = "Idle"
        print("Started Youtube Presence")

    def on_update(self, context: rpp.Runtime):

        # Get all tabs of the browser
        tabs = context.tabs()
        youtubeTab = None

        # Find last opened youtube tab
        for tab in tabs:
            if "youtube.com" in tab.url:
                youtubeTab = tab
                break

        # If youtube tab is not found, set presence to idle
        if youtubeTab is None:
            self.state = "Idle"
            self.details = "Idle"
            return

        # Set presence to watching youtube
        self.state = youtubeTab.execute(
            "document.querySelector('#title > h1 > yt-formatted-string').textContent"
        )
        self.details = "Watching Youtube"
        self.buttons = [{"label": "Open Youtube", "url": youtubeTab.url}]

        time.sleep(1)

    def on_close(self):
        print("Stopped Youtube Presence")
