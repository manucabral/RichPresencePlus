import { defineConfig } from "astro/config";

// The only place that says where the site lives. With a domain of its own,
// SITE_URL becomes that domain and SITE_BASE becomes "/".
export default defineConfig({
  site: process.env.SITE_URL ?? "https://manucabral.github.io",
  base: process.env.SITE_BASE ?? "/RichPresencePlus",
  trailingSlash: "always",
  compressHTML: true,
  // The style sheet is small, so it travels in the page and costs no request.
  build: { inlineStylesheets: "always" },
  // Logos stay files: the page's security policy allows images from this site only.
  vite: { build: { assetsInlineLimit: 0 } },
  markdown: {
    // Prism colors code with classes; a style on each word would double the page.
    syntaxHighlight: "prism",
  },
  i18n: {
    locales: ["en", "es"],
    defaultLocale: "en",
    routing: { prefixDefaultLocale: false },
  },
});
