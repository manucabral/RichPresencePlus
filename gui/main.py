"""
Beta version of the GUI. 
"""
import os
import yaml
import rpp
import customtkinter
import CTkMessagebox as CTkMb


# need to be moved to rpp.utils
def connect_runtime() -> rpp.Runtime:
    """
    Connect to the browser.
    """
    runtime = rpp.Runtime()
    res = runtime.connect()
    if not res:
        CTkMb.CTkMessagebox(
            icon="warning",
            title="Browser connection",
            message="Unable to connect to the browser. Please, make sure that you have the browser closed completely to use web presences.",
            option_1="Ok",
        )
    return runtime


# need to be moved to rpp.utils
def load_local_presences(uses_browser: bool = True) -> list:
    presences = []
    for file in rpp.utils.list_dir("presences"):
        subdir = os.path.join("presences", file)
        if not rpp.utils.is_dir(subdir):
            continue
        files = rpp.utils.list_dir(subdir)
        if "metadata.yml" not in files or "main.py" not in files:
            rpp.log(
                f"Skipping {file} because it is not a valid presence.", level="WARNING"
            )
            continue
        with open(os.path.join(subdir, "metadata.yml")) as _file:
            metadata = yaml.safe_load(_file)
        metadata["path"] = subdir
        presence = rpp.Presence(**metadata)
        if presence.metadata["use_browser"] and not uses_browser:
            presence.enabled = False
            rpp.log(
                f"Skipping {presence.metadata['name']} because the browser is not running.",
                level="WARNING",
            )
            continue
        presences.append(presence)

    rpp.log(f"Loaded {len(presences)} presences.")
    return presences


# TODO: need to be moved to rpp.utils
def validate_local_presences() -> list:
    pass


class ScrollableSwitchFrame(customtkinter.CTkScrollableFrame):
    """
    Frame with a scrollable area and switches.
    """

    def __init__(self, master, command=None, **kwargs):
        """
        Initialize the frame.
        """
        super().__init__(master, **kwargs)

        self.__command = command
        self.grid_columnconfigure(0, weight=1)
        self.grid_columnconfigure(1, weight=1)
        title_lb = customtkinter.CTkLabel(
            master=self, text="Presences", font=("Arial", 20)
        )
        title_lb.grid(row=0, column=0, sticky="ew", pady=(0, 10))
        self.__switches = []

    def add_item(self, item: str):
        """
        Add a switch to the frame.
        """
        switch = customtkinter.CTkSwitch(master=self, text=item)
        if self.__command is not None:
            switch.configure(command=lambda: self.__command(item, switch.get()))
        switch.grid(row=2 + len(self.__switches), column=0, pady=(0, 10))
        self.__switches.append(switch)

    def clear(self):
        """
        Clear all switches.
        """
        for switch in self.__switches:
            switch.destroy()
        self.__switches.clear()

    def get_switches_states(self):
        return [switch.get() for switch in self.__switches]

    def get_items(self):
        return [switch.text for switch in self.__switches]


