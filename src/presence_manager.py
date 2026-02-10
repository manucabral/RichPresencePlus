"""
Presence manager that discovers and runs presence workers.
"""

import os
import inspect
import sys
import json
import pathlib
import threading
import multiprocessing as _mp
import importlib
import importlib.util
import re
import types
from typing import Dict, Optional, Any

from .utils import _resolve_callable
from .github_sync import sync, force_sync
from .logger import logger, get_logger
from .constants import config
from .worker_spec import WorkerSpecification
from .rpc import ClientRPC
from .runtime.runtime import Runtime
from .runtime.runtime_shim import SimpleRuntimeShim
from .steam import SteamAccount


# pylint: disable=too-many-branches
# pylint: disable=too-many-locals
# pylint: disable=too-many-nested-blocks
def process_worker(
    path: str,
    entrypoint: str,
    callable_name: str,
    interval: int,
    client_id: Optional[str] = None,
    stop_event: Optional[Any] = None,
    shared_pages: Optional[Any] = None,
    steam_account: Optional[SteamAccount] = None,
    shared_state: Optional[Any] = None,
) -> None:
    """
    Worker process entrypoint. Loads and runs the specified presence worker.

    Parameters:
        path (str): Path to the presence worker module.
        entrypoint (str): Entrypoint filename within the module.
        callable_name (str): Name of the callable to execute.
        interval (int): Update interval in seconds.
        client_id (Optional[str]): RPC client ID.
        stop_event (Optional[Any]): Multiprocessing Event to signal shutdown.
        shared_pages (Optional[Any]): Manager list proxy for shared pages.
        steam_account (Optional[SteamAccount]): Steam account to use.
        shared_state (Optional[Any]): Manager dict proxy for sharing RPC state.
    """
    rpc: Optional[ClientRPC] = None
    runtime: Optional[Any] = None

    try:
        logger.debug(
            "process_worker starting in pid=%s name=%s",
            os.getpid(),
            _mp.current_process().name,
        )

        rpc = ClientRPC(client_id=client_id, debug=True)
        rpc.connect()
        module_path = pathlib.Path(path)

        package_base_name = None
        try:
            manifest_file_early = module_path / "manifest.json"
            if manifest_file_early.exists():
                try:
                    mf_early = json.loads(
                        manifest_file_early.read_text(encoding="utf-8", errors="ignore")
                    )
                except Exception:
                    mf_early = None
            else:
                mf_early = None
        except Exception:
            mf_early = None

        process_name = "pp_" + module_path.name

        # if manifest specifies imports, create a temporary package and preload files
        try:
            imports_list = None
            if mf_early and isinstance(mf_early, dict):
                imports_list = mf_early.get("imports")
            if imports_list and isinstance(imports_list, list):
                package_base_name = re.sub(r"[^0-9a-zA-Z_]+", "_", module_path.name)
                package_name = f"presences.{package_base_name}"
                pkg_mod = types.ModuleType(package_name)
                pkg_mod.__path__ = [str(module_path)]
                sys.modules[package_name] = pkg_mod
                for fname in imports_list:
                    try:
                        file_path = module_path / fname
                        if not file_path.exists():
                            logger.warning(
                                "Import file %s listed in manifest not found for %s",
                                fname,
                                module_path,
                            )
                            continue
                        mod_name = f"{package_name}.{file_path.stem}"
                        spec_i = importlib.util.spec_from_file_location(
                            mod_name, str(file_path)
                        )
                        if spec_i and spec_i.loader:
                            m = importlib.util.module_from_spec(spec_i)
                            spec_i.loader.exec_module(m)
                            sys.modules[mod_name] = m
                            logger.info(
                                "Preloaded import %s for %s", fname, module_path
                            )
                    except Exception:
                        logger.debug(
                            "Failed to preload import %s for %s", fname, module_path
                        )

                # set process_name to a namespaced module so __package__ is correct
                entry_stem = pathlib.Path(entrypoint).stem
                process_name = f"{package_name}.{entry_stem}"
        except Exception:
            process_name = "pp_" + module_path.name

        spec = importlib.util.spec_from_file_location(
            process_name, str(module_path / entrypoint)
        )
        if spec is None or spec.loader is None:
            logger.error("Could not load module for presence worker at %s", path)
            sys.exit(1)

        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)

        func, resolved_name = _resolve_callable(module, callable_name)
        if func is None:
            logger.error(
                "Callable %s not found in module %s", callable_name, entrypoint
            )
            sys.exit(1)
        if resolved_name != callable_name:
            logger.warning(
                "Requested callable %s not found, using %s instead",
                callable_name,
                resolved_name,
            )

        # prepare extensible context
        manifest_file = module_path / "manifest.json"
        web_enabled = False
        if manifest_file.exists():
            try:
                mf = json.loads(
                    manifest_file.read_text(encoding="utf-8", errors="ignore")
                )
                web_enabled = bool(mf.get("web", False))
            except Exception as exc:
                logger.debug("Could not read manifest for %s: %s", module_path, exc)
                web_enabled = False
        else:
            logger.debug(
                "No manifest.json for %s; runtime will be disabled unless manifest sets web:true",
                module_path,
            )

        runtime = None
        if web_enabled:
            logger.debug("Web runtime enabled for worker %s", module_path.name)
            if shared_pages is not None:
                try:
                    logger.debug(
                        "Using shared pages proxy inside worker %s", module_path.name
                    )
                    runtime = SimpleRuntimeShim(shared_pages)
                    try:
                        pages_count = len(list(shared_pages))
                    except Exception:
                        pages_count = 0
                    logger.debug(
                        "Using shared pages proxy inside worker %s (pages=%d)",
                        module_path.name,
                        pages_count,
                    )
                    logger.debug(
                        "Worker runtime type=shim (shared_pages) module=%s",
                        module_path.name,
                    )
                except Exception as exc:
                    runtime = None
                    logger.debug(
                        "Failed to build runtime from shared_pages for %s: %s",
                        module_path.name,
                        exc,
                    )
            else:
                try:
                    runtime = Runtime(origin=f"worker:{module_path.name}")
                    try:
                        try:
                            runtime.load(start_background=False)
                        except TypeError:
                            runtime.load()

                    except Exception as inner_exc:
                        runtime = None
                        logger.debug(
                            "Failed to initialize local Runtime for %s: %s",
                            module_path.name,
                            inner_exc,
                        )
                except Exception as exc:
                    runtime = None
                    logger.debug(
                        "Failed to create Runtime for %s: %s", module_path.name, exc
                    )

        context: Dict[str, Any] = {
            "client_id": client_id,
            "path": str(module_path),
            "interval": interval,
            "runtime": runtime,
            "stop_event": stop_event,
            "process_name": process_name,
            "steam_account": steam_account,
            "shared_state": shared_state,
        }
        try:
            signature = inspect.signature(func)
            params = list(signature.parameters.values())
            logger.debug("Calling worker %s with params: %s", path, params)
            if len(params) == 0:
                result = func()
            else:
                args = []
                for p in params:
                    if p.name == "rpc":
                        args.append(rpc)
                    elif p.name == "context":
                        args.append(context)
                    elif p.name == "stop_event":
                        args.append(stop_event)
                    elif p.name in ("runtime", "rt"):
                        args.append(runtime)
                    elif p.name in ("steam_account", "sa"):
                        args.append(steam_account)
                    elif p.name == "interval":
                        args.append(interval)
                    elif p.name == "logger":
                        args.append(get_logger("worker"))
                    elif p.name == "shared_state":
                        args.append(shared_state)
                    else:
                        args.append(context)
                result = func(*args)

            if isinstance(result, dict) and rpc is not None:
                try:
                    rpc.update(
                        state=result.get("state"),
                        details=result.get("details"),
                        activity_type=result.get("activity_type"),
                        start_time=result.get("start_time"),
                        end_time=result.get("end_time"),
                        large_image=result.get("large_image"),
                        large_text=result.get("large_text"),
                        small_image=result.get("small_image"),
                        small_text=result.get("small_text"),
                        buttons=result.get("buttons"),
                    )
                except Exception:
                    logger.exception("Failed to call rpc.update for %s", path)
        except Exception:
            logger.exception("Error calling callable for worker %s", path)
            sys.exit(1)
    except Exception as exc:
        logger.exception("Fatal error in process_worker for %s -> %s", path, exc)
    finally:
        if rpc is not None:
            try:
                try:
                    rpc.clear_activity()
                except Exception:
                    logger.debug(
                        "rpc.clear_activity() failed (continuing to close)",
                        exc_info=True,
                    )
                rpc.close()
            except Exception:
                logger.exception("Error closing rpc for worker %s", path)
        if runtime is not None:
            try:
                # if runtime has a close method, call it
                close_fn = getattr(runtime, "close", None)
                if callable(close_fn):
                    # pylint: disable=not-callable
                    close_fn()
            except Exception:
                logger.exception("Error closing runtime for worker %s", path)


