"""
This module contains the custom logger class for RPP.
"""

import logging


class RPPLogger(logging.Logger):
    """
    Custom logger class for RPP.
    """

    def __init__(self, name: str, level=logging.DEBUG, filename="rpp.log"):
        super().__init__(name, level)
        self.setLevel(level)

        # File handler
        file_handler = logging.FileHandler(filename, encoding="utf-8")
        file_handler.setLevel(logging.DEBUG)
        file_handler.setFormatter(
            logging.Formatter("%(asctime)s - %(name)s - %(levelname)s - %(message)s")
        )
        self.addHandler(file_handler)

        # Console handler
        self.console_handler = logging.StreamHandler()
        self.console_handler.setLevel(logging.DEBUG)
        self.console_handler.setFormatter(
            logging.Formatter("%(name)s - %(levelname)s - %(message)s")
        )
        self.addHandler(self.console_handler)

    def __log(
        self, level: int, message: str, *args, console: bool = True, **kwargs
    ) -> None:
        """
        Log a message with the specified level.
        """
        if console:
            self.console_handler.setLevel(level)
        else:
            self.console_handler.setLevel(logging.CRITICAL + 1)
        super().log(level, message, *args, **kwargs)

    def info(self, msg: str, *args, console: bool = True, **kwargs) -> None:
        self.__log(logging.INFO, msg, *args, console=console, **kwargs)

    def debug(self, msg: str, *args, console: bool = True, **kwargs) -> None:
        self.__log(logging.DEBUG, msg, *args, console=console, **kwargs)

    def error(self, msg: str, *args, console: bool = True, **kwargs) -> None:
        self.__log(logging.ERROR, msg, *args, console=console, **kwargs)

    def warning(self, msg: str, *args, console: bool = True, **kwargs) -> None:
        self.__log(logging.WARNING, msg, *args, console=console, **kwargs)

    def critical(self, msg: str, *args, console: bool = True, **kwargs) -> None:
        self.__log(logging.CRITICAL, msg, *args, console=console, **kwargs)


logging.setLoggerClass(RPPLogger)


def get_logger(name: str, filename="rpp.log") -> RPPLogger:
    """
    Get a custom logger for RPP.

    Args:
        name (str): The name of the logger.
        filename (str): The filename for the log file.

    Returns:
        RPPLogger: The custom logger instance.
    """
    return RPPLogger(name, filename=filename)
