import time
from rpp import extension, Presence, Runtime, Tab


@extension
class Youtube(Presence):

    def __init__(self):
        super().__init__(metadataFile=True)
        self.thumbnail = "https://i.ytimg.com/vi/{videoId}/hqdefault.jpg"
        self.logo = "https://cdn3.iconfinder.com/data/icons/social-network-30/512/social-06-1024.png"

    def extractYoutubeTabs(self, tabs: list[Tab]) -> list[Tab]:
        return [
            tab
            for tab in tabs
            if "www.youtube.com/watch" in tab.url or "www.youtube.com/shorts" in tab.url
        ]

    def extractVideoTitle(self, tab: Tab) -> str:
        return (
            tab.execute(
                "document.querySelector('#title > h1 > yt-formatted-string').textContent"
            )
            or "Unknown"
        )

    def extractVideoId(self, tab: Tab) -> str:
        if "v=" in tab.url:
            videoId = tab.url.split("v=")[1]
            return videoId.split("&")[0] if "&" in videoId else videoId
        return "Unknown"

    def extractShortId(self, tab: Tab) -> str:
        return tab.url.split("/shorts/")[1]

    def extractVideoAuthor(self, tab: Tab) -> str:
        return (
            tab.execute('document.querySelector("#owner #text").textContent')
            or "Unknown"
        )

    def extractVideoAuthorUrl(self, tab: Tab) -> str:
        return tab.execute('document.querySelector("#owner #text > a").href')

    def extractThumbnail(self, videoId: str) -> str:
        return self.thumbnail.format(videoId=videoId)

    def extractVideoPlayback(self, tab: Tab) -> str:
        return tab.execute("navigator.mediaSession.playbackState")

    def extractShortAuthor(self, tab: Tab) -> str:
        return tab.execute(
            'document.querySelectorAll("#channel-info #channel-name a")[document.querySelectorAll("#channel-info #channel-name a").length - 1].textContent'
        )

    def handleShort(self, tab: Tab):

        shortId = self.extractShortId(tab)
        shortThumbnail = self.extractThumbnail(shortId)
        shortAuthor = self.extractShortAuthor(tab)
        shortAuthorUrl = "https://www.youtube.com/" + shortAuthor

        self.state = "Watching YouTube Short"
        self.details = "By " + shortAuthor
        self.large_image = shortThumbnail

        self.buttons = [
            {
                "label": "Watch Short",
                "url": tab.url,
            },
            {
                "label": "Author",
                "url": shortAuthorUrl,
            },
        ]
        self.log.debug(f"Updating short: {shortId}, url: {tab.url}")

    def handleVideo(self, tab: Tab):

        videoId = self.extractVideoId(tab)
        videoTitle = self.extractVideoTitle(tab)
        videoAuthor = self.extractVideoAuthor(tab)
        videoAuthorUrl = self.extractVideoAuthorUrl(tab)
        videoThumbnail = self.extractThumbnail(videoId)
        videoPlayback = self.extractVideoPlayback(tab)

        self.small_text = "Playing" if videoPlayback == "playing" else "Paused"
        self.small_image = "play" if videoPlayback == "playing" else "pause"

        self.details = "By " + videoAuthor
        self.state = videoTitle
        self.large_image = videoThumbnail

        self.buttons = [
            {
                "label": "Watch on YouTube",
                "url": tab.url,
            },
            {
                "label": "Author",
                "url": videoAuthorUrl,
            },
        ]
        self.log.debug(f"Updating video: {videoTitle}, url: {tab.url}")

    def handleBrowsing(self):
        self.state = "Browsing..."
        self.details = "Idle"
        self.large_image = self.logo

    def on_load(self):
        self.state = "Watching YouTube"
        self.details = "Idle"
        self.large_image = self.logo
        self.log.info("Started successfully")

    def on_update(self, runtime: Runtime):
        tabs = runtime.tabs()
        tabs = self.extractYoutubeTabs(tabs)
        if not tabs:
            self.log.info("No YouTube tabs found")
            return
        lastTab = tabs[0]

        if "/shorts/" in lastTab.url:
            self.handleShort(lastTab)
        elif "/watch?" in lastTab.url:
            self.handleVideo(lastTab)
        else:
            self.handleBrowsing()

        time.sleep(1)

    def on_close(self):
        self.log.info("Closed")
