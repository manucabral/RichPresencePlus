// Shows what /v1/now answers. Text goes in as text, and only https pictures load.
const part = (id) => document.getElementById(id);
const card = part("card");
const picture = part("picture");

async function look() {
  try {
    const now = await (await fetch("/v1/now")).json();
    card.hidden = !now.showing;
    part("presence").textContent = now.presence ?? "";
    part("details").textContent = now.details ?? "";
    part("state").textContent = now.state ?? "";
    const image = now.large_image ?? "";
    picture.hidden = !image.startsWith("https://");
    if (!picture.hidden && picture.src !== image) picture.src = image;
  } catch {
    card.hidden = true;
  }
}

look();
setInterval(look, 2000);
