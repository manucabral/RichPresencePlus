---
title: Ayudantes
description: Las funciones que puede llamar cualquier script de presencia sin importar nada.
---

# Ayudantes

Cualquier script puede llamar a estas funciones, sin importar ni pedir nada. Sus nombres están
tomados: un script que declara su propia función o variable `format`, `icon` o cualquier otra de
ellas no carga, y el error nombra el ayudante.

| Ayudante | Da |
| --- | --- |
| `t(key, values)` | El texto de `key` en el idioma de la persona. |
| `cut(text, max)` | Un texto lo bastante corto para Discord. |
| `parseUrl(address)` | Las partes de una dirección. |
| `bestPage(pages)` | La página que importa cuando coinciden varias. |
| `playback(player, receivedAt)` | Si un video suena y su barra de progreso. |
| `parseTime(text)` | Un texto de reloj en segundos. |
| `progress(current, duration, now)` | La barra de progreso para una posición y una duración. |
| `rotate(items, seconds, now)` | Una de varias cosas, por turnos. |
| `format(template, values)` | Un texto que escribió la persona, completado. |
| `icon(name)` | Una imagen chica para reproducir, pausa y similares. |
| `console.log(...)` | Una línea en el bloque Consola de la página Desarrollo. |

## t(key, values)

El texto de `key` en `locales/<idioma>.json`, con cada `{name}` completado desde `values`.

```js
t("by", { name: "Manu" });   // "De Manu"
```

Una clave que falta en el idioma de la persona sale de `en.json`. Una que falta también ahí da la
clave misma. Mirá [Opciones e idiomas](05-options.md#escribir-los-textos-por-clave).

## cut(text, max)

Acorta un texto a `max` caracteres, 128 por defecto, que es el límite de Discord.

```js
cut("Un título muy largo", 10);   // "Un título…"
```

**Devuelve** `null` cuando `text` no es un texto.

## parseUrl(address)

Separa una dirección. Una presencia no tiene la clase `URL`.

```js
parseUrl("https://a.com/watch?v=1");   // { host: "a.com", path: "/watch", query: { v: "1" } }
```

**Devuelve** `null` cuando no es una dirección.

## bestPage(pages)

Elige cuando coinciden varias pestañas: la que suena, después la que está en pantalla, después
una con reproductor. Entre iguales toma la pestaña que tuvo el foco por última vez, después la
primera. Busca el valor llamado `player`, así que usá ese nombre para un extractor `media`.

Una extensión del navegador anterior a la 0.1.2 no dice qué pestaña está en pantalla; con ella
deciden el reproductor y el orden.

```js
const page = bestPage(context.pages);
if (!page) return null;
```

**Devuelve** `null` cuando `pages` está vacía.

## playback(player, receivedAt)

Convierte el valor de un extractor `media` en `{ playing, start, end }`. Pasá `page.received_at`
como `receivedAt`, así la barra cuenta desde que se leyó la página.

```js
return { title, ...playback(page.values.player, page.received_at) };
```

En pausa, `playing` es `false` y `start` y `end` son `null`, así que la barra desaparece sola.
Cuando no se conoce la duración, solo `end` es `null` y el tiempo cuenta hacia arriba. La
velocidad del reproductor cuenta: a 2x, un video de 4 minutos termina en 2.

## parseTime(text)

Lee `1:02:03`, `4:05` o `30` como segundos.

```js
parseTime("1:02:03");   // 3723
```

**Devuelve** `null` cuando el texto no es una hora.

## progress(current, duration, now)

El `{ start, end }` que dibuja una barra en `current` segundos de `duration`, para
`context.now`. Sirve cuando un sitio muestra el tiempo como texto en vez de un reproductor.

```js
const bar = progress(parseTime("4:05"), parseTime("10:00"), context.now);
```

## rotate(items, seconds, now)

Da uno de `items`, y el siguiente cada `seconds` segundos de `now`. Los vacíos (`null`,
`undefined`, `""`) se saltean, así que los valores que pueden faltar entran como están.

```js
state: rotate([video.channel, t("views", { count: video.views })], 30, context.now),
```

El cambio se ve cuando la presencia vuelve a leer, así que un `seconds` menor que `interval` del
manifiesto no cambia nada. Menos de 1 cuenta como 30.

**Devuelve** `null` cuando ningún elemento tiene valor.

## format(template, values)

Completa un texto que la persona escribió en una opción, como `{title} - {channel}`, con
`values`. Un valor `null` o que falta no deja nada, y un separador (`-`, `|`, `,`, `:`, `/`) que
queda al principio o al final se va con él. Un nombre que no está en `values` queda como está
escrito, así se nota un error de tipeo.

```js
const line = format(context.options.line, { title: video.title, channel: video.channel });
format("{title} - {channel}", { title: "Un paseo", channel: null });   // "Un paseo"
```

Usá [`t`](#tkey-values) para los textos que escribís vos, y `format` para los que escribe la persona.

**Devuelve** `null` cuando no queda nada, o cuando `template` no es un texto.

## icon(name)

La dirección de una imagen chica que puede usar cualquier presencia, sea cual sea su aplicación
de Discord. Discord muestra `small_image` como un círculo en la esquina de la imagen grande.

| `name` | Muestra |
| --- | --- |
| `play` | Un triángulo |
| `pause` | Dos barras |
| `stop` | Un cuadrado |
| `live` | Un punto rojo |
| `repeat` | Dos flechas en círculo |
| `repeat-one` | El círculo con un 1 |
| `shuffle` | Dos flechas cruzadas |

```js
small_image: icon(video.playing ? "play" : "pause"),
small_text: video.playing ? t("playing") : t("paused"),
```

**Devuelve** `null` para un nombre que no está en la tabla.

## console.log(...)

Escribe una línea en el bloque Consola de la página Desarrollo. Los objetos se escriben como
JSON. `console.warn` y `console.error` funcionan igual y marcan la línea. La página guarda las
últimas 200 líneas, 50 por llamada, de hasta 500 caracteres cada una.
