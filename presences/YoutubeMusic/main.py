import rpp
import time


@rpp.extension
class YoutubeMusic(rpp.Presence):
    def __init__(self):
        super().__init__(metadata_file=True)
        self.activity_type: rpp.ActivityType = rpp.ActivityType.LISTENING
        self.last_song: str = None
        self.tab: rpp.Tab = None

    def on_load(self):
        self.log.info("Loaded")

    def extract_tabs(self, runtime: rpp.Runtime) -> list[rpp.Tab]:
        tabs = runtime.tabs()
        tabs = runtime.filter_tabs("music.youtube.com")
        return [tab for tab in tabs if "sw.js" not in tab.url]

    def extract_mediasession_metadata(self):
        element = self.tab.execute("navigator.mediaSession.metadata")
        if not element.objectId:
            self.log.warning("MediaSession metadata not found")
            return
        props = self.tab.getProperties(element.objectId)
        return {
            "title": props.title.value,
            "artist": props.artist.value,
            "album": props.album.value,
            "artwork": props.artwork.value,
        }

    def extract_mediasession_playblackstate(self):
        element = self.tab.execute("navigator.mediaSession")
        if not element.objectId:
            self.log.debug("Could not find playback state of MediaSession")
            return "unknown"
        props = self.tab.getProperties(element.objectId)
        return props.playbackState.value

    def extract_mediasession_artwork(self):
        element = self.tab.execute("navigator.mediaSession.metadata.artwork[0]")
        if not element.objectId:
            self.log.warning("MediaSession artwork not found")
            return "https://music.youtube.com/favicon.ico"
        props = self.tab.getProperties(element.objectId)
        return props.src.value

    def on_update(self, runtime: rpp.Runtime):

        tabs = self.extract_tabs(runtime)
        if not tabs:  # No tabs found
            return

        last_tab = tabs[0]
        if self.tab is None:  # Set initial tab
            self.tab = last_tab
        elif self.tab.url != last_tab.url:  # New tab detected
            self.tab = last_tab
        else:  # Tab not changed
            pass

        if not self.tab.connected:
            self.tab.connect()

        metadata = self.extract_mediasession_metadata()
        playback_state = self.extract_mediasession_playblackstate()
        artwork = self.extract_mediasession_artwork()

        if self.last_song != metadata["title"]:  # Song changed
            self.last_song = metadata["title"]
            self.log.info(
                f"Now playing {metadata['title']} by {metadata['artist']} at {metadata['album']}"
            )
            self.details = metadata["artist"]
            self.state = metadata["title"]
            self.large_image = artwork
            self.large_text = metadata["album"]
            self.small_image = "play" if playback_state == "playing" else "pause"
            self.small_text = (
                "Listening to music" if playback_state == "playing" else "Paused"
            )
            self.buttons = [
                {
                    "label": "Listen on Youtube Music",
                    "url": self.tab.url,
                }
            ]
        else:
            # song not changed, update time, playback state...
            self.small_image = "playing" if playback_state == "playing" else "pause"
            self.small_text = "Listening" if playback_state == "playing" else "Paused"

    def on_close(self):
        self.log.info("Closed")