class PresenceManager:
    """
    Manages presence worker discovery and lifecycle.
    """

    def __init__(
        self, default_backoff: int = 5, runtime: Optional[Runtime] = None
    ) -> None:
        """
        Initializes the PresenceManager.
        """
        self.default_backoff = default_backoff
        self.presences_dir = config.presences_dir
        self.workers: Dict[str, WorkerSpecification] = {}
        self.supervisor_thread: Optional[threading.Thread] = None
        self.steam_account: Optional[SteamAccount] = None
        self._stop_event = threading.Event()
        self._lock = threading.Lock()
        self._mp_manager = None
        self.shared_pages = None
        self._runtime = runtime
        if self._runtime is None:
            try:
                self._runtime = Runtime(origin="presence_manager")
                self._runtime.load()
            except Exception:
                logger.debug("Failed to initialize shared Runtime in PresenceManager")
                self._runtime = None

        try:
            default_interval = float(getattr(self._runtime, "interval", 1.0) or 1.0)
        except Exception:
            default_interval = 1.0
        self._pages_sync_interval = default_interval
        self._pages_sync_thread = None

        try:
            if self._pages_sync_thread is None:
                self._pages_sync_thread = threading.Thread(
                    target=self._sync_pages_loop, daemon=True
                )
                self._pages_sync_thread.start()
        except Exception:
            logger.exception("Failed to start pages sync thread")

    def _sync_pages_loop(self) -> None:
        """
        Background loop that refreshes the shared_pages proxy from the
        manager's Runtime. Runs until self._stop_event is set.
        """
        logger.debug(
            "Starting shared pages sync loop (interval=%s)", self._pages_sync_interval
        )
        while not self._stop_event.is_set():
            try:
                if self.shared_pages is not None and self._runtime is not None:
                    # skip sync if runtime has no protocol set (no browser connected)
                    if not getattr(self._runtime, "protocol", None):
                        self._stop_event.wait(
                            self._pages_sync_interval
                            if getattr(self, "_pages_sync_interval", None) is not None
                            else 1.0
                        )
                        continue

                    try:
                        # pylint: disable=not-callable
                        load_fn = getattr(self._runtime, "load", None)
                        if callable(load_fn):
                            load_fn()
                    except Exception:
                        logger.debug(
                            "Runtime.load() failed in pages sync loop", exc_info=True
                        )

                    snapshot = []
                    try:
                        pages = list(self._runtime.pages)
                    except Exception:
                        pages = []

                    for p in pages:
                        try:
                            page_data = {
                                "id": getattr(p, "id", None),
                                "url": getattr(p, "url", None),
                                "title": getattr(p, "title", None),
                                "ws_url": getattr(p, "ws_url", None),
                                "protocol": getattr(p, "protocol", "shim"),
                            }
                            snapshot.append(page_data)
                        except Exception:
                            continue
                    try:
                        self.shared_pages[:] = snapshot
                    except Exception:
                        logger.debug("Failed to update shared_pages proxy")
            except Exception:
                logger.exception("Error in shared pages sync loop")
            self._stop_event.wait(
                self._pages_sync_interval
                if getattr(self, "_pages_sync_interval", None) is not None
                else 1.0
            )

    def _build_pages_snapshot(self) -> list:
        """
        Return a list-of-dict snapshot based on the current Runtime.pages.
        This is used to populate the Manager list proxy immediately when it is
        created so workers don't see an empty list until the background sync loop runs.
        """
        snapshot = []
        try:
            if self._runtime is not None:
                try:
                    load_fn = getattr(self._runtime, "load", None)
                    if callable(load_fn):
                        load_fn()
                except Exception:
                    logger.debug(
                        "Runtime.load() failed while building snapshot", exc_info=True
                    )

                try:
                    pages = list(self._runtime.pages)
                except Exception:
                    pages = []

                for p in pages:
                    try:
                        snapshot.append(
                            {
                                "id": getattr(p, "id", None),
                                "url": getattr(p, "url", None),
                                "title": getattr(p, "title", None),
                                "ws_url": getattr(p, "ws_url", None),
                            }
                        )
                    except Exception:
                        continue
        except Exception:
            logger.exception("Failed to build pages snapshot")
        return snapshot

    def discover(self, force: bool = False, dev: bool = False) -> None:
        """
        Discover worker specifications in the presences directory.
        """
        if not self.presences_dir.exists():
            logger.warning("Presences directory does not exist")
            return

        if force:
            logger.info("Forcing rediscovery of presence workers")
            self.workers.clear()

        for entry in self.presences_dir.iterdir():
            if not entry.is_dir():
                continue

            name = entry.name
            if name in self.workers:
                logger.debug("Worker %s already discovered, skipping", name)
                continue

            sync_result, sync_msg = sync(f"presences/{name}", str(entry))
            if not sync_result:
                if dev:
                    logger.info("Dev mode: forcing sync for %s", name)
                    sync_result, sync_msg = force_sync(f"presences/{name}", str(entry))
                    if sync_result:
                        logger.info("Presence %s: %s", name, sync_msg)
                    else:
                        logger.warning("Failed to force sync %s: %s", name, sync_msg)
                        continue
                else:
                    logger.warning("Presence %s skipped: %s", name, sync_msg)
                    continue
            else:
                logger.debug("Presence %s: %s", name, sync_msg)

            manifest = entry / "manifest.json"
            if not manifest.exists():
                logger.warning("Manifest file missing for presence: %s", name)
                continue

            specification = WorkerSpecification(
                name=name,
                path=str(entry),
                backoff_time=self.default_backoff,
                verified=sync_result,
            )
            try:
                data = json.loads(manifest.read_text(encoding="utf-8", errors="ignore"))
                specification.entrypoint = data.get("entry", specification.entrypoint)
                specification.callable_name = data.get(
                    "callable", specification.callable_name
                )
                specification.description = data.get(
                    "description", specification.description
                )
                specification.interval = data.get("interval", specification.interval)
                specification.enabled = data.get("enabled", specification.enabled)
                try:
                    setattr(specification, "web", bool(data.get("web", False)))
                except Exception:
                    pass
                specification.on_exit = data.get("on_exit", specification.on_exit)
                specification.client_id = data.get("client_id", specification.client_id)
                specification.image = data.get("image", None)
                logger.info(
                    "Discovered: %s (def %s, uses %ss)",
                    name,
                    specification.callable_name,
                    specification.interval,
                )
            except json.JSONDecodeError as exc:
                logger.error("Failed to parse manifest for presence %s: %s", name, exc)
                continue
            self.workers[name] = specification

    def get_worker(self, name: str) -> Optional[WorkerSpecification]:
        """
        Get a worker specification by name.
        """
        return self.workers.get(name)

    def list_workers(self) -> Dict[str, WorkerSpecification]:
        """
        List all discovered worker specifications.
        """
        return self.workers

    def _monitor_process(self, worker_spec: WorkerSpecification) -> None:
        """
        Monitor a worker process and log its exit.
        """
        process = worker_spec.process
        if not process:
            logger.warning(
                "No process found for monitoring worker %s", worker_spec.name
            )
            return
        logger.info("Monitoring worker %s (%s)", worker_spec.name, process.pid)
        try:
            process.join()
            exit_code = process.exitcode
            logger.info("Worker %s exited with code %s", worker_spec.name, exit_code)
        except Exception as exc:
            logger.error("Error monitoring %s: %s", worker_spec.name, exc)
            exit_code = None
        finally:
            worker_spec.process = None
            try:
                if hasattr(worker_spec, "stop_event"):
                    worker_spec.stop_event = None
            except Exception:
                pass

    def stop(self, worker_spec: WorkerSpecification) -> None:
        """
        Stop a worker based on its specification.

        Parameters:
            worker_spec (WorkerSpecification): The specification of the worker to stop.
        """
        process = worker_spec.process
        if process and process.is_alive():
            stop_event = getattr(worker_spec, "stop_event", None)
            if stop_event is not None:
                try:
                    logger.info("Signalling stop_event for worker %s", worker_spec.name)
                    stop_event.set()
                except Exception:
                    logger.exception(
                        "Failed to set stop_event for %s", worker_spec.name
                    )

                # wait briefly for the process to exit on its own
                process.join(timeout=5)
                if process.is_alive():
                    logger.warning(
                        "Worker %s did not exit after stop_event; terminating",
                        worker_spec.name,
                    )
                    try:
                        process.terminate()
                    except Exception:
                        logger.exception(
                            "Failed to terminate worker %s", worker_spec.name
                        )
                    process.join()
            else:
                try:
                    process.terminate()
                except Exception:
                    logger.exception("Failed to terminate worker %s", worker_spec.name)
                process.join()

            logger.info("Stopped worker %s", worker_spec.name)
            worker_spec.running = False
        else:
            logger.warning("Worker %s is not running", worker_spec.name)
            worker_spec.process = None
            worker_spec.running = False

    def start(self, worker_spec: WorkerSpecification) -> None:
        """
        Start a worker based on its specification.

        Parameters:
            worker_spec (WorkerSpecification): The specification of the worker to start.
        """

        if worker_spec.process and worker_spec.process.is_alive():
            logger.warning("%s is already running", worker_spec.name)
            return

        # Check if web presence requires a browser connection
        try:
            needs_browser = getattr(worker_spec, "web", False)
        except Exception:
            needs_browser = False

        if needs_browser:
            # validate that a browser is connected before starting web presence
            if not self._runtime or not self._runtime.is_connected():
                raise RuntimeError("No browser connected. Please start a browser first")
            logger.debug(
                "Browser connected, proceeding to start web presence %s",
                worker_spec.name,
            )

        stop_event = _mp.Event()
        needs_shared = needs_browser

        # init multiprocessing Manager if needed
        if self._mp_manager is None:
            try:
                self._mp_manager = _mp.Manager()
                logger.debug("Initialized multiprocessing.Manager")
            except Exception:
                logger.exception("Failed to create multiprocessing.Manager")
                self._mp_manager = None

        # initialize shared_pages for web workers
        if needs_shared and self.shared_pages is None and self._mp_manager:
            try:
                self.shared_pages = self._mp_manager.list()
                try:
                    snapshot = self._build_pages_snapshot()
                    self.shared_pages[:] = snapshot
                    logger.debug(
                        "Populated shared_pages proxy with snapshot (pages=%d)",
                        len(snapshot),
                    )
                except Exception:
                    logger.debug("Failed to populate shared_pages proxy on init")

                # start the pages sync thread (if not already running)
                if self._pages_sync_thread is None:
                    self._pages_sync_thread = threading.Thread(
                        target=self._sync_pages_loop, daemon=True
                    )
                    self._pages_sync_thread.start()
                logger.debug(
                    "PresenceManager: passing shared_pages proxy to workers (pages=%d)",
                    len(self.shared_pages),
                )
            except Exception:
                logger.exception("Failed to create shared_pages for worker")
                self.shared_pages = None

        shared_state = None
        if self._mp_manager:
            try:
                shared_state = self._mp_manager.dict()
                logger.debug(
                    "Created shared_state dict for worker %s", worker_spec.name
                )
            except Exception:
                logger.exception(
                    "Failed to create shared_state for worker %s", worker_spec.name
                )
                shared_state = {}
        else:
            shared_state = {}
        setattr(worker_spec, "shared_state", shared_state)

        arguments = (
            str(worker_spec.path),
            worker_spec.entrypoint,
            worker_spec.callable_name,
            worker_spec.interval,
            worker_spec.client_id,
            stop_event,
            self.shared_pages,
            self.steam_account,
            worker_spec.shared_state,
        )
        try:
            setattr(worker_spec, "stop_event", stop_event)
        except Exception:
            pass

        process = _mp.Process(
            target=process_worker,
            args=arguments,
            daemon=False,
        )
        worker_spec.process = process
        process.start()
        monitor = threading.Thread(
            target=self._monitor_process,
            args=(worker_spec,),
            daemon=True,
        )
        monitor.start()
        logger.info("Started worker %s with PID %d", worker_spec.name, process.pid)
        worker_spec.running = True

    def stop_all(self, only_web: bool = False) -> None:
        """
        Stop all running workers.
        """
        if only_web:
            logger.info("Stopping all web workers")
        for worker_spec in self.workers.values():
            if worker_spec.process and worker_spec.process.is_alive():
                if only_web:
                    is_web = getattr(worker_spec, "web", False)
                    if is_web:
                        self.stop(worker_spec)
                        worker_spec.running = False
                else:
                    self.stop(worker_spec)
                    worker_spec.running = False

    def start_all(self) -> None:
        """
        Start all enabled workers.
        """
        for worker_spec in self.workers.values():
            if worker_spec.enabled:
                self.start(worker_spec)
