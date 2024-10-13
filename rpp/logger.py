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
        file_handler = logging.FileHandler(filename)
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

    def info(self, message: str, *args, console: bool = True, **kwargs) -> None:
        self.__log(logging.INFO, message, *args, console=console, **kwargs)

    def debug(self, message: str, *args, console: bool = True, **kwargs) -> None:
        self.__log(logging.DEBUG, message, *args, console=console, **kwargs)

    def error(self, message: str, *args, console: bool = True, **kwargs) -> None:
        self.__log(logging.ERROR, message, *args, console=console, **kwargs)

    def warning(self, message: str, *args, console: bool = True, **kwargs) -> None:
        self.__log(logging.WARNING, message, *args, console=console, **kwargs)

    def critical(self, message: str, *args, console: bool = True, **kwargs) -> None:
        self.__log(logging.CRITICAL, message, *args, console=console, **kwargs)


logging.setLoggerClass(RPPLogger)


def get_logger(name: str, filename="rpp.log") -> RPPLogger:
    """
    Get a custom logger for RPP.
    """
    return logging.getLogger(name)
