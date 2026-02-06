import time
from typing import Any, Optional
from urllib.parse import parse_qs, urlparse
from src.logger import logger
from src.runtime import Page


def eval_page(page: Page, expression: str, timeout: float = 3.0) -> Any:
    """Evaluate JS on the page and return the result (safe wrapper)."""
    try:
        return page.evaluate(expression, return_by_value=True, timeout=timeout)
    except Exception:
        logger.debug("JS evaluation failed on page %s: %s", page.id, exc_info=True)
        return None


def extract_title(page: Page) -> Optional[str]:
    js = """(function(){
        try {
            const ytTitle = document.querySelector('h1.ytd-video-primary-info-renderer yt-formatted-string, h1 yt-formatted-string, h1.title yt-formatted-string');
            if (ytTitle && ytTitle.textContent) return ytTitle.textContent.trim();
            const h1 = document.querySelector('h1.title, h1');
            if (h1 && h1.textContent) return h1.textContent.trim();
            if (window.ytInitialPlayerResponse && ytInitialPlayerResponse.videoDetails && ytInitialPlayerResponse.videoDetails.title) {
                return ytInitialPlayerResponse.videoDetails.title;
            }
            const meta = document.querySelector('meta[name=title]');
            if (meta && meta.content) return meta.content.trim();
            if (document.title) return document.title.trim();
            return null;
        } catch(e) {
            return null;
        }
    })();"""
    return eval_page(page, js) or None


def extract_shorts_title(page: Page) -> Optional[str]:
    js = "(function(){try{const el=document.querySelector('yt-shorts-video-title-view-model h2'); if(!el) return null; return el.innerText||el.textContent||null;}catch(e){return null;}})()"
    return eval_page(page, js) or None


def extract_author(page: Page) -> Optional[str]:
    js = """(function(){
        try {
            const attributedLink = document.querySelector('#attributed-channel-name a, #upload-info yt-attributed-string a');
            if (attributedLink && attributedLink.textContent) {
                return attributedLink.textContent.trim();
            }
            const channelLink = document.querySelector('#owner ytd-video-owner-renderer a, #owner #channel-name a, ytd-channel-name a');
            if (channelLink && channelLink.textContent) return channelLink.textContent.trim();
            const ownerText = document.querySelector('#owner #text, #owner #channel-name yt-formatted-string');
            if (ownerText && ownerText.textContent) return ownerText.textContent.trim();
            if (window.ytInitialPlayerResponse && ytInitialPlayerResponse.videoDetails && ytInitialPlayerResponse.videoDetails.author) {
                return ytInitialPlayerResponse.videoDetails.author;
            }
            return null;
        } catch(e) {
            return null;
        }
    })();"""
    return eval_page(page, js) or None


def extract_shorts_author(page: Page) -> Optional[str]:
    js = "(function(){try{const el=document.querySelector('yt-reel-channel-bar-view-model a'); if(!el) return null; return el.innerText||el.textContent||null;}catch(e){return null;}})()"
    return eval_page(page, js) or None


def extract_author_url(page: Page) -> Optional[str]:
    js = "(function(){try{const el=document.querySelector('#owner #text > a')||document.querySelector('#text a'); if(!el) return null; return el.href||null;}catch(e){return null;}})()"
    return eval_page(page, js) or None


def extract_video_id(page: Page, url: Optional[str]) -> Optional[str]:
    def id_from_url(u: Optional[str]) -> Optional[str]:
        if not u:
            return None
        p = urlparse(u)
        if "youtu.be" in (p.netloc or ""):
            return p.path.lstrip("/") or None
        if p.path.startswith("/shorts/"):
            return p.path.split("/shorts/")[1].split("/")[0] or None
        q = parse_qs(p.query).get("v")
        if q:
            return q[0]
        return None

    vid = id_from_url(url)
    if vid:
        logger.debug("video_id source=url id=%s url=%s", vid, url)
        return vid

    js_ytplayer = """
    (function(){
        try {
            if (window.ytplayer && ytplayer.config && ytplayer.config.args && ytplayer.config.args.video_id) {
                return ytplayer.config.args.video_id;
            }
            return null;
        } catch(e) { return null; }
    })()
    """
    try:
        vid = eval_page(page, js_ytplayer)
    except Exception:
        vid = None
    if vid:
        logger.debug("video_id source=ytplayer id=%s", vid)
        return vid

    js_canonical = "(function(){try{const c=document.querySelector('link[rel=\"canonical\"]'); return c ? c.href : location.href;}catch(e){return null;}})()"
    try:
        canonical = eval_page(page, js_canonical)
    except Exception:
        canonical = None
    vid = id_from_url(canonical)
    if vid:
        logger.debug("video_id source=canonical href=%s id=%s", canonical, vid)
        return vid

    js_og = "(function(){try{const m=document.querySelector('meta[property=\"og:video:url\"]'); return m ? m.content : null;}catch(e){return null;}})()"
    try:
        og = eval_page(page, js_og)
    except Exception:
        og = None
    vid = id_from_url(og)
    if vid:
        logger.debug("video_id source=og id=%s og=%s", vid, og)
        return vid

    return None


def extract_thumbnail(video_id: Optional[str]) -> Optional[str]:
    if not video_id:
        return None
    return f"https://i.ytimg.com/vi/{video_id}/hqdefault.jpg"


def extract_video_times(page: Page) -> tuple[int, int]:
    js = "(function(){try{const v=document.querySelector('video'); if(!v) return {d:0,c:0}; return {d:Math.floor(v.duration||0),c:Math.floor(v.currentTime||0)};}catch(e){return {d:0,c:0};}})()"
    res = eval_page(page, js)
    if not isinstance(res, dict):
        return 0, 0
    d = int(res.get("d") or 0)
    c = int(res.get("c") or 0)
    return d, c


def extract_playback_state(page: Page) -> Optional[str]:
    js = "(function(){try{ if(navigator && navigator.mediaSession && navigator.mediaSession.playbackState) return navigator.mediaSession.playbackState; const v=document.querySelector('video'); if(!v) return null; return v.paused? 'paused' : 'playing'; }catch(e){return null;}})()"
    return eval_page(page, js) or None


def calc_time_from_now(duration: int, current: int) -> tuple[int, Optional[int]]:
    if duration <= 0:
        return int(time.time() - current), None
    start = int(time.time() - current)
    end = start + int(duration)
    return start, end
