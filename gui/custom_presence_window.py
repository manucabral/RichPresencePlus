"""
Custom Presence Window
----------------------
This module contains the CustomPresenceWindow class, 
which is a custom tkinter toplevel window used in the main application.
It allows the user to set a custom presence for their Discord account.
"""

import os
import time
import pickle
import customtkinter
import CTkMessagebox as CTkMb
import rpp


# pylint: disable=R0902, R0913, R0917
class Profile:
    """
    Profile class.
    """

    def __init__(
        self,
        name: str = "Default",
        app_id: str = None,
        details: str = None,
        state: str = None,
        activity_type: rpp.ActivityType = rpp.ActivityType.PLAYING,
        large_image: str = None,
        large_image_text: str = None,
        small_image: str = None,
        small_image_text: str = None,
        start_time: int = None,
        end_time: int = None,
        buttons: list[dict] = None,
    ):
        """
        Initialize the Profile object.
        """
        self.name = name
        self.app_id = app_id
        self.details = details
        self.state = state
        self.activity_type = activity_type
        self.large_image = large_image
        self.large_image_text = large_image_text
        self.small_image = small_image
        self.small_image_text = small_image_text
        self.start_time = start_time
        self.end_time = end_time
        self.buttons = buttons

    def __str__(self):
        """
        Return a string representation of the Profile object.
        """
        return f"Profile({self.name}, {self.app_id})"

    def __repr__(self):
        """
        Return a string representation of the Profile object.
        """
        return self.__str__()


