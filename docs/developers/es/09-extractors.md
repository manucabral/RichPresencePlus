---
title: Extractores
description: Cada forma en que una presencia web puede leer una página, con sus campos y lo que le da a tu script.
---

# Extractores

Un extractor lee un valor de una página. Cada entrada de `page.extract` o de
`page.frames.extract` es un extractor, y su valor llega a `read` en `page.values` con el mismo
nombre. Un valor que no se puede leer llega como `null`. Los textos llegan con los espacios
unidos, hasta 512 caracteres, y una lista u objeto de la página hasta 4 KB.

```json
"extract": {
  "title": { "from": "text", "selector": "h1" },
  "player": { "from": "media", "selector": "video" }
}
```

## Resumen

| `from` | Campos | Da |
| --- | --- | --- |
| `text` | `selector` | Texto |
| `attr` | `selector`, `name` | Texto |
| `list` | `selector`, `name` y `max` opcionales | Lista de textos |
| `meta` | `name` | Texto |
| `media` | `selector` opcional | `{ paused, current, duration, rate }` |
| `session` | | `{ title, artist, album, artwork, state }` |
| `variable` | `path` | Texto, número, booleano, o una lista u objeto chico |
| `json` | `selector`, `path` opcional | Igual que `variable` |
| `storage` | `key`, `area` y `path` opcionales | Igual que `variable` |
| `style` | `selector`, `name` | Texto |
| `response` | `url`, `path` opcional | Igual que `variable` |

Un selector es CSS, de hasta 256 caracteres. Escribí `>>>` para entrar en un componente web,
como en `video-player >>> h2`. Una ruta son nombres unidos por puntos, como `video.title`: hasta
128 caracteres y 8 nombres. Solo puede leer, nunca llamar.

## text

El texto visible del primer elemento que coincide con `selector`.

```json
"title": { "from": "text", "selector": "h1.video-title" }
```

## attr

El atributo `name` del primer elemento que coincide con `selector`. `href` y `src` llegan como
direcciones completas, aunque la página las escriba relativas.

```json
"channel_url": { "from": "attr", "selector": "a.channel-name", "name": "href" }
```

## list

Todos los elementos que coinciden, como una lista de textos o del atributo `name`. `max` la
limita: 16 por defecto, 32 como máximo.

```json
"genres": { "from": "list", "selector": "a.genre", "max": 3 }
```

## meta

El contenido de la etiqueta `<meta>` cuyo `name` o `property` es `name`.

```json
"image": { "from": "meta", "name": "og:image" }
```

## media

El estado de un `<video>` o `<audio>` que coincide con `selector`, o de cualquiera si no hay
selector. Si coinciden varios, elige uno que suena con audio, después uno con audio, después uno
que se reproduce. `current` y `duration` son segundos; `duration` es `null` mientras no se conoce.
Pasalo a [`playback`](12-helpers.md#playbackplayer-receivedat) para la barra de progreso.

```json
"player": { "from": "media", "selector": "video" }
```

## session

Lo que el sitio le da a los controles de medios del navegador: `title`, `artist`, `album`,
`artwork` (una dirección) y `state`. Los sitios de música suelen llenarlo, así que una presencia
puede no necesitar ningún selector.

```json
"song": { "from": "session" }
```

## variable

Lo que la página guarda en una variable global, por su `path`.

```json
"episode": { "from": "variable", "path": "app.video.episode" }
```

## json

JSON escrito en el elemento que coincide con `selector`, como `script#__NEXT_DATA__`. Con
`path`, solo esa parte.

```json
"title": { "from": "json", "selector": "script#__NEXT_DATA__", "path": "props.pageProps.title" }
```

## storage

Una entrada de `localStorage` por su `key`, o de `sessionStorage` con `"area": "session"`. Con
`path`, solo esa parte de su JSON.

```json
"song": { "from": "storage", "key": "player-state", "path": "track.title" }
```

## style

Una propiedad CSS calculada del primer elemento que coincide con `selector`, por su `name`.

```json
"cover": { "from": "style", "selector": ".poster", "name": "background-image" }
```

## response

La última respuesta JSON que la página recibió de una dirección que coincide con `url`, la haya
pedido con `fetch` o con `XMLHttpRequest`. Con `path`, solo esa parte. `url` es un patrón que
nombra un sitio, como `*://api.example.com/v1/player/*`, o una ruta que empieza con `/` en el
sitio de la misma página; su ruta también abarca la consulta.

```json
"title": { "from": "response", "url": "*://api.example.com/v1/player/*", "path": "data.title" }
```

La página recibe su respuesta como siempre; la extensión solo mira una copia. Las respuestas de
más de 1 MB se dejan pasar sin leer. Una respuesta que llega antes de que la extensión arranque
en la página igual se lee, y una pestaña que ya estaba abierta antes de activar la presencia
muestra solo las respuestas que llegan después. Una presencia lee hasta 8 direcciones, y solo
en sus páginas, no en frames.

Para encontrar la dirección, abrí la pestaña Red de las herramientas del navegador, filtrá por
Fetch/XHR, y reproducí o abrí lo que la presencia muestra.

## Lo que no se puede leer

Una ruta o clave que contiene `token`, `password`, `secret`, `cookie` y parecidos se rechaza al
cargar la presencia, así que una presencia no puede leer un inicio de sesión. Tampoco una
dirección que contiene esas palabras, `login`, `oauth` o `auth/`. Un frame se lee
solo dentro de las páginas de su propia presencia.
