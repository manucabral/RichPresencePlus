// __NAME__: a web presence.
// Replace `page` in manifest.json with the site and the elements to read.

function read(context) {
  const page = bestPage(context.pages);
  if (!page) return null;

  const url = parseUrl(page.url);
  if (!url) return null;

  // The extractor named "title" in manifest.json gives the heading of the page.
  const title = page.values.title || page.title;
  return { host: url.host, title: context.options.include_page_title ? title : url.host };
}

function activity(site) {
  return {
    name: "__NAME__",
    details: cut(site.title) || t("browsing"),
    state: site.host,
    kind: "watching",
  };
}
