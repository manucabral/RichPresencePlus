import assert from "node:assert/strict";
import { test } from "node:test";
import { linkParts } from "../src/shared/links.ts";

test("known addresses become links and the rest stays text", () => {
  assert.deepEqual(linkParts("Create one at discord.com/developers. Its name shows."), [
    { text: "Create one at ", target: null },
    { text: "discord.com/developers", target: "developers" },
    { text: ". Its name shows.", target: null },
  ]);
  assert.deepEqual(linkParts("Learn in the developer documentation."), [
    { text: "Learn in the ", target: null },
    { text: "developer documentation", target: "docs" },
    { text: ".", target: null },
  ]);
  assert.deepEqual(linkParts("Aprendé en la documentación para desarrolladores."), [
    { text: "Aprendé en la ", target: null },
    { text: "documentación para desarrolladores", target: "docs-es" },
    { text: ".", target: null },
  ]);
});

test("a text with no address has no link", () => {
  assert.deepEqual(linkParts("Nothing here, like example.com"), [{ text: "Nothing here, like example.com", target: null }]);
  assert.deepEqual(linkParts(""), []);
});

test("asking for a presence opens the issues", () => {
  const parts = linkParts("Ask for it at github.com/manucabral/RichPresencePlus/issues.");
  assert.deepEqual(parts[1], { text: "github.com/manucabral/RichPresencePlus/issues", target: "issues" });
});