# pylint: disable=too-many-instance-attributes
class CustomPresenceWindow(customtkinter.CTkToplevel):
    """
    CustomPresenceWindow class
    """

    def __init__(self, parent: customtkinter.CTk, title: str, width: int, height: int):
        """
        Initialize the CustomPresenceWindow class.
        """

        super().__init__(parent)
        self.profiles: list[Profile] = []
        self.current_profile: Profile = None
        self.frame: customtkinter.CTkFrame = None
        self.rpc: rpp.ClientRPC = None
        self.log = rpp.get_logger(self.__class__.__name__)
        self.rpc_connected = False
        self.title(title)
        self.geometry(f"{width}x{height}")
        self.resizable(False, False)
        self.protocol("WM_DELETE_WINDOW", self.on_close)

        self.configure_frame()
        self.configure_header()
        self.add_widgets()
        self.add_buttons()
        self.configure_profiles()
        self.load_profiles()

    def configure_frame(self):
        """
        Configure the frame for the custom presence window.
        """
        frame = customtkinter.CTkFrame(self)
        frame.grid(row=0, column=3, padx=20, pady=20, sticky="nsew")
        frame.grid_columnconfigure(0, weight=1)
        frame.grid_columnconfigure(1, weight=1)
        frame.grid_columnconfigure(2, weight=1)
        frame.grid_columnconfigure(3, weight=1)
        self.frame = frame

    def configure_header(self):
        """
        Configure the header for the custom presence window.
        """
        custom_label = customtkinter.CTkLabel(
            self.frame, text="Custom Presence", font=("Arial", 20)
        )
        custom_label.grid(row=0, column=0, pady=(0, 10), sticky="ew", columnspan=2)

        self.switch_connection = customtkinter.CTkSwitch(
            self.frame,
            text="Connect",
            command=self.on_connect_custom_presence,
        )
        self.switch_connection.grid(row=0, column=3, pady=(0, 10))

    def add_widgets(self):
        """
        Add widgets to the custom presence window.
        """
        # App Id
        app_id_label = customtkinter.CTkLabel(self.frame, text="App ID")
        app_id_label.grid(row=2, column=0, pady=(0, 10), padx=(0, 10))
        self.app_id_entry = customtkinter.CTkEntry(
            self.frame, placeholder_text="123456789010"
        )
        self.app_id_entry.grid(row=2, column=1, pady=(0, 10), padx=(0, 10))

        # type
        label_combo = customtkinter.CTkLabel(self.frame, text="Type")
        label_combo.grid(row=2, column=2, pady=(0, 10), padx=(0, 10))
        self.type_combo = customtkinter.CTkComboBox(
            self.frame,
            state="readonly",
            values=["PLAYING", "WATCHING", "LISTENING", "COMPETING"],
        )
        self.type_combo.grid(row=2, column=3, pady=(0, 10))

        # details and state fields
        details_label = customtkinter.CTkLabel(self.frame, text="Details")
        details_label.grid(row=3, column=0, sticky="e", pady=(0, 10), padx=(0, 10))

        self.details_entry = customtkinter.CTkEntry(
            self.frame, placeholder_text="Thinking..."
        )
        self.details_entry.grid(row=3, column=1, pady=(0, 10), padx=(0, 10))

        state_label = customtkinter.CTkLabel(self.frame, text="State", anchor="e")
        state_label.grid(row=3, column=2, pady=(0, 10), padx=(0, 10))
        self.state_entry = customtkinter.CTkEntry(
            self.frame, placeholder_text="How to code?"
        )
        self.state_entry.grid(row=3, column=3, pady=(0, 10))

        # large image value and large image text
        large_image_label = customtkinter.CTkLabel(self.frame, text="Image #1")
        large_image_label.grid(row=4, column=0, pady=(0, 10), padx=(0, 10))
        self.large_image_entry = customtkinter.CTkEntry(
            self.frame, placeholder_text="https://example.com/image.png"
        )
        self.large_image_entry.grid(
            row=4, column=1, pady=(0, 10), columnspan=2, sticky="ew", padx=(0, 5)
        )
        self.large_image_text_entry = customtkinter.CTkEntry(
            self.frame, placeholder_text="My Large Image"
        )
        self.large_image_text_entry.grid(row=4, column=3, pady=(0, 10))

        # small image value and small image text
        small_image_label = customtkinter.CTkLabel(self.frame, text="Image #2")
        small_image_label.grid(row=5, column=0, pady=(0, 10), padx=(0, 10))
        self.small_image_entry = customtkinter.CTkEntry(
            self.frame, placeholder_text="https://example.com/image.png"
        )
        self.small_image_entry.grid(
            row=5, column=1, pady=(0, 10), columnspan=2, sticky="ew", padx=(0, 5)
        )
        self.small_image_text_entry = customtkinter.CTkEntry(
            self.frame, placeholder_text="My Small Image"
        )
        self.small_image_text_entry.grid(row=5, column=3, pady=(0, 10))

        # buttons
        first_button_label = customtkinter.CTkLabel(self.frame, text="Button #1")
        first_button_label.grid(row=6, column=0, pady=(0, 10), padx=(0, 10))
        self.first_button_label_entry = customtkinter.CTkEntry(
            self.frame, placeholder_text="Click me!"
        )
        self.first_button_label_entry.grid(row=6, column=1, pady=(0, 10), padx=(0, 10))
        self.first_button_url_entry = customtkinter.CTkEntry(
            self.frame, placeholder_text="https://example.com"
        )
        self.first_button_url_entry.grid(
            row=6, column=2, pady=(0, 10), padx=(0, 10), columnspan=2, sticky="ew"
        )

        second_button_label = customtkinter.CTkLabel(self.frame, text="Button #2")
        second_button_label.grid(row=7, column=0, pady=(0, 10), padx=(0, 10))
        self.second_button_label_entry = customtkinter.CTkEntry(
            self.frame, placeholder_text="Click me!"
        )
        self.second_button_label_entry.grid(row=7, column=1, pady=(0, 10), padx=(0, 10))
        self.second_button_url_entry = customtkinter.CTkEntry(
            self.frame, placeholder_text="https://example.com"
        )
        self.second_button_url_entry.grid(
            row=7, column=2, pady=(0, 10), padx=(0, 10), columnspan=2, sticky="ew"
        )

        # timestamps
        start_time_label = customtkinter.CTkLabel(self.frame, text="Start Time")
        start_time_label.grid(row=8, column=0, pady=(0, 10), padx=(0, 10))
        self.start_time_combo = customtkinter.CTkComboBox(
            self.frame, values=["Now", "None"]
        )
        self.start_time_combo.grid(row=8, column=1, pady=(0, 10), padx=(0, 10))

        end_time_label = customtkinter.CTkLabel(self.frame, text="End Time")
        end_time_label.grid(row=9, column=0, pady=(0, 10), padx=(0, 10))
        self.end_time_combo = customtkinter.CTkComboBox(
            self.frame, values=["None", "Tomorrow"]
        )
        self.end_time_combo.grid(row=9, column=1, pady=(0, 10), padx=(0, 10))

    def add_buttons(self):
        """
        Add buttons to the custom presence window.
        """
        # save button
        save_button = customtkinter.CTkButton(
            self.frame, text="Update", command=self.on_custom_presence_update
        )
        save_button.grid(row=10, column=1, pady=(0, 10), sticky="ew")

        # reset
        reset_button = customtkinter.CTkButton(self.frame, text="Reset", command=None)
        reset_button.grid(row=10, column=3, pady=(0, 10), sticky="ew")

        # new profile
        new_profile_button = customtkinter.CTkButton(
            self.frame, text="New Profile", command=self.on_new_profile
        )
        new_profile_button.grid(row=11, column=1, pady=(0, 10), sticky="ew")

        # delete profile
        delete_profile_button = customtkinter.CTkButton(
            self.frame, text="Delete Profile", command=self.on_delete_profile
        )
        delete_profile_button.grid(row=11, column=3, pady=(0, 10), sticky="ew")

    def configure_profiles(self):
        """
        Configure the profiles for the custom presence window.
        """
        current_profile_name_label = customtkinter.CTkLabel(
            self.frame, text="Current Profile"
        )
        current_profile_name_label.grid(row=1, column=0, pady=(0, 10), padx=(0, 10))
        self.current_profile_combo = customtkinter.CTkComboBox(
            self.frame,
            values=[],
            corner_radius=5,
        )
        self.current_profile_combo.grid(row=1, column=1, pady=(0, 10), padx=(0, 10))

        self.selected_profile_button = customtkinter.CTkButton(
            self.frame, text="Select", command=self.on_profile_selected
        )
        self.selected_profile_button.grid(
            row=1, column=2, pady=(0, 10), padx=(0, 10), columnspan=2, sticky="ew"
        )

    def update_custom_presence(self) -> dict:
        """
        Update the custom presence data
        It returns a dictionary with each field needed to update the presence.
        """
        details = self.details_entry.get()
        state = self.state_entry.get()
        large_image = self.large_image_entry.get()
        large_image_text = self.large_image_text_entry.get()
        small_image = self.small_image_entry.get()
        small_image_text = self.small_image_text_entry.get()
        first_button_label = self.first_button_label_entry.get()
        first_button_url = self.first_button_url_entry.get()
        second_button_label = self.second_button_label_entry.get()
        second_button_url = self.second_button_url_entry.get()

        # timestamps, for now we will only support "Now" and "Tomorrow"
        start_time = self.start_time_combo.get()
        end_time = self.end_time_combo.get()
        start_time = int(time.time()) if start_time == "Now" else None
        end_time = (
            int(time.time() + 86400)
            if end_time == "Tomorrow" and start_time is not None
            else None
        )

        def parse_activity_type(activity_type: str) -> rpp.ActivityType:
            """
            Parse the activity type.
            """
            if activity_type == "WATCHING":
                return rpp.ActivityType.WATCHING
            if activity_type == "LISTENING":
                return rpp.ActivityType.LISTENING
            if activity_type == "COMPETING":
                return rpp.ActivityType.COMPETING
            return rpp.ActivityType.PLAYING

        def parse_buttons() -> list[dict]:
            """
            Parse the buttons.
            """
            buttons = []
            if first_button_label and len(first_button_label) > 0:
                buttons.append({"label": first_button_label, "url": first_button_url})
            if second_button_label and len(second_button_label) > 0:
                buttons.append({"label": second_button_label, "url": second_button_url})
            if len(buttons) == 0:
                return None
            return buttons

        return {
            "app_id": self.app_id_entry.get(),
            "details": details if details else None,
            "state": state if state else None,
            "activity_type": parse_activity_type(self.type_combo.get()),
            "large_image": large_image if large_image else None,
            "large_image_text": large_image_text if large_image_text else None,
            "small_image": small_image if small_image else None,
            "small_image_text": small_image_text if small_image_text else None,
            "start_time": start_time if start_time else None,
            "end_time": end_time if end_time else None,
            "buttons": parse_buttons(),
        }

    def on_custom_presence_connect(self):
        """
        Connect to the custom presence.
        """
        data = self.update_custom_presence()
        app_id = data["app_id"]
        if not app_id.isdigit():
            message = CTkMb.CTkMessagebox(
                icon="warning",
                title="Invalid App ID",
                message="Please enter a valid discord App ID.",
                option_1="OK",
            )
            message.get()
            self.log.warning("Invalid App ID entered.")
            return

        try:
            self.rpc = rpp.ClientRPC(data["app_id"], debug=True)
            self.rpc.connect()
            self.rpc_connected = True
            self.log.info("Connected successfully to Discord.")
        # pylint: disable=broad-except
        # For now, we will catch all exceptions, but we should catch specific ones.
        except Exception as exc:
            self.log.error("Connecting to custom presence: %s", exc)
            self.rpc_connected = False
            message = CTkMb.CTkMessagebox(
                icon="cancel",
                title="Connection error",
                message="An error occurred: " + str(exc),
                option_1="OK",
            )
            message.get()

    def on_custom_presence_disconnect(self):
        """
        Disconnect from the custom presence.
        """
        if self.rpc is None:
            self.log.warning("RPC not connected.")
            return
        try:
            self.rpc.close()
            self.rpc_connected = False
            self.log.info("Disconnected successfully from Discord.")
        # pylint: disable=broad-except
        except Exception as exc:
            self.log.error("Error disconnecting from custom presence: %s", exc)
            message = CTkMb.CTkMessagebox(
                icon="cancel",
                title="Disconnection error",
                message="An error occurred: " + str(exc),
                option_1="OK",
            )
            message.get()

    def on_connect_custom_presence(self):
        """
        Connect or disconnect from the custom presence.
        """
        if self.rpc_connected:
            self.log.info("Disconnecting")
            self.on_custom_presence_disconnect()
        else:
            self.log.info("Connecting...")
            self.on_custom_presence_connect()
        if self.rpc_connected:
            self.switch_connection.select()
        else:
            self.switch_connection.deselect()

    def save_profiles_entries(self):
        """
        Save the profile entries.
        """
        with open("profiles.rpp", "wb") as file:
            pickle.dump(self.profiles, file)
            self.log.info("Saved profiles.")

    def on_custom_presence_update(self):
        """
        Update the custom presence.
        """
        data = self.update_custom_presence()
        if self.current_profile:
            self.current_profile.__dict__.update(data)
        self.update_profiles()
        self.save_profiles_entries()
        if not self.rpc_connected:
            message = CTkMb.CTkMessagebox(
                icon="warning",
                title="Connection required",
                message="Please connect it first.",
                option_1="OK",
            )
            message.get()
            return
        if self.rpc is None:
            self.log.warning("RPC not connected.")
            return
        self.rpc.update(
            details=data["details"],
            state=data["state"],
            activity_type=data["activity_type"],
            large_image=data["large_image"],
            large_text=data["large_image_text"],
            small_image=data["small_image"],
            small_text=data["small_image_text"],
            start_time=data["start_time"],
            end_time=data["end_time"],
            buttons=data["buttons"],
        )
        self.log.info("Updated manually.")

    def update_profiles(self):
        """
        Update the profiles list.
        Also updates the current profile combo box.
        """
        if not self.current_profile:
            self.current_profile = self.profiles[0]
        self.current_profile_combo.configure(
            values=[profile.name for profile in self.profiles],
        )
        self.current_profile_combo.set(self.current_profile.name)
        self.on_profile_selected()

    def on_new_profile(self):
        """
        Create a new profile.
        """
        name = self.current_profile_combo.get()
        if name == "Default" or len(name) == 0:
            message = CTkMb.CTkMessagebox(
                icon="warning",
                title="Invalid Profile Name",
                message="Please enter a valid profile name.",
                option_1="OK",
            )
            message.get()
            self.log.warning("Invalid profile name entered.")
            return
        data = self.update_custom_presence()
        profile = Profile(
            name=name,
            app_id=data["app_id"],
            details=data["details"],
            state=data["state"],
            activity_type=data["activity_type"],
            large_image=data["large_image"],
            large_image_text=data["large_image_text"],
            small_image=data["small_image"],
            small_image_text=data["small_image_text"],
            start_time=data["start_time"],
            end_time=data["end_time"],
            buttons=data["buttons"],
        )
        self.profiles.append(profile)
        self.current_profile = profile
        self.update_profiles()
        self.log.info("Created new profile: %s", profile.name)

    def on_delete_profile(self):
        """
        Delete the selected profile.
        """
        if not self.current_profile:
            self.log.warning("No profile selected.")
            return

        name = self.current_profile_combo.get()
        for profile in self.profiles:
            if profile.name == name:
                self.profiles.remove(profile)
                self.log.info("Deleted profile: %s", profile.name)
                self.update_profiles()
                break

    def load_profiles(self):
        """
        Load the profiles from profiles.rpp.
        """
        if not os.path.exists("profiles.rpp"):
            self.log.info("No profiles found. Using default profile.")
            self.current_profile = Profile()
            self.profiles.append(self.current_profile)
            self.save_profiles_entries()
        else:
            with open("profiles.rpp", "rb") as file:
                self.profiles = pickle.load(file)
                self.current_profile = self.profiles[0]
                self.log.info("Loaded %s profiles.", len(self.profiles))
        self.update_profiles()

    def on_profile_selected(self):
        """
        Select a profile from the combo box.
        """
        name = self.current_profile_combo.get()
        for profile in self.profiles:
            if profile.name == name:
                self.current_profile = profile
                self.log.info("Selected profile: %s", profile.name)

        if not self.current_profile:
            self.log.warning("No profile selected.")
            return

        # change entries
        self.app_id_entry.delete(0, "end")
        self.app_id_entry.insert(0, "")
        if self.current_profile.app_id:
            self.app_id_entry.insert(0, self.current_profile.app_id)

        self.details_entry.delete(0, "end")
        self.details_entry.insert(0, "")
        if self.current_profile.details:
            self.details_entry.insert(0, self.current_profile.details)

        self.state_entry.delete(0, "end")
        self.state_entry.insert(0, "")
        if self.current_profile.state:
            self.state_entry.insert(0, self.current_profile.state)

        if self.current_profile.activity_type:
            self.type_combo.set(self.current_profile.activity_type.name)

        self.large_image_entry.delete(0, "end")
        self.large_image_entry.insert(0, "")
        if self.current_profile.large_image:
            self.large_image_entry.insert(0, self.current_profile.large_image)

        self.large_image_text_entry.delete(0, "end")
        self.large_image_text_entry.insert(0, "")
        if self.current_profile.large_image_text:
            self.large_image_text_entry.insert(0, self.current_profile.large_image_text)

        self.small_image_entry.delete(0, "end")
        self.small_image_entry.insert(0, "")
        if self.current_profile.small_image:
            self.small_image_entry.insert(0, self.current_profile.small_image)

        self.small_image_text_entry.delete(0, "end")
        self.small_image_text_entry.insert(0, "")
        if self.current_profile.small_image_text:
            self.small_image_text_entry.insert(0, self.current_profile.small_image_text)

        if self.current_profile.buttons and len(self.current_profile.buttons) >= 2:
            self.first_button_label_entry.delete(0, "end")
            self.first_button_label_entry.insert(
                0, self.current_profile.buttons[0]["label"]
            )

            self.first_button_url_entry.delete(0, "end")
            self.first_button_url_entry.insert(
                0, self.current_profile.buttons[0]["url"]
            )

            self.second_button_label_entry.delete(0, "end")
            self.second_button_label_entry.insert(
                0, self.current_profile.buttons[1]["label"]
            )

            self.second_button_url_entry.delete(0, "end")
            self.second_button_url_entry.insert(
                0, self.current_profile.buttons[1]["url"]
            )

    def on_close(self):
        """
        Close the custom presence window
        """
        self.log.info("Window closed.")
        self.destroy()
