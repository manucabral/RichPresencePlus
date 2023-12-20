"""
Utilities methods and classes used by the RPP.
"""
import os
import yaml
from .logger import log
from .constants import RESTRICTED_GLOBALS


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


def restrict_globals(_origin: dict, src: "Presence") -> dict:
    """
    Restrict the globals of the execution environment.
    """
    for name in RESTRICTED_GLOBALS:
        _origin[name] = lambda: src.stop() and RestrictedUse(name)
    return _origin


def import_modules(_origin: dict) -> dict:
    """
    Import a list of modules for the presence (custom not supported yet).
    """
    if not isinstance(_origin, list):
        return _origin
    for module in self.__metadata[_type]:
        try:
            _origin[module] = __import__(module)
        except Exception as exc:
            log(
                "Failed to import " + module + " because " + str(exc),
                level="ERROR",
            )
    return _origin


def is_dir(path) -> bool:
    """
    Check if a path is a directory.
    """
    return os.path.isdir(path)


def list_dir(path) -> list:
    """
    List the contents of a directory.
    """
    return os.listdir(path)


def load_local_presences_metadata(dev_mode: bool = False) -> list:
    """
    Load the local presences metadata.

    Args:
        dev_mode (bool): Whether to load the presences in dev mode. Defaults to False.

    Returns:
        list: A list of Presence objects.
    """
    presences = []
    if not is_dir("presences"):
        return presences
    for file in list_dir("presences"):
        subdir = os.path.join("presences", file)
        if not is_dir(subdir):
            continue
        files = list_dir(subdir)
        if "metadata.yml" not in files or "main.py" not in files:
            log(
                f"Skipping {file} because it is not a valid presence.",
                level="WARNING",
            )
            continue
        with open(os.path.join(subdir, "metadata.yml")) as _file:
            metadata = yaml.safe_load(_file)
        metadata["path"] = subdir
        metadata["dev_mode"] = dev_mode
        presences.append(metadata)
    log(f"Loaded a total of {len(presences)} presences metadata.", dev_mode=dev_mode)
    return presences