class App(customtkinter.CTk):
    """
    Main app.
    """

    __slots__ = ("__runtime", "__presences", "scrollable_frame", "reload_btn")

    def __init__(self):
        super().__init__()
        self.__configure_window()
        self.__configure_grid()
        self.__presences = []
        self.__runtime = connect_runtime()
        self.scrollable_frame = ScrollableSwitchFrame(
            master=self,
            command=self.on_switch_change,
        )
        self.scrollable_frame.grid(row=0, column=0, sticky="ns")
        self.__configure_buttons_frame()

        self.runtime_lb = customtkinter.CTkLabel(master=self, text="Hello World")
        self.runtime_lb.grid(row=1, column=0, sticky="ew", pady=(0, 10))
        self.__update_runtime_label()

        self.version_lb = customtkinter.CTkLabel(
            master=self, text=f"v{rpp.__version__}"
        )
        self.version_lb.grid(row=1, column=1, sticky="ew", pady=(0, 10))

        self.load_presences()

    def __configure_window(self):
        """
        Configure the window.
        """
        self.title(rpp.__title__)
        self.geometry("400x300")
        customtkinter.set_appearance_mode("dark")
        # customtkinter.set_default_color_theme("blue")

    def __configure_grid(self):
        self.grid_rowconfigure(0, weight=1)
        self.grid_columnconfigure(0, weight=1)
        self.grid_columnconfigure(1, weight=1)

    def __configure_buttons_frame(self):
        buttons_frame = customtkinter.CTkFrame(master=self)
        buttons_frame.grid(row=0, column=1, sticky="ns")

        config_lb = customtkinter.CTkLabel(
            master=buttons_frame, text="Configuration", font=("Arial", 16)
        )
        config_lb.grid(row=0, column=0, columnspan=2, sticky="ew", pady=(0, 10))

        # buttons
        self.reload_btn = customtkinter.CTkButton(
            master=buttons_frame, text="Reload presences", command=self.reload_presences
        )
        self.reload_btn.grid(row=2, column=0, sticky="ew", pady=(0, 10))

        text = (
            "Connect to browser"
            if not self.__runtime.connected
            else "Reconnect to browser"
        )
        self.runtime_btn = customtkinter.CTkButton(
            master=buttons_frame, text=text, command=self.on_runtime_connect
        )
        self.runtime_btn.grid(row=3, column=0, sticky="ew", pady=(0, 10))

        """
        self.exit_btn = customtkinter.CTkButton(master=buttons_frame, text="Exit")
        self.exit_btn.grid(row=4, column=0, sticky="ew")
        """

    def __update_runtime_label(self):
        """
        Update the runtime label.
        """
        text = "Browser"
        text += " connected" if self.__runtime.connected else " disconnected"
        self.runtime_lb.configure(text=text)
        self.runtime_btn.configure(
            text="Reconnect to browser"
            if self.__runtime.connected
            else "Connect to browser"
        )

    def reload_presences(self):
        """
        Reload presences.
        """
        for presence in self.__presences:
            if presence.enabled:
                msg = f"Please, disable {presence.metadata['name']} before reloading."
                CTkMb.CTkMessagebox(
                    icon="warning",
                    title="Reload presences",
                    message=msg,
                    option_1="Ok",
                )
                return
        self.__presences.clear()
        self.scrollable_frame.clear()
        self.reload_btn.configure(state="disabled")
        self.load_presences()
        self.reload_btn.configure(state="normal")
        CTkMb.CTkMessagebox(
            icon="info",
            title="Reload presences",
            message="Presences reloaded successfully.",
            option_1="Ok",
        )

    def load_presences(self):
        """
        Reload presences.
        """
        self.__presences = load_local_presences(self.__runtime.connected)
        for presence in self.__presences:
            if self.__runtime.connected and presence.metadata["use_browser"]:
                presence.runtime = self.__runtime
            self.scrollable_frame.add_item(presence.name)

    def get_presence(self, name: str) -> rpp.Presence:
        """
        Return a presence by its name.
        """
        for presence in self.__presences:
            if presence.name == name:
                return presence
        return None

    def on_runtime_connect(self):
        tab = self.__runtime.current_tab
        print(tab)
        if tab is None:
            self.__runtime = connect_runtime()
            self.__update_runtime_label()
            if self.__runtime.connected:
                CTkMb.CTkMessagebox(
                    icon="info",
                    title="Browser connection",
                    message="The browser has been connected successfully. Please, reload the presences.",
                    option_1="Ok",
                )
        else:
            CTkMb.CTkMessagebox(
                icon="info",
                title="Browser connection",
                message="The browser is already connected.",
                option_1="Ok",
            )

    def on_switch_change(self, name: str, state: bool):
        """
        Called when a switch changes its state.
        """
        target = None
        for presence in self.__presences:
            if presence.name == name:
                target = presence
        if target is None:
            rpp.log(f"Presence {name} not found.", level="WARNING")
            return
        if state:
            target.enabled = True
            target.start()
            target.connect()
            return
        rpp.log(f"Stopping {name}...")
        target.stop()
        target.enabled = False
        print(target.enabled)

    def on_exit(self):
        """
        Called when the app is closed.
        """
        result = CTkMb.CTkMessagebox(
            icon="question",
            title="Exit",
            message="Are you sure you want to exit?",
            option_1="Yes",
            option_2="No",
        )
        if result == "No":
            return
        for presence in self.__presences:
            if not presence.enabled:
                continue
            presence.stop()
        self.destroy()


if __name__ == "__main__":
    app = App()
    app.protocol("WM_DELETE_WINDOW", app.on_exit)
    app.mainloop()
