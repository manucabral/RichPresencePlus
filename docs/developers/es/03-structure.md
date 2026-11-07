---
title: Estructura
description: Qué hace cada archivo de una presencia de Rich Presence Plus, cómo funciona el manifiesto y cómo agregar opciones y traducciones.
---

# Estructura

Cada presencia es una carpeta. Esto es todo lo que puede tener adentro:

```
mi-sitio/
  manifest.json       quién es la presencia y qué puede leer
  main.js             tu código
  locales/en.json     los textos que muestra tu código
  locales/es.json     los mismos textos en otro idioma
  icon.webp           logo opcional
  .vscode/            ayuda para tu editor, la escribe la app
  jsconfig.json       conecta esa ayuda con tu editor
```

Tus presencias viven en `%LOCALAPPDATA%\com.manucabral.richpresenceplus\presences`. La página
Desarrollo te abre esa carpeta.

## manifest.json

El manifiesto describe la presencia antes de que corra tu código. La aplicación lo usa para
mostrar la tarjeta y para decidir qué puede tocar tu script.

```json
{
  "name": "Mi Sitio",
  "version": "1.0.0",
  "description": "Muestra la página en la que estás.",
  "client_id": "1567580382377803786",
  "kind": "video",
  "interval": 5,
  "entry": "main.js",
  "domains": ["misitio.com"],
  "page": {
    "matches": ["*://www.misitio.com/*"],
    "extract": {
      "title": { "from": "text", "selector": "h1" }
    }
  }
}
```

| Campo | Qué significa |
| --- | --- |
| `name` | El nombre que ve la gente. Tiene que ser único. |
| `version` | Subila cada vez que cambies una presencia publicada. |
| `description` | Una línea para la tarjeta. |
| `client_id` | La aplicación de Discord. La plantilla usa una compartida por todas las presencias. |
| `kind` | `game`, `video`, `music` u `other`. Define qué presencia gana cuando corren varias. |
| `interval` | Segundos entre dos llamadas a tu script. |
| `entry` | El archivo con tu código. |
| `domains` | Los sitios involucrados, para que la gente pueda excluirlos en Privacidad. |
| `page` | Qué leer de los sitios web. Mirá [Presencias web](05-web.md). |
| `permissions` | Qué leer de la computadora. Mirá [Presencias de escritorio](06-desktop.md). |

Tu script arranca sin acceso a nada. Solo llega a lo que pide el manifiesto, y la gente ve esa
lista antes de activar la presencia.

Los campos opcionales completan la tarjeta: `author`, `contributors`, `color`, `tags` e `icon`.
La pestaña Programar de la página Desarrollo tiene un formulario para ellos.

Las etiquetas salen de una lista fija, y así con una palabra se encuentran todas las presencias de
un tipo. Elegí hasta 8: `video`, `streaming`, `movie`, `series`, `anime`, `live`, `sport`, `music`, `radio`,
`podcast`, `game`, `multiplayer`, `reading`, `manga`, `news`, `wiki`, `blog`, `code`, `design`,
`study`, `social`, `chat`, `forum`, `shopping`, `ai` y `tool`.

## main.js

Este archivo tiene tus funciones, que explica la página siguiente. Es un solo archivo: un
script no puede cargar otros ni usar `import`.

Abrí la carpeta de la presencia en Visual Studio Code y el editor te explica lo que estás usando.
La aplicación escribe los archivos para eso, y siguen a tu manifiesto:

- `page.values.` ofrece los valores de tu `extract`, y muestra de qué selector sale cada uno.
- `context.options.` ofrece tus opciones, y una `choice` lista sus valores.
- `host.` ofrece solo lo que dan tus `permissions`, y marca cualquier otra cosa como error.
- Al pasar sobre `playback`, `cut` o cualquier campo de `activity`, te lo explica con un ejemplo.
- `manifest.json` autocompleta y avisa si un campo está mal.

Para que el editor sepa qué recibe cada función, la aplicación agrega un comentario arriba de
`read`, `activity` y `meta`. No cambia el código, y estos archivos nunca llegan al catálogo.

## locales

El script no escribe sus textos directamente. Los pide por clave, así una presencia habla varios
idiomas sin cambiar el código.

`locales/en.json` tiene todos los textos:

```json
{
  "description": "Shows the page you are on.",
  "browsing": "Browsing",
  "by": "By {name}"
}
```

El script usa `t`:

```js
t("browsing");               // "Browsing"
t("by", { name: "Manu" });   // "By Manu"
```

Para agregar un idioma, copiá el archivo a `locales/es.json` y traducí los valores. Dejá las
claves y las partes `{name}` como están. Una clave que no traduzcas se ve en inglés.

## Opciones

Las opciones dejan que cada persona cambie cómo se comporta la presencia. Se declaran en el
manifiesto:

```json
"options": [
  { "type": "boolean", "key": "show_title", "default": true }
]
```

La aplicación dibuja el control en la tarjeta de la presencia. Tu código lee el valor:

```js
function read(context) {
  const page = bestPage(context.pages);
  if (!page) return null;
  return { title: context.options.show_title ? page.title : "Una página privada" };
}
```

La etiqueta sale de las traducciones, con la clave `option.show_title`.

| `type` | Campos |
| --- | --- |
| `boolean` | `default` |
| `number` | `default`, opcionales `min` y `max` |
| `text` | `default`, opcional `max_length` |
| `choice` | `default`, `choices` |

Cada opción de una lista toma su etiqueta de `option.<clave>.<opción>`. Agregar `"when": "<clave>"`
muestra una opción solo mientras esa otra opción booleana está activa.
