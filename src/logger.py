"""
Logger module.
"""

import logging
import logging.handlers
import pathlib
import typing

from .constants import config

try:
    config.logs_dir.mkdir(exist_ok=True, parents=True)
except Exception as exc:
    print(f"Error creating log directory: {exc}")

_LOG_FILENAME = "app.log"


def _configure_root(
    level: int = logging.INFO,
    directory: pathlib.Path = config.logs_dir,
    filename: str = _LOG_FILENAME,
) -> None:
    """
    Configures the root logger.
    """
    root = logging.getLogger()
    if root.handlers:
        return
    root.setLevel(level)
    # ISO8601-like
    fmt = "%(asctime)s.%(msecs)03d %(levelname)s %(module)s %(name)s:%(lineno)d - %(message)s"
    _format = logging.Formatter(fmt)
    _console_handler = logging.StreamHandler()
    _console_handler.setLevel(level)
    _console_handler.setFormatter(_format)
    root.addHandler(_console_handler)
    log_path = directory / filename
    max_bytes = 5 * 1024 * 1024
    backup_count = 3
    encoding = "utf-8"
    _file_handler = logging.handlers.RotatingFileHandler(
        str(log_path),
        maxBytes=max_bytes,
        backupCount=backup_count,
        encoding=encoding,
    )
    _file_handler.setLevel(level)
    _file_handler.setFormatter(_format)
    root.addHandler(_file_handler)


_configure_root()


def set_log_level(level: str) -> None:
    """
    Change log level at runtime for root logger and all handlers.
    Allowed values: DEBUG, INFO, WARNING, ERROR, CRITICAL
    """
    level = level.upper()
    if level == "DEBUG":
        log_level = logging.DEBUG
    elif level == "INFO":
        log_level = logging.INFO
    elif level == "WARNING":
        log_level = logging.WARNING
    elif level == "ERROR":
        log_level = logging.ERROR
    elif level == "CRITICAL":
        log_level = logging.CRITICAL
    else:
        raise ValueError(f"Invalid log level: {level}")
    root = logging.getLogger()
    root.setLevel(log_level)
    for handler in root.handlers:
        handler.setLevel(log_level)
    root.info("Log level changed to %s", level)


def get_logger(name: typing.Optional[str] = None) -> logging.Logger:
    """
    Returns a logger with the specified name.
    """
    return logging.getLogger(name)


logger = get_logger(config.logs_default_name)
