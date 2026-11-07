---
title: Presencias web
description: Armá paso a paso una presencia para un sitio web, leyendo la página con la extensión y mostrando el título, un botón y la barra de progreso.
---

# Presencias web

Una presencia web muestra lo que hacés en un sitio. Tu script no corre dentro de la página. La
extensión lee los valores que nombrás en el manifiesto y se los pasa a tu script, así que una
página nunca puede ejecutar tu código y tu código nunca toca la página.

Esta página arma una presencia para un sitio de videos, de a una parte.

## 1. Decir qué páginas leer

`matches` lista las direcciones donde funciona la presencia. `*` coincide con cualquier cosa.

```json
"page": {
  "matches": ["*://www.example.com/watch*"],
  "extract": {}
}
```

## 2. Decir qué leer

Cada entrada de `extract` toma un valor de la página, con un nombre que elegís vos.

```json
"extract": {
  "title": { "from": "text", "selector": "h1.video-title" },
  "channel": { "from": "text", "selector": "a.channel-name" },
  "channel_url": { "from": "attr", "selector": "a.channel-name", "name": "href" },
  "player": { "from": "media", "selector": "video" }
}
```

Para encontrar un selector, abrí el sitio, hacé clic derecho en el elemento y elegí Inspeccionar.
En las herramientas, hacé clic derecho en su línea y elegí Copiar, Copiar selector.

| `from` | Qué te da |
| --- | --- |
| `text` | El texto visible del primer elemento que coincide con `selector`. |
| `attr` | El atributo `name` de ese elemento, como el `href` de un enlace. |
| `meta` | El contenido de la etiqueta `<meta>` con ese `name` o `property`. |
| `media` | `{ paused, current, duration, rate }` de un `<video>` o `<audio>`. |
| `session` | `{ title, artist, album, artwork, state }` del Media Session del sitio. |
| `list` | Todos los elementos que coinciden, como lista. Agregá `name` para leer un atributo, y `max` para un límite (16 por defecto, 32 como máximo). |
| `variable` | Lo que la página guarda en una variable global, por su `path`. |
| `json` | JSON escrito en un elemento, opcionalmente por su `path`. |
| `storage` | Una entrada de `localStorage`, o de `sessionStorage` con `"area": "session"`, por su `key`. |
| `style` | Una propiedad CSS calculada de un elemento, por su `name`. |

## 3. Leer los valores

Los valores llegan en `page.values`, con los nombres que elegiste. Los sitios cambian, así que
cualquier valor puede venir vacío: usá una alternativa.

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

`bestPage` elige cuando coinciden varias pestañas, y prefiere la que está reproduciendo. Busca el
valor llamado `player`, así que usá ese nombre para tu extractor `media`. `playback` convierte
ese valor en `playing`, `start` y `end`, y los `...` suman esos tres al objeto.

## 4. Mostrarlo

```js
function activity(video) {
  const buttons = [{ label: t("watch"), url: video.url }];
  if (video.channel_url) buttons.push({ label: t("channel"), url: video.channel_url });

  return {
    details: cut(video.title),
    state: video.channel || t("watching"),
    kind: "watching",
    start: video.start,
    end: video.end,
    buttons,
  };
}
```

`cut` mantiene los títulos largos dentro del límite de Discord. `start` y `end` dibujan la barra
de progreso, y vienen vacíos en pausa, así que la barra se va sola. El botón del canal se agrega
solo si existe el enlace.

## 5. Avisar cuando está en pausa

```js
function meta(video) {
  return { paused: !video.playing };
}
```

Con esto, una canción que suena en otro lado le gana a tu video en pausa.

## 6. Agregar los textos y probar

```json
{
  "description": "Shows the video you are watching.",
  "watch": "Watch",
  "channel": "Channel",
  "watching": "Watching"
}
```

Guardá, tocá Vigilar en la página Desarrollo y abrí un video. Si la vista previa sigue vacía, el
sitio necesita permiso: abrí la extensión y tocá "Permitir este sitio".

## Cuando el video está en un iframe

Muchos sitios ponen su reproductor en un iframe de otro sitio. Un selector no llega adentro, así
que `page.frames` dice qué iframes leer y qué tomar de ellos:

```json
"page": {
  "matches": ["*://www.example.com/watch*"],
  "extract": {
    "title": { "from": "text", "selector": "h1" }
  },
  "frames": {
    "matches": ["*://player.example.net/*"],
    "extract": {
      "player": { "from": "media" }
    }
  }
}
```

Tu script no cambia: `player` llega en `page.values` junto a `title`, y `bestPage` y `playback`
funcionan igual.

Para encontrar la dirección del iframe, hacé clic derecho en el reproductor, elegí Inspeccionar
y buscá el `<iframe>` que lo rodea. Cada patrón tiene que nombrar un sitio, así que `*://*/*` no
se acepta. La gente da permiso a la página y a sus iframes con un solo clic. Un iframe solo se lee
dentro de las páginas de su presencia, y los que no coinciden no cuestan nada.

## Listas

`list` lee todas las coincidencias, algo útil cuando una página tiene varios elementos iguales:

```json
"genres": { "from": "list", "selector": "a.genre", "max": 3 }
```

```js
const genres = page.values.genres;         // ["Acción", "Drama", "Comedia"]
const state = genres.join(", ");           // "Acción, Drama, Comedia"
```

## Datos que la página no muestra

El texto en pantalla muchas veces no es la mejor fuente. Los sitios guardan datos más limpios en
otros lugares, y una presencia puede leerlos todos sin ejecutar nada dentro de la página.

**Variables de la página.** Para ver qué tiene un sitio, abrí la Consola del navegador y escribí
`window.` para explorarlo. Después pedí la ruta. Da un texto, un número, un booleano, o una lista
u objeto chicos.

```json
"episode": { "from": "variable", "path": "app.video.episode" }
```

**JSON en la página.** Muchos sitios escriben sus datos en una etiqueta script. Buscá en el código
de la página `__NEXT_DATA__` o `application/ld+json`.

```json
"title": { "from": "json", "selector": "script#__NEXT_DATA__", "path": "props.pageProps.title" }
```

**Almacenamiento.** Los reproductores de música suelen guardar la canción actual. Mirá la pestaña
Aplicación de las herramientas del navegador.

```json
"song": { "from": "storage", "key": "player-state", "path": "track.title" }
```

**Estilos.** Algunos sitios ponen una imagen como fondo en vez de usar una etiqueta de imagen.

```json
"cover": { "from": "style", "selector": ".poster", "name": "background-image" }
```

**Componentes.** Un selector no ve dentro de un componente web. Escribí `>>>` para entrar:

```json
"title": { "from": "text", "selector": "video-player >>> h2" }
```

## Qué no puede leer una presencia

Una ruta son nombres con puntos, así que no puede llamar a nada. Un nombre que contiene `token`,
`password`, `secret`, `cookie` y similares se rechaza al cargar la presencia, así que una presencia
no puede leer un inicio de sesión. Leé solo lo que mostrás: la revisión mira cada ruta.

## Un caso real

`presences/youtube` es esta misma presencia para un sitio real, y `presences/youtube-music` lee el
valor `session` en vez de selectores. Las dos están en el repositorio.

## Sin código

Para empezar más rápido, abrí la extensión en un sitio y tocá "Crear presencia". Hacé clic en los
textos que quieras y ponele un nombre. La aplicación escribe `matches` y `extract` por vos, y
podés seguir editando desde ahí.
