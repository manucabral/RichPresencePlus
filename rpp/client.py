"""
Client object to interact with the browser using a WebSocket connection.
"""

import json
import websocket


class Client:
    """
    Client object to interact with the browser.
    """

    def __init__(self, url):
        """
        Initialize a new Client object.
        """
        self.url = url
        self.ws = None

    def connect(self):
        """
        Connect to the browser.
        """
        self.ws = websocket.create_connection(self.url)

    def send(self, payload: dict):
        """
        Send a payload to the browser.
        """
        self.ws.send(json.dumps(payload))

    def receive(self):
        """
        Receive a payload from the browser.
        """
        return self.ws.recv()

    def close(self):
        """
        Close the connection to the browser.
        """
        self.ws.close()
