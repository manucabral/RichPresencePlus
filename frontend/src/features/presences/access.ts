import { t } from "../../shared/i18n";
import { siteNames } from "../../shared/presence";
import type { Access } from "../../shared/types";

/** Says in words what a presence may read or reach, one line per capability. */
export function accessLines(access: Access): string[] {
  const lines: string[] = [];
  const sites = siteNames(access.sites);
  if (sites.includes("*")) lines.push(t("access.anySite"));
  else if (sites.length) lines.push(t("access.sites", { sites: sites.join(", ") }));
  if (access.http.length) lines.push(t("access.http", { hosts: access.http.join(", ") }));
  if (access.process) lines.push(t("access.process"));
  for (const [program, flags] of Object.entries(access.args)) {
    lines.push(t("access.args", { program, flags: flags.join(" ") }));
  }
  if (access.window) lines.push(t("access.window"));
  if (access.media) lines.push(t("access.media"));
  if (access.query.length) lines.push(t("access.query", { protocols: access.query.join(", ") }));
  if (access.files.length) lines.push(t("access.files", { paths: access.files.join(", ") }));
  return lines;
}
