import rpp
import time


@rpp.extension
class YoutubeMusic(rpp.Presence):
    def __init__(self):
        super().__init__(metadata_file=True)
        self.activity_type: rpp.ActivityType = rpp.ActivityType.LISTENING
        self.last_song: str = None
        self.last_status: str = "unknown" 
        self.duration: int = 0
        self.tab: rpp.Tab = None

    def on_load(self):
        self.state = "Initializing"
        self.details = "Idle"
        self.log.info("Loaded")

    def extract_tabs(self, runtime: rpp.Runtime) -> list[rpp.Tab]:
        tabs = runtime.tabs()
        tabs = runtime.filter_tabs("music.youtube.com")
        return [tab for tab in tabs if "sw.js" not in tab.url]

    def extract_video_stream(self):
        element = self.tab.execute(
            'Array.from(document.querySelectorAll(".video-stream")).find(element => element.duration)'
        )
        if not element.objectId:
            self.log.warning("Video stream not found")
            return None
        return self.tab.getProperties(element.objectId)

    def extract_mediasession_metadata(self):
        element = self.tab.execute("navigator.mediaSession.metadata")
        if not element.objectId:
            self.log.warning("MediaSession metadata not found")
            return None
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

    def extract_extra_metadata(self):
        """
        Simply extracts the year or likes if available.
        """
        element = self.tab.execute(
            "document.querySelector('yt-formatted-string.byline.style-scope.ytmusic-player-bar span:last-of-type')"
        )
        if not element.objectId:
            self.log.debug("Extra metadata not found")
            return None
        return self.tab.getProperties(element.objectId).textContent.value

    def set_idle(self):
        if self.last_status is not None:
            self.last_status = None
            self.state = "No listening..."
            self.details = "Idle"
            self.large_image = self.small_image = None
            self.large_text = self.small_text = None
            self.end = None
            self.log.info("No listening activity detected")
            self.force_update()

    def on_update(self, runtime: rpp.Runtime, **context):
        
        tabs = self.extract_tabs(runtime)
        if not tabs:  # No tabs found}
            self.set_idle()
            return

        last_tab = tabs[0]
        if self.tab is None:  # Set initial tab
            self.tab = last_tab
        elif self.tab.url != last_tab.url:  # New tab detected
            self.tab = last_tab
            self.log.info(f"Tab changed to {self.tab.url}")
        else:  # Tab not changed
            pass
        
        if not self.tab.connected:
            self.tab.connect()
        
        metadata = self.extract_mediasession_metadata()
        if not metadata:
            return
        
        playback_state = self.extract_mediasession_playblackstate()
        artwork = self.extract_mediasession_artwork()
        video_stream = self.extract_video_stream()
        extra = self.extract_extra_metadata()
    
        def update_time():
            self.small_image = "playing" if playback_state == "playing" else "pause"
            self.small_text = "Listening" if playback_state == "playing" else "Paused"
            if video_stream:
                if playback_state == "playing":
                    self.duration = int(video_stream.duration.value)
                    self.start = int(time.time()) - int(video_stream.currentTime.value)
                    self.end = self.start + self.duration
                else:
                    self.end = None
            
        if self.last_song != metadata["title"]:  # Song changed
            self.log.info(
                f"Now playing {metadata['title']} by {metadata['artist']} at {metadata['album']}"
            )
            self.last_song = metadata["title"]
            self.details = metadata["artist"]
            self.state = metadata["title"]
            self.large_image = artwork
            if metadata["album"] == metadata["title"]:
                metadata["album"] = None
            self.large_text = metadata["album"] if metadata["album"] else extra
            self.small_image = "play" if playback_state == "playing" else "pause"
            self.last_status = playback_state
            self.small_text = (
                "Listening to music" if playback_state == "playing" else "Paused"
            )
            self.buttons = [{ "label": "Listen on Youtube Music", "url": self.tab.url }]
            if video_stream:
                self.duration = int(video_stream.duration.value)
            update_time()
            self.force_update()
        else:
            # Update time and playback state
            update_time()
            if self.last_status != playback_state:
                self.last_status = playback_state
                self.log.info(f"Playback state changed to {playback_state}")
                self.force_update()

    def on_close(self):
        self.last_song = None
        self.tab = None
        self.log.info("Closed")
