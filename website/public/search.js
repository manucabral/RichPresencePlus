// Filters the list of presences. Without this file the whole list still shows.
const bar = document.querySelector(".search");
const box = bar.querySelector("input");
const chips = [...bar.querySelectorAll("button")];
const rows = [...document.querySelectorAll("[data-name]")];
const groups = [...document.querySelectorAll("[data-group]")];
const empty = document.querySelector(".search-empty");
let category = "";

function apply() {
  const text = box.value.trim().toLowerCase();
  let shown = 0;
  for (const row of rows) {
    const fits = row.dataset.name.includes(text) && (category === "" || row.dataset.category === category);
    row.hidden = !fits;
    if (fits) shown += 1;
  }
  for (const group of groups) {
    group.hidden = !group.querySelector("[data-name]:not([hidden])");
  }
  empty.hidden = shown > 0;
}

for (const chip of chips) {
  chip.addEventListener("click", () => {
    category = chip.dataset.value;
    for (const other of chips) other.setAttribute("aria-pressed", String(other === chip));
    apply();
  });
}
box.addEventListener("input", apply);
bar.hidden = false;
