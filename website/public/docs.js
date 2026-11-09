// Search and copy buttons for the documentation. Without this file every page still
// reads and links the same; only these two helpers are missing.
const search = document.querySelector(".docs-search");
const box = search.querySelector("input");
const results = search.querySelector(".docs-results");
let entries = null;

// The index is fetched only when someone starts typing.
async function load() {
  if (!entries) {
    const answer = await fetch(search.dataset.index);
    entries = answer.ok ? await answer.json() : [];
  }
  return entries;
}

// Accents and case do not count, so "funcion" finds "función".
const fold = (text) =>
  text
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase();

function item(text, page, href) {
  const li = document.createElement("li");
  const link = document.createElement("a");
  link.href = href;
  link.textContent = text;
  if (page) {
    const small = document.createElement("small");
    small.textContent = page;
    link.append(small);
  }
  li.append(link);
  return li;
}

async function show() {
  const words = fold(box.value).split(/\s+/).filter(Boolean);
  if (words.length === 0) {
    results.hidden = true;
    return;
  }
  const all = await load();
  const fits = (text) => words.every((word) => text.includes(word));
  // A heading that matches by itself goes before one that matches through its page.
  const own = all.filter((entry) => fits(fold(entry.text)));
  const through = all.filter((entry) => !own.includes(entry) && fits(fold(`${entry.text} ${entry.page}`)));
  const found = [...own, ...through].slice(0, 8);
  results.replaceChildren(
    ...found.map((entry) => item(entry.text, entry.page, `${search.dataset.base}${entry.path}`)),
  );
  if (found.length === 0) {
    const li = document.createElement("li");
    li.className = "none";
    li.textContent = search.dataset.none;
    results.append(li);
  }
  results.hidden = false;
}

search.hidden = false;
box.addEventListener("input", show);
box.addEventListener("keydown", (event) => {
  if (event.key === "Enter") {
    const first = results.querySelector("a");
    if (first) location.href = first.href;
  }
  if (event.key === "Escape") {
    box.value = "";
    results.hidden = true;
  }
});

for (const block of document.querySelectorAll(".docs-body pre")) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "copy";
  button.textContent = search.dataset.copy;
  button.addEventListener("click", async () => {
    await navigator.clipboard.writeText(block.querySelector("code")?.innerText ?? block.innerText);
    button.textContent = search.dataset.copied;
    setTimeout(() => {
      button.textContent = search.dataset.copy;
    }, 1500);
  });
  block.append(button);
}
