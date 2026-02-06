"""
Nepu Presence for Rich Presence Plus
"""

from typing import Optional, Any

from src.rpc import ActivityType, ClientRPC
from src.runtime import Runtime
from src.logger import logger

JS_EXTRACT_NEPU = r"""
(() => {
  try {
    const path = location.pathname;

    const getBgImage = (el) => {
      if (!el) return null;
      const bg = el.style.backgroundImage;
      if (!bg) return null;
      const m = bg.match(/url\(["']?(.*?)["']?\)/);
      return m ? m[1] : null;
    };

    const cover =
      getBgImage(document.querySelector(".media-cover")) || null;

    // Movie
    if (path.includes("/movie/")) {
      const h1 = document.querySelector(".caption-content h1");
      const aka = document.querySelector(".caption-content h2");

      let title = null;
      let year = null;

      if (h1?.textContent) {
        const text = h1.textContent.trim();
        const m = text.match(/^(.*?)(?:\s*\((\d{4})\))?$/);
        if (m) {
          title = m[0].trim();
          year = m[2] || null;
        }
      }

      return {
        type: "movie",
        title,
        subtitle: aka ? aka.textContent.trim() : null,
        year,
        cover
      };
    }

    // Series / Episode
    if (
      path.includes("/season/") ||
      path.includes("/episode/") ||
      path.includes("/watch")
    ) {
      const seriesH1 = document.querySelector(".caption-content h1");
      const episodeH3 = document.querySelector(".caption-content h3");
      const episodeTitle =
        document.querySelector(".caption-content h2")?.textContent?.trim() ||
        null;

      let subtitle = null;

      // Prefer exact episode text
      if (episodeH3?.textContent) {
        subtitle = episodeH3.textContent.trim();
      } else {
        // Fallback: episode-nav
        const navEpisode = document.querySelector(".episode-nav .episode");
        if (navEpisode?.textContent) {
          subtitle = navEpisode.textContent.trim();
        }
      }

      return {
        type: "episode",
        title: seriesH1 ? seriesH1.textContent.trim() : null,
        subtitle,
        episode_title: episodeTitle,
        cover
      };
    }

    return null;
  } catch (e) {
    return { __error: String(e) };
  }
})();
"""


def main(
    rpc: ClientRPC, runtime: Optional[Runtime], interval: int, stop_event: Any
) -> None:
    if runtime is None:
        raise RuntimeError("Runtime is required for Nepu presence")

    logger.info("Nepu presence started")

    last_page_id = None
    last_payload = None

    rpc.update(
        state="Browsing Nepu",
        details=None,
        activity_type=ActivityType.WATCHING,
        start_time=None,
        end_time=None,
        large_image="logo",
        large_text="Nepu",
        small_image=None,
        small_text=None,
        buttons=[],
    )

    try:
        while not stop_event.is_set():
            for page in runtime.pages:
                try:
                    if not page.url or "nepu.to" not in page.url:
                        continue

                    page.connect_if_needed()

                    data = page.evaluate(JS_EXTRACT_NEPU)
                    if not data or "__error" in data:
                        logger.debug(
                            "Failed to extract Nepu data on page %s: %s",
                            page.id,
                            data.get("__error") if data else "No data",
                        )
                        continue

                    if page.id == last_page_id and data == last_payload:
                        logger.debug("No changes detected on page %s", page.id)
                        continue

                    # Movie
                    if data["type"] == "movie":
                        title = data.get("title") or "Movie"
                        subtitle = data.get("subtitle")
                        year = data.get("year")
                        logger.info("Updating to movie=%s", title)
                        rpc.update(
                            details=title,
                            state=subtitle or (f"Released {year}" if year else None),
                            activity_type=ActivityType.WATCHING,
                            start_time=None,
                            end_time=None,
                            large_image=data.get("cover") or "logo",
                            large_text=data.get("subtitle") or "Watching on Nepu",
                            small_image="logo" if data.get("cover") else None,
                            small_text=(
                                "Watching on Nepu" if data.get("cover") else None
                            ),
                            buttons=[{"label": "Watch", "url": page.url}],
                        )

                    # Series / Episode
                    elif data["type"] == "episode":
                        series = data.get("title") or "TV Show"
                        subtitle = data.get("subtitle")
                        episode_title = data.get("episode_title")
                        logger.info(
                            "Updating to series=%s episode=%s", series, subtitle
                        )
                        rpc.update(
                            details=series,
                            state=subtitle
                            or (episode_title if episode_title else None),
                            activity_type=ActivityType.WATCHING,
                            start_time=None,
                            end_time=None,
                            large_image=data.get("cover") or "logo",
                            large_text=episode_title or "Watching on Nepu",
                            small_image="logo" if data.get("cover") else None,
                            small_text=(
                                "Watching on Nepu" if data.get("cover") else None
                            ),
                            buttons=[{"label": "Watch", "url": page.url}],
                        )

                    last_page_id = page.id
                    last_payload = data
                    break

                except Exception as exc:
                    logger.debug("Nepu presence error: %s", exc)

            stop_event.wait(interval)

    finally:
        logger.info("Nepu presence stopped")
