---
title: Leer un sitio
description: Encontrá los selectores, frames, listas y datos ocultos que necesita una presencia web, y llevalos a tu script.
---

# Leer un sitio

Tu script nunca corre dentro de la página. La extensión lee los valores que nombrás en `page`
del manifiesto y se los pasa a `read` en `page.values`. Esta guía muestra cómo encontrar cada
tipo de valor. Todos los extractores y sus campos están en [Extractores](09-extractors.md).

## Encontrar un selector

1. Abrí el sitio, hacé clic derecho en el elemento y elegí Inspeccionar.
2. En las herramientas, hacé clic derecho en su línea y elegí Copiar, Copiar selector.
3. Acortá lo que copiaste. `h1.video-title` aguanta un rediseño mejor que
   `#app > div:nth-child(3) > h1`.

```json
"title": { "from": "text", "selector": "h1.video-title" }
```

Para probar un selector antes de guardar, escribí `document.querySelector("h1.video-title")` en
la Consola del navegador.

Los sitios cambian, así que cualquier valor puede llegar vacío. Dale a cada uno un respaldo en
`read`:

```js
const title = page.values.title || page.title;
```

## Leer un reproductor dentro de un frame

Muchos sitios ponen su reproductor en un iframe de otro sitio, y un selector no puede entrar
ahí. `page.frames` dice qué frames leer y qué tomar de ellos:

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

`player` llega a `page.values` junto a `title`, así que el script no cambia. Para encontrar la
dirección del frame, hacé clic derecho en el reproductor, elegí Inspeccionar y buscá el
`<iframe>` que lo rodea. `presences/dailymotion` lee su reproductor así.

## Leer varios elementos

`list` lee todas las coincidencias:

```json
"genres": { "from": "list", "selector": "a.genre", "max": 3 }
```

```js
const genres = page.values.genres;   // ["Acción", "Drama", "Comedia"]
const state = genres.join(", ");     // "Acción, Drama, Comedia"
```

## Leer datos que la página no muestra

El texto en pantalla muchas veces no es la mejor fuente. Los sitios guardan datos más limpios en
otros lugares, y una presencia puede leerlos sin correr nada en la página.

| Dónde está el dato | Cómo encontrarlo | Extractor |
| --- | --- | --- |
| Una variable global | Escribí `window.` en la Consola y explorá. | `{ "from": "variable", "path": "app.video.episode" }` |
| JSON en una etiqueta script | Buscá `__NEXT_DATA__` o `application/ld+json` en el código fuente. | `{ "from": "json", "selector": "script#__NEXT_DATA__", "path": "props.pageProps.title" }` |
| Guardado por el sitio | Mirá la pestaña Aplicación de las herramientas del navegador. | `{ "from": "storage", "key": "player-state", "path": "track.title" }` |
| Una imagen de fondo | Inspeccioná el elemento y buscá `background-image`. | `{ "from": "style", "selector": ".poster", "name": "background-image" }` |
| Dentro de un componente web | Escribí `>>>` para entrar. | `{ "from": "text", "selector": "video-player >>> h2" }` |
| Los controles de medios del sitio | Los sitios de música suelen llenarlos. | `{ "from": "session" }` |
| Una respuesta que la página pide | Filtrá la pestaña Red por Fetch/XHR. | `{ "from": "response", "url": "*://api.example.com/v1/*", "path": "data.title" }` |

Un nombre que contiene `token`, `password`, `secret`, `cookie` y parecidos se rechaza, así que
una presencia no puede leer un inicio de sesión. Leé solo lo que mostrás: quienes revisan miran
cada ruta.

## Leer un sitio que cada uno instala

Algunos servicios no tienen una dirección fija: cada persona los instala en una computadora o un
servidor de su casa. Escribí `"user_sites": true` y no pongas `matches`, o dejá en él un sitio
de prueba:

```json
"page": {
  "user_sites": true,
  "extract": {
    "title": { "from": "response", "url": "/api/now-playing", "path": "item.name" },
    "player": { "from": "media" }
  }
}
```

Entonces la página de la presencia en la aplicación tiene "Tus sitios", donde la persona escribe
la dirección de su servidor, como `media.home:8096`. La extensión pregunta antes de leerlo, como
con cualquier sitio. Una dirección de respuesta que empieza con `/` se lee en el sitio de la
página, sea cual sea.

## Empezar sin código

Abrí la extensión en un sitio y tocá "Crear presencia". Hacé clic en los textos que quieras y
ponele un nombre. La aplicación escribe `matches` y `extract` por vos, y podés seguir editando
desde ahí.
