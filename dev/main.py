"""
Rich Presence Plus Development Script
-------------------------------------
Development script for creating custom presences for Rich Presence Plus.

Usage:
    python main.py [-h] [-bt] [-p PORT] [-ri RUNTIME_INTERVAL]

Options:
    -bt, --browser-tools    Include browser tools for managing the browser.
    -p PORT, --port PORT    The port for start the remote debugging (default: 9222).
    -ri RUNTIME_INTERVAL, --runtime-interval RUNTIME_INTERVAL
                            The interval for updating the runtime in seconds (default: 1).
"""

__title__ = "Rich Presence Plus Development Script"
__description__ = (
    "Development script for creating custom presences for Rich Presence Plus."
)
__author__ = "Manuel Cabral"
__version__ = "0.3.0"
__license__ = "MIT"

import argparse
import time
import sys
import rpp


def browser_tools(browser: rpp.Browser, log: rpp.logger.RPPLogger, port: int) -> None:
    """
    Browser tools for managing the browser.

    Args:
        browser (rpp.Browser): The browser instance.
        log (rpp.logger.RPPLogger): The logger instance.
        port (int): The port for start the remote debugging.
    """
    log = rpp.get_logger("Browser Tools")
    log.info(f"Detected {browser.name} ({browser.process})")
    log.info("Using the default browser.")
    log.info(
        f"{browser.name} is actually {'running' if browser.running() else 'not running'}"
    )
    print("------------------")
    print("1. Force restart", browser.name)
    print("2. Close/kill", browser.name)
    print("3. Open", browser.name)
    print("4. Do nothing")
    print("------------------")
    while True:
        choice = input("Select an option: ")
        if not choice.isdigit():
            print("Invalid input.")
            continue
        choice = int(choice)
        if not choice in range(1, 5):
            print("Invalid choice.")
            continue

        log.info("Please wait...")
        if choice == 1:
            browser.kill()
            time.sleep(3)
            browser.start(remote_port=port)
            log.info("Browser restarted.")
            break
        elif choice == 2:
            browser.close()
            log.info("Browser closed.")
            break
        elif choice == 3:
            browser.start(remote_port=port)
            log.info("Browser opened.")
            break
        elif choice == 4:
            break

    input("Press Enter to continue...")


def prepare_args() -> argparse.Namespace:
    """
    Prepare the arguments.

    Returns:
        argparse.Namespace: The arguments.
    """
    parser = argparse.ArgumentParser(description=__description__, prog=__title__)
    parser.add_argument(
        "-bt",
        "--browser-tools",
        action="store_true",
        help="Include browser tools for managing the browser.",
        default=False,
    )
    parser.add_argument(
        "-p",
        "--port",
        type=int,
        help="The port for start the remote debugging (default: 9222).",
        default=9222,
    )
    parser.add_argument(
        "-ri",
        "--runtime-interval",
        type=int,
        help="The interval for updating the runtime in seconds (default: 1).",
        default=1,
    )
    parser.add_argument(
        "-v",
        "--version",
        action="version",
        version=f"{__title__} v{__version__}",
    )
    return parser.parse_args()


if __name__ == "__main__":
    args = prepare_args()
    log = rpp.get_logger("Main")
    log.info(f"{__title__} v{__version__}")
    log.info(f"Using RPP v{rpp.__version__}")

    if args.browser_tools:
        browser = rpp.Browser()
        browser_tools(browser, log, args.port)

    runtime = rpp.Runtime(port=args.port)
    manager = rpp.Manager(
        runtime=runtime, dev_mode=True, runtime_interval=args.runtime_interval
    )
    manager.load()
    manager.compare()
    manager.start()

    input("Press Enter to continue...")
    sys.exit(0)
