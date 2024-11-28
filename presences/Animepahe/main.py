import rpp
import time

IMAGE_URL = "https://raw.githubusercontent.com/manucabral/RichPresencePlus/refs/heads/main/presences/Animepahe/image.webp"

@rpp.extension
class Animepahe(rpp.Presence):
    def __init__(self):
        super().__init__(metadata_file=True)
        self.activity_type: rpp.ActivityType = rpp.ActivityType.WATCHING
        self.tab: rpp.Tab = None
        self.last_status: str = None 

    def on_load(self):
        self.state = "Initializing"
        self.details = "Idle"
        self.log.info("Loaded")

    def extract_tabs(self, runtime: rpp.Runtime) -> list[rpp.Tab]:
        tabs = runtime.filter_tabs("animepahe")
        available_domains = ["animepahe.ru", "animepahe.org", "animepahe.com"]
        return [tab for tab in tabs 
                if any(domain in tab.url for domain in available_domains)]

    def extract_metadata(self) -> str:
        element = self.tab.execute("document.querySelector('.theatre-info h1')")
        if not element.objectId:
            self.log.warning("Metadata not found")
            return None
        return self.tab.getProperties(element.objectId).textContent.value

    def extract_anime_image(self) -> str:
        element = self.tab.execute("document.querySelector('.theatre-info img')")
        if not element.objectId:
            self.log.warning("Anime image not found")
            return None
        return self.tab.getProperties(element.objectId).src.value

    def extract_video_frame(self) -> str:
        element = self.tab.execute("document.querySelector('#kwikPlayer')")
        if not element.objectId:
            self.log.warning("Video frame not found")
            return None
        return self.tab.getProperties(element.objectId).src.value

    def get_rpp_label(self) -> str:
        return f"{rpp.__title__} v{rpp.__version__}"

    def set_idle(self):
        if self.last_status != "idle":
            self.state = "Idle"
            self.details = "Nothing to show"
            self.large_text = self.get_rpp_label()
            self.large_image =  self.buttons = None
            self.end = self.start = None
            self.log.info("Idle")
            self.last_status = "idle"
            self.force_update()

    def set_viewing_home(self):
        if self.last_status != "viewing_home":
            self.state = "Home"
            self.details = "Last releases"
            self.large_text = self.get_rpp_label()
            self.large_image = self.buttons = None
            self.start = int(time.time())
            self.log.info("Viewing home")
            self.last_status = "viewing_home"
            self.force_update()
    
    def set_viewing_queue(self):
        if self.last_status != "viewing_queue":
            self.state = "Queue"
            self.details = "Watching queue"
            self.lage_image = ""
            self.large_text = self.get_rpp_label()
            self.buttons = None
            self.start = int(time.time())
            self.log.info("Viewing queue")
            self.last_status = "viewing_queue"
            self.force_update()

    def set_browsing_catalog(self):
        if self.last_status != "browsing_catalog":
            self.state = "Browsing..."
            self.details = "Searching in catalog"
            self.large_text = self.get_rpp_label()
            self.large_image =  self.buttons = None
            self.start = int(time.time())
            self.log.info("Browsing in catalog")
            self.last_status = "browsing_catalog"
            self.force_update()

    def set_watching_episode(self):
        metadata = self.extract_metadata()
        image = self.extract_anime_image()
        if metadata is None:
            self.log.warning("Episode metadata not found")
            return
    
        # parsing metadata
        title, episode = metadata.split(" - ")
        title = title.replace("Watch ", "").strip()
        episode = episode.split(" ")[0]
        
        self.details = title
        self.state = f"Episode {episode}"
        self.large_image = image
        self.large_text = title
        self.buttons = [
            {
                "label": "Watch",
                "url": self.tab.url
            }
        ]

        if self.last_status != metadata:
            self.start = int(time.time())
            self.last_status = metadata
            self.log.info("Watching %s episode %s", title, episode)
            self.force_update()

    def on_update(self, runtime: rpp.Runtime, **context):

        # Extract only animepahe tabs
        tabs = self.extract_tabs(runtime)

        if not tabs:
            # No animepahe tabs found
            self.set_idle()
            return

        last_tab = tabs[0]

        if self.tab is None:
            self.tab = last_tab
            self.log.info("Initial tab set to %s", self.tab.url)

        elif self.tab.url != last_tab.url:
            self.tab.close()
            self.tab = last_tab
            self.log.info("Tab changed to %s", self.tab.url)

        else:
            # Tab is the same, do nothing
            pass
            
        if not self.tab.connected:
            self.tab.connect()

        url_parts = self.tab.url.split("/")
        state = url_parts[3]

        if state == "":
            self.set_viewing_home()

        elif state == "queue":
            self.set_viewing_queue()

        elif state == "anime":
            self.set_browsing_catalog()
        
        elif state == "play":
            self.set_watching_episode()

    def on_close(self):
        self.title = self.tab = None
        self.log.info("Closed")
