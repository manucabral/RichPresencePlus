"""
Easy-to-use GUI for RichPresencePlus.
"""

import customtkinter
import CTkMessagebox as CTkMb
import rpp


class ScrollableFrame(customtkinter.CTkScrollableFrame):
    """
    Scrollable frame.
    """

    def __init__(self, master, command=None, **kwargs):
        """
        Initialize the scrollable frame.
        """
        super().__init__(master, **kwargs)
        self.command = command
        self.switches: list[customtkinter.CTkSwitch] = []

        self.configure_grid()

        self.label = customtkinter.CTkLabel(self, text="Presences", font=("Arial", 20))
        self.label.grid(row=0, column=0, sticky="ew", pady=(0, 10))
    
    def configure_grid(self):
        """
        Configure the grid.
        """
        self.grid_columnconfigure(0, weight=1)
        self.grid_columnconfigure(1, weight=1)

    def add_item(self, item: str):
        """
        Add a presence to the list.
        """
        switch = customtkinter.CTkSwitch(master=self, text=item, font=("Arial", 16))
        if self.command is not None:
            switch.configure(command=lambda: self.command(item, switch.get()))
        switch.grid(row=2 + len(self.switches), column=0, pady=(0, 10))
        self.switches.append(switch)

    def clear(self):
        """
        Clear all switches.
        """
        for switch in self.switches:
            switch.destroy()
        self.switches.clear()

    def change_switch_state(self, item: str, state: bool):
        """
        Change the state of a switch.
        """
        for switch in self.switches:
            if switch.cget("text") == item:
                switch.select() if state else switch.deselect()
                break

    def get_switches_states(self):
        """
        Get the states of all switches.
        """
        return [switch.get() for switch in self.switches]

    def get_items(self):
        """
        Get all items.
        """
        return [switch.text for switch in self.switches]


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

        self.configure_window()
        self.configure_grid()

        self.scrollable_frame = ScrollableFrame(self, command=self.on_switch_change)
        self.scrollable_frame.grid(row=0, column=0, sticky="nsew")

        self.configure_frame_buttons()


        self.runtime_label = customtkinter.CTkLabel(master=self, text="Loading...", font=("Arial", 16))
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
            master=buttons_frame, text="Reload", command=self.on_reload_presences, font=("Arial", 16)
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

    def load_presences(self):
        """
        Load the presences.
        """
        self.log.info("Loading presences...")
        self.manager.load()
        for presence in self.manager.presences:
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
        temp = f"{browser.name} browser {'connected' if self.runtime.connected else 'disconnected'}"
        self.runtime_label.configure(text=temp)
        self.manager.run_runtime()

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
            manager.log.info(f"Presence {switch_name} not found.")
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
            target.running = True
            self.manager.run_presence(target)
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
        if result.lower() == "no":
            return
        self.manager.stop_event.set()
        self.destroy()


if __name__ == "__main__":
    browser = rpp.Browser()
    runtime = rpp.Runtime(9222)
    manager = rpp.Manager(runtime=runtime, dev_mode=False)
    manager.run_main()
    app = App(manager, runtime, browser)
    app.protocol("WM_DELETE_WINDOW", app.on_exit)
    app.mainloop()
