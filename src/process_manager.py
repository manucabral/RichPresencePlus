"""
Port process management for Windows systems.
"""

import subprocess
import os
from dataclasses import dataclass, asdict
from typing import Iterable, List, Dict
from .logger import logger


@dataclass
class ProcessInfo:
    """
    Simple process descriptor.

    Attributes:
        pid: Process id (int).
        state: Connection/state string (e.g. 'LISTENING').
    """

    pid: int
    state: str

    def to_dict(self) -> Dict[str, object]:
        """Return a dict representation of the ProcessInfo."""
        return asdict(self)


def is_discord_running() -> bool:
    """Check if Discord is currently running."""
    try:
        output = subprocess.check_output(
            ["tasklist", "/FI", "IMAGENAME eq Discord.exe"],
            universal_newlines=True,
            creationflags=subprocess.CREATE_NO_WINDOW,
        )
        return "Discord.exe" in output
    except Exception as exc:
        logger.warning("Failed to check if Discord is running: %s", exc)
        return False


def close_pids(processes: Iterable[ProcessInfo]):
    """Force close the given processes by their PIDs.

    Filters out invalid/non-positive PIDs, skips the current process PID,
    removes duplicates and logs skipped items.
    """
    all_pids = [getattr(p, "pid", None) for p in processes]
    logger.info("Force closing PIDs: %s", all_pids)

    to_kill = set()
    for pid in all_pids:
        try:
            if pid is None:
                raise ValueError("PID is None")
            ival = int(pid)
        except Exception:
            logger.warning("Skipping invalid pid: %r", pid)
            continue
        if ival <= 0:
            logger.warning("Skipping non-positive pid: %d", ival)
            continue
        if ival == os.getpid():
            logger.warning("Skipping current process pid: %d", ival)
            continue
        to_kill.add(ival)

    if not to_kill:
        logger.info("No valid PIDs to close")
        return

    for pid in to_kill:
        try:
            subprocess.run(
                ["taskkill", "/PID", str(pid), "/F"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                creationflags=subprocess.CREATE_NO_WINDOW,
                check=False,
            )
            logger.info("Requested taskkill for PID %d", pid)
        except Exception as exc:
            logger.warning("Failed to taskkill PID %d: %s", pid, exc)


def get_processes_by_port(port: int) -> List[ProcessInfo]:
    """
    Get a list of processes using the specified port.
    """
    logger.info("Getting processes using port %d", port)
    output = subprocess.check_output(
        ["netstat", "-ano"],
        creationflags=subprocess.CREATE_NO_WINDOW,
        universal_newlines=True,
    )
    results: List[ProcessInfo] = []
    for line in output.splitlines():
        if f":{port}" not in line:
            continue
        parts = line.split()
        # TCP 127.0.0.1:9222 0.0.0.0:0 LISTENING 38792
        if len(parts) >= 5 and parts[0].upper() == "TCP":
            state = parts[3]
            if state == "SYN_SENT":
                continue
            try:
                pid = int(parts[4])
                if pid > 0:
                    results.append(ProcessInfo(pid=pid, state=state))
                else:
                    logger.warning(
                        "Skipping non-positive pid found in netstat: %d", pid
                    )
            except ValueError:
                logger.warning("Invalid PID value in netstat output: %r", parts[4])
    return results
