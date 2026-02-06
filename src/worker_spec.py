"""
Worker specification dataclass.
"""

import dataclasses
import threading
from typing import Optional, Any
from .constants import config


@dataclasses.dataclass
class WorkerSpecification:
    """
    Specification for a worker process.
    """

    name: str
    path: str
    client_id: Optional[str] = None
    entrypoint: str = config.presences_entrypoint
    callable_name: str = config.presences_callable
    interval: int = 15
    enabled: bool = True
    backoff_time: int = 5
    runs: int = 0
    verified: bool = False
    web: bool = False
    running: bool = False
    description: Optional[str] = None
    on_exit: Optional[str] = None
    image: Optional[str] = None
    # thread: Optional[threading.Thread] = dataclasses.field(default=None, repr=False)  # Not used
    process: Optional[Any] = dataclasses.field(default=None, repr=False)
    stop_event: Optional[threading.Event] = dataclasses.field(default=None, repr=False)
    shared_state: Optional[Any] = dataclasses.field(default=None, repr=False)
    # last_exception: Optional[str] = dataclasses.field(default=None, repr=False)  # Not used
    # runtime: Optional[Any] = dataclasses.field(default=None, repr=False)  # Not used
