---
title: Límites y errores
description: Los límites dentro de los que corre cada presencia, y qué significa cada mensaje de error de la aplicación.
---

# Límites y errores

## Límites de un script

| Qué | Límite |
| --- | --- |
| Memoria | 16 MiB |
| Cargar el archivo | 1 segundo |
| Cada llamada | 2 segundos |
| Lo que devuelve `read` | 64 KiB |
| `interval` | De 1 a 300 segundos |
| Consola | 200 líneas guardadas, 50 por llamada, 500 caracteres por línea |

Las variables persisten entre llamadas, así que un caché está bien, pero limitá cuánto guarda.
Un script que pasa un límite se detiene en esa vuelta y la pestaña Probar muestra el error.

## Límites de un manifiesto

| Qué | Límite |
| --- | --- |
| `page.matches` | De 1 a 16 patrones |
| `page.frames.matches` | De 1 a 8 patrones |
| Valores en un `extract` | 32 |
| Un selector, atributo o nombre de meta | 256 caracteres |
| Una ruta | 128 caracteres, 8 nombres |
| Elementos de `list` | 16 por defecto, 32 como máximo |
| `options` | 16, con hasta 16 valores cada una |
| `domains` | 32 |
| `args` | 16 programas, de 1 a 16 opciones cada uno |
| `files.tail` | 500 líneas |
| `tags` | 8 |

## Límites de la tarjeta

| Qué | Límite |
| --- | --- |
| Cada texto | De 2 a 128 caracteres |
| Una dirección o imagen | 512 caracteres, `http://` o `https://`, sin usuario ni contraseña |
| Botones | 2, con etiquetas de 1 a 32 caracteres |
| `party_size` | De 1 a 1000 |

## Mensajes del script

| Mensaje | Qué significa |
| --- | --- |
| `script failed to load: ...` | El archivo tiene un error de sintaxis, o falló al cargar. |
| `script must define the function read` | Falta `read` o `activity`, o está mal escrita. |
| `script error: ...` | Una función lanzó un error. El resto del mensaje es ese error. |
| `script exceeded its time budget` | Una llamada tardó más de 2 segundos. Buscá un ciclo que no termina. |
| `script returned invalid data: ...` | Una función devolvió algo que no son datos. `read must not return a promise` significa que era `async`, y eso no se permite. |
| `snapshot exceeds 65536 bytes` | `read` devolvió demasiado. Quedate solo con lo que usa `activity`. |
| `permission denied: ...` | El script llamó a una función de `host` que el manifiesto no pide. |
| `code generation is disabled` | El script intentó `new Function`. Armar código desde texto no se permite, y `eval` no existe. |

## Mensajes de la actividad

| Mensaje | Qué significa |
| --- | --- |
| `details must have at most 128 characters` | Un texto es demasiado largo. Usá `cut`. |
| `use at most 2 buttons` | `buttons` tiene más de dos. |
| `button label must have 1 to 32 characters` | La etiqueta de un botón está vacía o es demasiado larga. |
| `button address must be HTTP or HTTPS without credentials` | El `url` de un botón no es una dirección web. |
| `end must not be earlier than start` | `end` es anterior a `start`. |
| `details_url must be an HTTP or HTTPS address without credentials` | Un enlace no es una dirección web. |
| `party_size must be [current, largest] with 1 <= current <= largest <= 1000` | Los números del grupo están fuera de rango. |

La primera columna muestra un campo; el mismo mensaje nombra el campo que haya roto la regla.

## Mensajes del manifiesto

Un error del manifiesto nombra el campo y lo que espera, como `interval: expected 1 to 300
seconds` o `color: expected #RRGGBB`. Todos los campos y sus límites están en
[Manifiesto](08-manifest.md).
