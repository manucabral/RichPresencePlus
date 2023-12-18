"""
This version updates the presence only when the tab is active.
Therefore, if you have multiple YouTube tabs open, it will only update the active one.
Note this is a example of how to use the runtime and the presence module.
"""

# Get the current tab
tab = runtime.current_tab

if "www.youtube.com" in tab.url:
    # Get the media session
    media_session = tab.media_session()

    # Check if the media session is active
    active = media_session and media_session.state != "none"

    if active:
        # Get the media session data
        paused = media_session.state == "paused"
        state = media_session.title
        details = media_session.artist
        large_image = media_session.artwork
        small_image = "pause" if paused else "play"
        buttons = [{"label": "Watch", "url": tab.url}]
        start = time.time()

        # Update the presence.
        presence_update(
            state=state,
            details=details,
            large_image=large_image,
            small_image=small_image,
            buttons=buttons,
            start=start,
        )

    else:
        # If the media session is not active that means the user is browsing in YouTube.
        presence_update(
            state="Browsing...",
            large_image=tab.favicon,
        )
else:
    # If the tab is not in YouTube, then the user is not watching a video.
    presence_update(state="Not watching.")

time.sleep(5)
