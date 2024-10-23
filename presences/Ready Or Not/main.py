import rpp
import os
import time


@rpp.extension
class ReadyOrNot(rpp.Presence):
    def __init__(self):
        super().__init__(metadata_file=True)
        self.account: rpp.SteamAccount = None
        self.last_state: str = None
        self.logo = "https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/1144200/header.jpg"

    def on_load(self):
        self.state = "Waiting"
        self.details = "No connection"
        self.start_time = int(time.time())

    def set_account(self, accounts: list[rpp.SteamAccount]):
        steam_id = os.getenv("STEAM_ID")
        if steam_id:
            self.account = rpp.SteamAccount("User", int(steam_id))
        elif len(accounts) > 0:
            self.account = accounts[0]
        if self.account is None:
            self.log.info("No steam account found")
            return
        self.log.info("Using %s" % self.account)

    def on_update(self, steam: rpp.Steam, **context):
        if self.account is None:
            self.set_account(steam.accounts)
        if not self.account:
            return
        data = rpp.get_steam_presence(self.account.steam_id32)
        if data is None:
            self.log.info("No data found")
            return
        self.details = data["name"] or "Ready or Not"
        self.state = data["state"] or "Playing"
        if self.last_state != self.state:
            self.last_state = self.state
            self.start_time = int(time.time())
            self.log.info("State changed to %s" % self.state)
        self.start = self.start_time
        self.large_image = self.logo

    def on_close(self):
        self.log.info("Closed")
