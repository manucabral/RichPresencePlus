"""
Simply a class derived from pypresence.Presence with some extra features.
"""
import os
import time
import threading
import pypresence as pp
from .logger import log


class Presence:
    """
    Rich Presence manager.
    """

    __slots__ = [
        "__metadata",
        "__code",
        "__rpc",
        "__rpc_data",
        "__running",
        "__connected",
        "__code_running",
        "__thread_code",
        "__thread_rpc",
    ]

    def __init__(self, **metadata):
        """
        Create a new Presence object.
        """
        self.__metadata = metadata
        try:
            self.__rpc = pp.Presence(self.__metadata["client_id"])
            main_path = os.path.join(metadata["path"], "main.py")
            with open(main_path) as _file:
                self.__code = compile(_file.read(), main_path, "exec")
            log(
                "Compiled successfully v" + metadata["version"],
                src=metadata["name"],
            )
        except Exception as exc:
            log(
                f"{metadata['name']} failed because {exc}",
                level="ERROR",
            )
            return
        self.__running = self.__connected = self.__code_running = False
        self.__rpc_data = {}

    def __repr__(self):
        """
        Return the representation of the object.
        """
        return f"<Presence {self.__metadata['name']}>"

    def __code_update(self) -> None:
        """
        Run the presence code.
        """
        _globals = {
            "__builtins__": __builtins__,
            "log": lambda text, **kwargs: log(
                text, src=self.__metadata["name"], **kwargs
            ),
            "presence_update": self.update,
            "presence_metadata": self.__metadata,
            "time": time,
        }
        if self.__metadata["modules"] is not None:
            for module in self.__metadata["modules"]:
                _globals[module] = __import__(module)
        _globals = restrict_globals(_globals, self)
        self.__code_running = True
        try:
            while self.__code_running:
                exec(self.__code, _globals)
                time.sleep(0.1)
        except Exception as exc:
            print(exc)
            log(
                f"{self.__metadata['name']} failed because {exc}",
                level="ERROR",
            )
        finally:
            self.__code_running = False
            self.__connected = False

    def __rpc_update(self) -> None:
        """
        Update the RPC connection.
        """
        try:
            while self.__connected:
                self.__rpc.update(**self.__rpc_data)
                log("Updated", src=self.__metadata["name"])
                time.sleep(15)
        except Exception as exc:
            log(
                f"{self.__metadata['name']} failed because RPC {exc}",
                level="ERROR",
            )
        finally:
            self.__connected = False

    def connect(self) -> None:
        """
        Connect to the Discord client.
        """
        if self.__connected:
            log(f"{self.__metadata['name']} is already connected.", level="WARNING")
            return
        log(f"Connecting to {self.__metadata['name']}...")
        try:
            self.__rpc.connect()
            self.__connected = True
            log(f"Connected to {self.__metadata['name']}!")
        except Exception as exc:
            log(
                f"{self.__metadata['name']} failed because {exc}",
                level="ERROR",
            )

    def update(self, **kwargs) -> None:
        """
        Update the presence data.
        """
        if not self.__connected:
            log(f"Can't update {self.__metadata['name']} because it's not connected.")
            return
        self.__rpc_data = kwargs

    def start(self) -> None:
        """
        Start the presence in two threads.
        1. The code thread.
        2. The RPC thread.
        """
        if not self.__connected:
            log(f"Can't start {self.__metadata['name']} because it's not connected.")
            return
        self.__running = True
        self.__thread_code = threading.Thread(target=self.__code_update)
        self.__thread_rpc = threading.Thread(target=self.__rpc_update)
        self.__thread_rpc.start()
        self.__thread_code.start()

    def stop(self) -> None:
        """
        Stop the presence.
        """
        self.__code_running = self.__running = False
        log(f"Stopping {self.__metadata['name']}...")
        try:
            self.__rpc.clear()
            self.__rpc.close()
        except Exception as exc:
            print(exc)
            log(
                f"{self.__metadata['name']} CON failed because {exc}",
                level="ERROR",
            )
        self.__connected = False
        self.__thread_code.join()
        self.__thread_rpc.join()
        log(f"Stopped {self.__metadata['name']}!")


# temporary, will be moved to a separate file
class RestrictedUse:
    """
    A class that raises an error when called.
    """

    def __init__(self, name: str) -> None:
        """
        Create a new RestrictedUse object.
        """
        self.name = name

    def __call__(self, *args, **kwargs):
        """
        Raise an error.
        """
        raise RuntimeError(f"{self.name} is not allowed for security reasons.")


# temporary, will be moved to a separate file
def restrict_globals(_origin: dict, src: Presence) -> None:
    not_allowed = [
        "input",
        "open",
        "eval",
        "exec",
        "compile",
        "exit",
        "quit",
    ]
    for name in not_allowed:
        _origin[name] = lambda: src.stop() and RestrictedUse(name)
    return _origin
