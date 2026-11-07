// Lists the open source components inside the Windows application.
//
//   node tools/third-party.mjs
//
// Writes docs/legal/THIRD_PARTY_NOTICES.md, with each license text once, which
// the installer carries, and website/src/data/third-party.json, which the site
// shows. Run it before a release. Only what ships counts: build tools do not.
import { execFileSync, execSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));

/** Writes every license expression the same way: `MIT/Apache-2.0` is `Apache-2.0 OR MIT`. */
export function normalize(license) {
  const text = (license ?? "").trim();
  if (!text) return "Unknown";
  if (/\bAND\b|\(/.test(text)) return text;
  return [...new Set(text.split(/\s*(?:\/|\bOR\b)\s*/))].sort().join(" OR ");
}

/**
 * Groups components by license, the most used first.
 *
 * @param {{name: string, license: string}[]} components
 * @returns {{license: string, names: string[]}[]}
 */
export function groupByLicense(components) {
  const groups = new Map();
  for (const { name, license } of components) {
    const key = normalize(license);
    groups.set(key, new Set(groups.get(key)).add(name));
  }
  return [...groups]
    .map(([license, names]) => ({ license, names: [...names].sort() }))
    .sort((a, b) => b.names.length - a.names.length || a.license.localeCompare(b.license));
}

function licenseTexts(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((name) => /^(licen[sc]e|copying|notice)/i.test(name))
    .map((name) => readFileSync(join(dir, name), "utf8").trim());
}

function rust() {
  const text = execFileSync(
    "cargo",
    ["metadata", "--format-version", "1", "--filter-platform", "x86_64-pc-windows-msvc"],
    { cwd: ROOT, encoding: "utf8", maxBuffer: 256 * 1024 * 1024 },
  );
  const metadata = JSON.parse(text);
  const used = new Set(metadata.resolve.nodes.map((node) => node.id));
  return metadata.packages
    .filter((item) => used.has(item.id) && !item.name.startsWith("rpp-"))
    .map((item) => ({
      name: item.name,
      version: item.version,
      license: item.license,
      texts: licenseTexts(dirname(item.manifest_path)),
    }));
}

function interfacePackages() {
  const text = execSync("pnpm licenses list --prod --json", {
    cwd: join(ROOT, "frontend"),
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  return Object.entries(JSON.parse(text)).flatMap(([license, items]) =>
    items.flatMap((item) =>
      (item.versions ?? [""]).map((version, index) => ({
        name: item.name,
        version,
        license,
        texts: licenseTexts(item.paths?.[index] ?? item.path ?? ""),
      })),
    ),
  );
}

function notices(components) {
  const lines = [
    "# Third-party notices",
    "",
    "Rich Presence Plus includes the components below. Each keeps its own license.",
    "",
  ];
  for (const { license, names } of groupByLicense(components)) {
    lines.push(`## ${license}`, "", names.join(", "), "");
  }
  lines.push("# License texts", "");
  const seen = new Map();
  for (const component of components) {
    for (const text of component.texts) {
      const owners = seen.get(text) ?? [];
      owners.push(component.name);
      seen.set(text, owners);
    }
  }
  for (const [text, owners] of seen) {
    lines.push(`## ${[...new Set(owners)].sort().join(", ")}`, "", "```text", text, "```", "");
  }
  return lines.join("\n");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const components = [...rust(), ...interfacePackages()].sort((a, b) => a.name.localeCompare(b.name));
  writeFileSync(join(ROOT, "docs/legal/THIRD_PARTY_NOTICES.md"), notices(components));
  writeFileSync(
    join(ROOT, "website/src/data/third-party.json"),
    `${JSON.stringify(groupByLicense(components), null, 2)}\n`,
  );
  console.log(`${components.length} components`);
}
