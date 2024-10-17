"""
Easy-to-use GUI for RichPresencePlus.
"""

import rpp
import app
import logging

if __name__ == "__main__":
    rpp.load_env()
    """
    # Uncomment this block to set the GitHub API token
    import os
    os.environ["GITHUB_API_TOKEN"] = (
        "your_github_api_token_here"
    )
    """
    logging.getLogger("PIL.PngImagePlugin").setLevel(logging.INFO)
    browser = rpp.Browser()
    runtime = rpp.Runtime(9222)
    manager = rpp.Manager(runtime=runtime, dev_mode=False)
    manager.run_main()
    app = app.App(manager, runtime, browser)
    app.protocol("WM_DELETE_WINDOW", app.on_exit)
    app.mainloop()
