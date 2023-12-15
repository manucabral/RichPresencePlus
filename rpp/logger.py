"""
A simple logging module.

Example:
    >>> from rpp import log
    >>> log("Hello, world!")
    [01/01/1970|00:00:00] @ INFO main: Hello, world!
"""
import time
import typing
import inspect
from .constants import LogLevel, LOG_FILENAME


# pylint: disable=redefined-outer-name
def log(
    text: typing.Optional[str] = None,
    level: typing.Optional[str] = LogLevel.INFO.name,
    src: typing.Optional[str] = None,
    **kwargs,
) -> None:
    """
    Logs a message. Defaults to INFO level.

    Args:
        text (str): The message to log.
        level (str): The level to log at. Defaults to INFO.
        src (str): The source file to log from. Defaults to the caller.

    Other Kwargs:
        console (bool): Whether to log to the console. Defaults to True.
        file (bool): Whether to log to a file. Defaults to True.

    Raises:
        ValueError: If the level is not valid.
    """
    if text is None:
        raise ValueError("No text to log.")
    if level not in LogLevel.__members__:
        raise ValueError("Invalid log level: " + level)
    if src is not None:  # if src is specified, use it.
        caller_filename = src
    else:
        caller_filename = inspect.stack()[1].filename.split("\\")[-1].replace(".py", "")
    # pylint: disable=pointless-string-statement
    """
    for future Linux support
    caller_filename = caller.filename.split("/")[-1]
    """
    date = time.strftime("[%d/%m/%Y|%H:%M:%S]")
    log = f"{date} @ {level} {caller_filename}: {text}"
    if kwargs.get("console", True):
        print(log)
    if kwargs.get("file", True):
        with open(LOG_FILENAME, "a", encoding="utf-8") as log_file:
            log_file.write(log + "\n")
