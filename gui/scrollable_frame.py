"""
Scrollable frame for the GUI.
"""

import customtkinter


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

        self.label = customtkinter.CTkLabel(
            self, text="My Presences", font=("Arial", 20)
        )
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
