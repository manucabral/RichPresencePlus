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
