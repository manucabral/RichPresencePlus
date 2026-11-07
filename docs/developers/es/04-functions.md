---
title: Funciones
description: Cómo funciona un script de Rich Presence Plus, qué reciben y devuelven read, activity y meta, y las ayudas que puede usar toda presencia.
---

# Funciones

Un script define funciones y la aplicación las llama. Nunca las llamás vos, y nunca hablás con
Discord.

## Cómo corre una presencia

Cada `interval` segundos la aplicación hace esto:

1. Llama a `read`. Tu código mira lo que pasa y devuelve lo que encontró.
2. Si `read` devolvió `null`, no se muestra nada y la vuelta termina acá.
3. Llama a `activity` con lo que devolvió `read`. Tu código decide qué muestra Discord.
4. Llama a `meta` con ese mismo valor, si escribiste una.
5. Aplica las reglas de privacidad de la persona, compara con las demás presencias y manda la
   ganadora a Discord.

Separar las dos funciones simplifica todo: `read` junta, `activity` presenta. Si cambia el diseño
de un sitio, tocás `read`. Si querés un botón nuevo, tocás `activity`.

## read(context)

`read` recibe lo que sabe la aplicación y devuelve un objeto simple con lo que necesitás, o `null`
cuando no hay nada para mostrar.

```js
function read(context) {
  const page = bestPage(context.pages);
  if (!page) return null;
  return { title: page.title, opened: context.now };
}
```

| Campo de `context` | Qué tiene |
| --- | --- |
| `now` | La hora, en segundos Unix. |
| `pages` | Las páginas que informó la extensión para esta presencia. |
| `options` | El valor de cada opción. |

Cada página tiene `url`, `title`, `received_at` y `values`, los datos que pediste en el manifiesto.

Las variables fuera de las funciones se conservan entre llamadas. Así recuerda cosas un script,
por ejemplo cuándo empezó algo:

```js
let since = 0;

function read(context) {
  const running = host.process.list().includes("game.exe");
  if (!running) {
    since = 0;
    return null;
  }
  if (since === 0) since = context.now;
  return { since };
}
```

## activity(seen)

`activity` recibe lo que devolvió `read` y describe la tarjeta. Todos los campos son opcionales,
pero la tarjeta necesita por lo menos un texto o una imagen.

```js
function activity(seen) {
  return {
    details: cut(seen.title),
    state: t("browsing"),
    kind: "watching",
    start: seen.opened,
    buttons: [{ label: t("open"), url: seen.url }],
  };
}
```

| Campo | Qué hace |
| --- | --- |
| `name` | El título de la actividad, en lugar del nombre de la aplicación de Discord. |
| `details`, `state` | Las dos líneas de texto, hasta 128 caracteres. Con menos de dos caracteres se descartan. |
| `kind` | `playing`, `listening`, `watching` o `competing`. |
| `start`, `end` | Segundos Unix. `start` solo cuenta hacia arriba; con `end` dibuja una barra de progreso. |
| `large_image`, `small_image` | Una dirección `https://`, o el nombre de una imagen subida a tu aplicación de Discord. |
| `large_text`, `small_text` | El texto que aparece al pasar sobre las imágenes. |
| `buttons` | Hasta dos `{ label, url }`. |
| `details_url`, `state_url` | La dirección que se abre al hacer clic en esa línea. |
| `large_url`, `small_url` | La dirección que se abre al hacer clic en esa imagen. |
| `party_size` | `[actual, máximo]`, se ve como "(2 de 4)". |
| `status_display` | Qué texto muestra la lista de miembros: `name`, `state` o `details`. |

## meta(seen)

`meta` es opcional. Le dice a la aplicación cosas que Discord no muestra, que la ayudan a elegir
entre presencias y a aplicar las reglas de privacidad.

```js
function meta(seen) {
  return { paused: seen.paused, application: "game.exe" };
}
```

| Campo | Qué hace |
| --- | --- |
| `paused` | Una presencia en pausa le deja lugar a una que reproduce, y por defecto se oculta. |
| `foreground` | Sube la presencia mientras su ventana tiene el foco. |
| `application` | Nombre del programa, para que la gente pueda excluirlo en Privacidad. |
| `domains` | Sitios extra, para que la gente pueda excluirlos en Privacidad. |

Para que una presencia siga a la vista en pausa, agregá `"hide_when_paused": false` al manifiesto.

## Ayudas

Todo script puede usar estas funciones sin importar nada.

| Ayuda | Qué hace |
| --- | --- |
| `t(key, values)` | Da el texto de `key` en el idioma de la persona. |
| `cut(text, max)` | Acorta un texto a `max` caracteres (128 por defecto). |
| `parseUrl(address)` | Da `{ host, path, query }`, o `null`. No existe la clase `URL`. |
| `bestPage(pages)` | Elige la página con un reproductor en marcha, o si no la primera. |
| `playback(player, receivedAt)` | Da `{ playing, start, end }` para un valor `media`. |
| `parseTime(text)` | Lee `1:02:03`, `4:05` o `30` como segundos, o da `null`. |
| `progress(current, duration, now)` | Da `{ start, end }` para una posición y una duración en segundos. |
| `console.log(...)` | Escribe una línea en el bloque Consola de la página Desarrollo. |

Para saber qué tiene `host`, mirá [Presencias de escritorio](06-desktop.md).

## Límites

Un script tiene 16 MiB de memoria y 2 segundos por llamada, y `read` tiene que devolver menos de
64 KiB. Las variables se conservan, así que podés guardar datos, pero limitá cuántos.
