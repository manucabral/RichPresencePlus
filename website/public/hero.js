// Starts the examples at a point that depends on the time of the visit, so
// people see a different one first. Without this file the first one shows.
const stage = document.querySelector(".stage");
const now = Date.now();
stage.style.setProperty("--start", `${-(now % 15000)}ms`);
stage.dataset.pick = String(Math.floor(now / 5000) % 3);
