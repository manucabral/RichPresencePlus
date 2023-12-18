"""
No comments. See rpp/tab.py.
"""


class MediaSession:
    """
    Media session data from a tab.
    """

    # pylint: disable=too-few-public-methods
    def __init__(self, **kwargs):
        """
        Create a new MediaSession object.
        """
        print(kwargs)
        self.__dict__.update(kwargs)

    def __str__(self):
        # pylint: disable=no-member
        return f"<MediaSession {self.title}>"
