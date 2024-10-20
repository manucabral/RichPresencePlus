"""
Core application GUI.
Contains the main application class.
"""

import sys
import customtkinter
import CTkMessagebox as CTkMb
import rpp
import scrollable_frame


class App(customtkinter.CTk):
    """
    Main application.
    """

    def __init__(
        self, manager: rpp.Manager, runtime: rpp.Runtime, browser: rpp.Browser
    ):
        """
        Initialize the application.

        """
        super().__init__()

        self.manager: rpp.Manager = manager
        self.runtime: rpp.Runtime = runtime
        self.browser: rpp.Browser = browser
        self.log: rpp.RPPLogger = rpp.get_logger("Interface")
        self.presences: list[rpp.Presence] = []

        self.presences_window: customtkinter.CTkToplevel = None
        self.search_entry: customtkinter.CTkEntry = None
        self.presences_frame: customtkinter.CTkFrame = None

        self.configure_window()
        self.configure_grid()
        self.configure_presences_window()

        self.scrollable_frame = scrollable_frame.ScrollableFrame(
            self, command=self.on_switch_change
        )
        self.scrollable_frame.grid(row=0, column=0, sticky="nsew")

        self.configure_frame_buttons()

        self.runtime_label = customtkinter.CTkLabel(
            master=self, text="Loading...", font=("Arial", 16)
        )
        self.runtime_label.grid(row=1, column=0, sticky="ew", pady=(0, 10))
        self.update_runtime_label()

        self.version_lb = customtkinter.CTkLabel(
            master=self, text=f"Using v{rpp.__version__}"
        )
        self.version_lb.grid(row=1, column=1, sticky="ew", pady=(0, 10))

        self.load_presences()

    def configure_window(self):
        """
        Configure the window.
        """
        self.title(rpp.__title__)
        self.geometry("400x300")
        self.resizable(False, False)
        self.iconbitmap("logo.ico")
        customtkinter.set_appearance_mode("System")
        customtkinter.set_default_color_theme("green")

    def configure_presences_window(self):
        """
        Configure the presences window.
        """
        self.presences_window = customtkinter.CTkToplevel(self)
        self.presences_window.title("Presences List")
        self.presences_window.geometry("300x400")
        self.presences_window.resizable(False, False)
        self.presences_window.iconbitmap("logo.ico")
        self.presences_window.withdraw()
        self.presences_window.protocol("WM_DELETE_WINDOW", self.on_hide_presences_list)
        label = customtkinter.CTkLabel(
            master=self.presences_window, text="Presences", font=("Arial", 18)
        )
        label.pack(pady=10)

        self.search_entry = customtkinter.CTkEntry(
            master=self.presences_window, placeholder_text="Search...", width=200
        )
        self.search_entry.pack(pady=5)

        frame = customtkinter.CTkFrame(self.presences_window)
        frame.pack(fill="both", expand=True, padx=10, pady=10)
        self.presences_frame = frame

    def configure_grid(self):
        """
        Configure the grid.
        """
        self.grid_rowconfigure(0, weight=1)
        self.grid_columnconfigure(0, weight=1)
        self.grid_columnconfigure(1, weight=1)
        self.grid_columnconfigure(2, weight=1)

    def configure_frame_buttons(self):
        """
        Configure the buttons.
        """
        buttons_frame = customtkinter.CTkFrame(self)
        buttons_frame.grid(row=0, column=1, sticky="ns")

        frame_title = customtkinter.CTkLabel(
            buttons_frame, text="Settings", font=("Arial", 20)
        )
        frame_title.grid(row=0, column=0, columnspan=2, sticky="ew", pady=(0, 10))

        # buttons
        self.reload_button = customtkinter.CTkButton(
            master=buttons_frame,
            text="Reload",
            command=self.on_reload_presences,
            font=("Arial", 16),
        )
        self.reload_button.grid(row=2, column=0, sticky="ew", pady=(0, 10))

        # browser buttons
        self.open_browser_button = customtkinter.CTkButton(
            master=buttons_frame,
            text="Open browser",
            command=self.on_browser_open,
            font=("Arial", 16),
        )
        self.open_browser_button.grid(row=3, column=0, sticky="ew", pady=(0, 10))

        self.close_browser_button = customtkinter.CTkButton(
            master=buttons_frame,
            text="Close browser",
            command=self.on_browser_close,
            font=("Arial", 16),
        )
        self.close_browser_button.grid(row=4, column=0, sticky="ew", pady=(0, 10))

        # runtime
        self.runtime_button = customtkinter.CTkButton(
            master=buttons_frame,
            text="Connect",
            command=self.on_runtime_connect,
            font=("Arial", 16),
        )
        self.runtime_button.grid(row=5, column=0, sticky="ew", pady=(0, 10))

        # white space
        customtkinter.CTkLabel(master=buttons_frame, text="").grid(
            row=6, column=0, sticky="ew", pady=(0, 10)
        )

        # presences
        self.presences_button = customtkinter.CTkButton(
            master=buttons_frame,
            text="Presences",
            command=self.on_open_presences_list,
            font=("Arial", 16),
        )
        self.presences_button.grid(row=7, column=0, sticky="ew", pady=(0, 0))

    def load_presences(self):
        """
        Load the presences.
        """
        self.log.info("Loading presences...")
        self.manager.load()
        self.manager.compare()
        for presence in self.manager.presences:
            if presence.enabled:
                self.scrollable_frame.add_item(presence.name)

    def on_reload_presences(self):
        """
        Handle presence reload.
        """
        for presence in self.manager.presences:
            if presence.running:
                message = CTkMb.CTkMessagebox(
                    icon="warning",
                    title="Presence running",
                    message=f"{presence.name} is running. Please stop it first.",
                    option_1="OK",
                )
                message.get()
                return
        self.manager.presences.clear()
        self.scrollable_frame.clear()
        self.load_presences()
        message = CTkMb.CTkMessagebox(
            icon="info",
            title="Presences reloaded",
            message="Presences reloaded successfully.",
            option_1="OK",
        )

    def on_browser_open(self):
        """
        Handle browser open.
        """
        if self.browser.running():
            message = CTkMb.CTkMessagebox(
                icon="warning",
                title="Browser already running",
                message=f"{self.browser.name} ({self.browser.process}) is already running.",
                option_1="OK",
            )
            message.get()
            return
        self.browser.start()
        message = CTkMb.CTkMessagebox(
            icon="info",
            title="Browser opened",
            message=f"{self.browser.name} ({self.browser.process}) opened.",
            option_1="OK",
        )
        message.get()
        self.update_runtime_label()

    def on_browser_close(self):
        """
        Handle browser close.
        """
        if not self.browser.running():
            message = CTkMb.CTkMessagebox(
                icon="warning",
                title="Browser not running",
                message=f"{self.browser.name} ({self.browser.process}) is not running.",
                option_1="OK",
            )
            message.get()
            return
        self.browser.kill()
        message = CTkMb.CTkMessagebox(
            icon="info",
            title="Browser closed",
            message=f"{self.browser.name} ({self.browser.process}) closed.",
            option_1="OK",
        )
        message.get()
        self.update_runtime_label()

    def on_runtime_connect(self):
        """
        Handle runtime connect.
        """
        if self.runtime.connected:
            message = CTkMb.CTkMessagebox(
                icon="warning",
                title="Runtime already connected",
                message=f"Runtime is already connected to port {self.runtime.port}.",
                option_1="OK",
            )
            message.get()
            return

        self.runtime.update()
        self.update_runtime_label()
        if not self.runtime.connected:
            message = CTkMb.CTkMessagebox(
                icon="warning",
                title="Runtime not connected",
                message="Cannot connect to the browser.",
                option_1="OK",
            )
            message.get()
            return

    def update_runtime_label(self):
        """
        Update the runtime label.
        """
        self.runtime.update()
        temp = f"{self.browser.name} browser {'connected' if self.runtime.connected else 'disconnected'}"
        self.runtime_label.configure(text=temp)
        self.manager.run_runtime()

    def add_presence_to_list(self, i: int, presence: dict):
        """
        Add a presence to the list.
        """
        name_label = customtkinter.CTkLabel(
            master=self.presences_frame, text=presence["name"], font=("Arial", 14)
        )
        name_label.grid(row=i, column=0, padx=10, pady=5, sticky="w")

        if not presence["installed"]:
            download_button = customtkinter.CTkButton(
                master=self.presences_frame,
                text="Download",
                command=lambda p=presence: self.on_download_presence(p),
                fg_color="blue",
                width=80,
            )
            download_button.grid(row=i, column=1, padx=10, pady=5)
        else:
            delete_button = customtkinter.CTkButton(
                master=self.presences_frame,
                text="Delete",
                command=lambda p=presence: self.on_delete_presence(p),
                fg_color="red",
                width=80,
            )
            delete_button.grid(row=i, column=1, padx=10, pady=5)

    def on_open_presences_list(self):
        """
        Handle presences list open.
        """
        self.presences_window.deiconify()
        self.presences_window.lift()

        self.presences_window.grab_set()
        self.search_entry.bind("<KeyRelease>", self.on_search)
        self.presences = self.manager.sync_presences()
        for i, presence in enumerate(self.presences):
            self.add_presence_to_list(i, presence)

    def on_hide_presences_list(self):
        """
        Handle presences list hide.
        """
        self.presences_window.withdraw()
        self.presences_window.grab_release()

    def on_search(self, event):
        """
        Handle search.
        """
        search_text = self.search_entry.get().lower()
        for widget in self.presences_frame.winfo_children():
            widget.destroy()
        for i, presence in enumerate(self.presences):
            if search_text in presence["name"].lower():
                self.add_presence_to_list(i, presence)

    def on_delete_presence(self, presence: dict):
        """
        Handle presence deletion.
        """
        self.manager.remove_presence(presence["name"])
        self.on_reload_presences()
        self.on_hide_presences_list()
        self.log.info(f"{presence['name']} deleted.")

    def on_download_presence(self, presence: dict):
        """
        Handle presence download.
        """
        self.manager.download_presence(presence["name"])
        self.on_reload_presences()
        self.on_hide_presences_list()
        self.log.info(f"{presence['name']} downloaded.")

    def on_switch_change(self, switch_name: str, state: bool):
        """
        Handle switch change.
        """
        target = None
        for presence in self.manager.presences:
            if presence.name == switch_name:
                target = presence
                break
        if target is None:
            self.log.info(f"Presence {switch_name} not found.")
            return
        if state:
            target.prepare()
            if not self.runtime.connected and target.web:
                message = CTkMb.CTkMessagebox(
                    icon="warning",
                    title="Connection required",
                    message=f"{target.name} requires connection to the browser. Please connect first.",
                    option_1="OK",
                )
                message.get()
                self.scrollable_frame.change_switch_state(target.name, False)
                return
            self.manager.run_presence(target)
            target.running = True
            return
        target.running = False
        target.on_close()
        self.log.info(f"Presence {target.name} closed.")

    def on_exit(self) -> None:
        """
        Called when the app is closed.
        """
        for presence in self.manager.presences:
            if presence.running:
                message = CTkMb.CTkMessagebox(
                    icon="warning",
                    title="Presence running",
                    message=f"{presence.name} is running. Please stop it first.",
                    option_1="OK",
                )
                message.get()
                return
        message = CTkMb.CTkMessagebox(
            icon="question",
            title="Exit",
            message="Are you sure you want to exit?",
            option_1="Yes",
            option_2="No",
        )

        result: str = message.get()
        if result is None:
            return
        if result.lower() == "no":
            return
        self.manager.stop_event.set()
        try:
            self.manager.runtime.running = False
            self.manager.executor.shutdown()
        except Exception as exc:
            self.log.error(f"Error stopping: {exc}")
        self.destroy()
        self.quit()
        sys.exit(0)
