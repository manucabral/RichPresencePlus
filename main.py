"""
Main entry point for the application.
"""

import os
import socket
import threading
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler
from pathlib import Path
import webview
from dotenv import load_dotenv

env_path = Path(__file__).parent / ".env"
load_dotenv(dotenv_path=env_path)

from src.api import RPPApi
from src.constants import config
from src.browser_manager import BrowserManager
from src.presence_manager import PresenceManager
from src.runtime import Runtime
from src.user import get_user_settings
from src.logger import logger, set_log_level


def serve_dist(directory: Path, port: int) -> None:
    """Serve static files from the given directory on the specified port."""
    handler_class = SimpleHTTPRequestHandler
    prev_cwd = Path.cwd()
    try:
        os.chdir(str(directory))
        httpd = ThreadingHTTPServer(("127.0.0.1", port), handler_class)
        logger.info("Static server serving %s at http://127.0.0.1:%d", directory, port)
        httpd.serve_forever()
    finally:
        os.chdir(str(prev_cwd))


def find_free_port() -> int:
    """Find a free port on localhost."""
    socket_obj = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    socket_obj.bind(("127.0.0.1", 0))
    port = socket_obj.getsockname()[1]
    socket_obj.close()
    return port


def start():
    """Start the main application and webview window."""
    logger.info("Starting backend...")
    user_settings = get_user_settings()
    set_log_level(user_settings.logs_level)
    rt = Runtime(
        origin="user",
        interval=user_settings.runtime_interval,
        port=user_settings.browser_target_port,
    )
    bm = BrowserManager(
        profile_name=user_settings.profile_name,
        target_port=user_settings.browser_target_port,
    )
    pm = presence_manager = PresenceManager(runtime=rt)

    def load_managers() -> None:
        """Load browser and presence managers in background."""
        bm.load()
        presence_manager.discover(force=True, dev=config.development_mode)
        rt.load(True)

    background_thread = threading.Thread(target=load_managers, daemon=True)
    background_thread.start()
    logger.info("Initializing API...")
    api = RPPApi(
        browser_manager=bm, presence_manager=pm, runtime=rt, user_settings=user_settings
    )

    try:
        logger.info(
            "Starting webview in %s mode (%dx%dpx)...",
            "development" if config.development_mode else "production",
            config.window_width,
            config.window_height,
        )

        if config.development_mode:
            logger.info(
                "Dev mode: connecting to server at %s", config.frontend_dev_server_url
            )
            webview.create_window(
                title=config.title,
                url=config.frontend_dev_server_url,
                js_api=api,
                width=config.window_width,
                height=config.window_height,
            )
            webview.start(debug=True)
            return

        dist_dir = Path(config.frontend_dir)
        if not dist_dir.exists():
            raise RuntimeError(f"Frontend dist directory not found: {dist_dir}")

        port = find_free_port()
        t = threading.Thread(target=serve_dist, args=(dist_dir, port), daemon=True)
        t.start()
        logger.info("Started static file server thread on port %d", port)

        url = f"http://127.0.0.1:{port}/index.html"
        webview.create_window(
            title=config.title,
            url=url,
            js_api=api,
            width=config.window_width,
            height=config.window_height,
        )
        webview.start(debug=False)

    finally:
        logger.info("Shutting down...")
        try:
            pm.stop_all()
        except Exception as exc:
            logger.error("Error stopping presences: %s", exc)


if __name__ == "__main__":
    logger.info("Starting application...")
    start()
