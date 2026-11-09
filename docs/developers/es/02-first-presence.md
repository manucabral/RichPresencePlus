---
title: Tu primera presencia
description: Armá en unos diez minutos una presencia que funciona para una página de videos, y mirá cómo cambia cada vez que guardás.
---

# Tu primera presencia

En este tutorial armás una presencia para una página de videos. Al terminar, la tarjeta muestra
el título y el canal del video, dos botones y una barra de progreso que se detiene cuando
pausás.

Vas a usar una página de práctica hecha para este tutorial. No cambia nunca, así que cada paso
funciona tal como está escrito:
[manucabral.github.io/RichPresencePlus/es/practice](https://manucabral.github.io/RichPresencePlus/es/practice/).

## 1. Crear la carpeta

1. Abrí la aplicación y andá a la página Desarrollo.
2. En "Crear una presencia", escribí `Mi Video`.
3. Dejá la plantilla en "Página web" y tocá Crear.

La aplicación escribe una presencia que ya funciona y abre la pestaña Programar. Tocá "Abrir en
VS Code", o "Abrir carpeta" si usás otro editor.

## 2. Verla en vivo

1. Abrí la pestaña Probar, elegí Mi Video y tocá Vigilar.
2. Dejá esa ventana donde la puedas ver.

Desde ahora, cada vez que guardás un archivo la aplicación carga la versión nueva en menos de
un segundo. La pestaña Probar muestra lo que devolvió `read`, la actividad, tus líneas de
`console.log` y una vista previa de la tarjeta.

## 3. Decir qué página leer

Abrí `manifest.json` y reemplazá las entradas `page` y `domains` por estas:

```json
"domains": ["manucabral.github.io"],
"page": {
  "matches": ["*://manucabral.github.io/*practice/*"],
  "extract": {
    "title": { "from": "text", "selector": "h1.video-title" },
    "channel": { "from": "text", "selector": "a.channel-name" },
    "channel_url": { "from": "attr", "selector": "a.channel-name", "name": "href" },
    "player": { "from": "media", "selector": "video" }
  }
}
```

Cambiá también `"kind": "other"` por `"kind": "video"`, borrá la entrada `options`, que esta
presencia no usa, y guardá.

`matches` dice qué direcciones son de esta presencia. Cada entrada de `extract` lee un valor de
la página, con un nombre que elegís vos: un texto, un atributo y el estado del video.

Ahora abrí la página de práctica en tu navegador. La extensión pregunta antes de leer un sitio
nuevo: abrí la extensión y tocá "Permitir este sitio".

La pestaña Probar ya muestra los cuatro valores debajo de `read`. Tocá reproducir en la página
de práctica y mirá cómo cambia `player`.

## 4. Juntar lo que necesitás

Reemplazá todo `main.js` por esto:

```js
function read(context) {
  const page = bestPage(context.pages);
  if (!page) return null;

  const values = page.values;
  return {
    title: values.title || page.title,
    channel: values.channel,
    channel_url: values.channel_url,
    url: page.url,
    ...playback(values.player, page.received_at),
  };
}
```

Guardá. La pestaña Probar muestra el objeto, con tres campos nuevos: `playing`, `start` y `end`.

`bestPage` elige la pestaña correcta cuando coinciden varias. `playback` convierte el estado del
video en esos tres campos, y `...` los suma al objeto. Cuando la página se cierra, `read`
devuelve `null` y la tarjeta desaparece.

## 5. Mostrarlo en la tarjeta

Agregá esto debajo de `read`:

```js
function activity(video) {
  const buttons = [{ label: t("watch"), url: video.url }];
  if (video.channel_url) buttons.push({ label: t("channel"), url: video.channel_url });

  return {
    name: "Mi Video",
    details: cut(video.title),
    state: video.channel || t("watching"),
    kind: "watching",
    start: video.start,
    end: video.end,
    buttons,
  };
}
```

Después abrí `locales/en.json` y reemplazalo por los textos que pide el script:

```json
{
  "description": "Shows the video you are watching.",
  "watch": "Watch",
  "channel": "Channel",
  "watching": "Watching"
}
```

Guardá los dos archivos. La vista previa muestra el nombre, el título, el canal, una barra de
progreso y dos botones. `name` reemplaza el nombre de la aplicación de Discord compartida. Con
Discord abierto, tu perfil muestra la misma tarjeta. Para verla en español, copiá el archivo a
`locales/es.json` y traducí los valores.

## 6. Dejar lugar cuando está en pausa

Agregá una función más:

```js
function meta(video) {
  return { paused: !video.playing };
}
```

Guardá y pausá el video. La barra de progreso desaparece, porque `start` y `end` quedan vacíos
en pausa, y la tarjeta se oculta en la vuelta siguiente. Si mientras tanto suena música en otro
lado, esa presencia ocupa la tarjeta.

## Lo que armaste

`main.js` tiene ahora tres funciones, y la aplicación las llama en orden: `read` junta,
`activity` presenta y `meta` informa. Este es el archivo completo:

```js
function read(context) {
  const page = bestPage(context.pages);
  if (!page) return null;

  const values = page.values;
  return {
    title: values.title || page.title,
    channel: values.channel,
    channel_url: values.channel_url,
    url: page.url,
    ...playback(values.player, page.received_at),
  };
}

function activity(video) {
  const buttons = [{ label: t("watch"), url: video.url }];
  if (video.channel_url) buttons.push({ label: t("channel"), url: video.channel_url });

  return {
    name: "Mi Video",
    details: cut(video.title),
    state: video.channel || t("watching"),
    kind: "watching",
    start: video.start,
    end: video.end,
    buttons,
  };
}

function meta(video) {
  return { paused: !video.playing };
}
```

Para hacer lo mismo en un sitio real, cambiá `matches` y los selectores: [Leer un sitio](03-read-a-site.md)
muestra cómo encontrarlos. `presences/youtube`, en el repositorio, es esta misma presencia para
un sitio real.
