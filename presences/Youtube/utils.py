import enum


class Query(enum.Enum):
    VIDEO_STREAM = 'Array.from(document.querySelectorAll(".video-stream")).find(element => element.duration)'
    VIDEO_TITLE = 'document.querySelector("#title > h1 > yt-formatted-string")'
    SHORT_AUTHOR = 'document.querySelectorAll("#channel-info #channel-name a")[document.querySelectorAll("#channel-info #channel-name a").length - 1]'
