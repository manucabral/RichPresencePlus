"""
Utilities methods and classes used by the RPP.
"""
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
