"""
Development script for create presences.
"""
import sys
import argparse
import rpp

VERSION = "0.0.1"
NAME = "Rich Presence Plus Development Script"


def parse_args() -> argparse.Namespace:
    """Parse the arguments."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--allow-web-presences", default=False, action="store_true")
    return parser.parse_args()


if __name__ == "__main__":
    args = parse_args()

    rpp.log(NAME, dev_mode=True)
    rpp.log(f"Using v{VERSION}", dev_mode=True)

    if args.allow_web_presences:
        rpp.log("Web presences are allowed.", dev_mode=True)
        runtime = rpp.Runtime()
        res = runtime.connect()
        if not res:
            rpp.log("Cannot connect to the browser.", dev_mode=True)
            input("Press enter to continue...")
            sys.exit(1)

    # load presences metadata
    presences_metadata = rpp.utils.load_local_presences_metadata(dev_mode=True)

    # create presences from metadata
    presences = []
    for presence_metadata in presences_metadata:
        # exclude web presences if they are not allowed
        if presence_metadata["use_browser"] and not args.allow_web_presences:
            rpp.log(
                f"Skipping {presence_metadata['name']} because web presences are not allowed.",
                dev_mode=True,
            )
            continue

        presence = rpp.Presence(**presence_metadata)
        presence.enabled = True
        presences.append(presence)

    total = len(presences)
    rpp.log(f"Succesfully loaded {total} presences.", dev_mode=True)

    if total == 0:
        rpp.log("No presences to run.", dev_mode=True)
        input("Press enter to continue...")
        sys.exit(0)

    try:
        for presence in presences:
            if presence.metadata["use_browser"]:
                presence.runtime = runtime
            presence.start()
        while True:
            pass
    except KeyboardInterrupt:
        rpp.log("Stopped by user.")
        for presence in presences:
            presence.stop()
