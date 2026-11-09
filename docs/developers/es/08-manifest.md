---
title: Manifiesto
description: Cada campo de manifest.json, con su tipo, su valor por defecto y sus límites.
---

# Manifiesto

`manifest.json` describe una presencia antes de que corra su código. La aplicación lo usa para
dibujar la tarjeta, para decidir qué puede tocar el script y para revisar las actualizaciones.
Un campo que no conoce es un error.

```json
{
  "format": 1,
  "name": "Mi Video",
  "version": "1.0.0",
  "description": "Shows the video you are watching.",
  "client_id": "1567580382377803786",
  "kind": "video",
  "interval": 5,
  "domains": ["manucabral.github.io"],
  "page": {
    "matches": ["*://manucabral.github.io/*practice/*"],
    "extract": { "title": { "from": "text", "selector": "h1.video-title" } }
  },
  "author": "manucabral",
  "tags": ["video"]
}
```

## Formato

`format` dice para qué versión de este archivo y de las funciones del script está escrita la
presencia. Una aplicación que lee un formato anterior rechaza la presencia con `needs a newer
version of the application`, en vez de fallar en un campo o una ayuda que no tiene. Una presencia
sin `format` es de formato 1.

Dejá el número con el que empieza una presencia nueva. Cuando una página de esta documentación
dice que un campo o una ayuda necesita un formato mayor, subilo a ese número. Las presencias
escritas para un formato anterior siguen andando en las aplicaciones nuevas.

## Identidad

| Campo | Tipo | Obligatorio | Por defecto | Qué hace |
| --- | --- | --- | --- | --- |
| `format` | número | no | `1` | El formato para el que está escrita la presencia. Ver [Formato](#formato). |
| `name` | texto | sí | | El nombre que ve la gente. Letras, números, espacios, `_`, `-` y `.`. Tiene que ser único. |
| `version` | texto | sí | | Hasta 32 caracteres, como `1.2.0`. Subila cada vez que publicás un cambio. |
| `description` | texto | no | | Una línea para la tarjeta, de hasta 300 caracteres. `description` en un archivo de idioma la reemplaza en ese idioma, en la aplicación y en el catálogo. |
| `client_id` | texto | sí | | La aplicación de Discord, de 17 a 20 dígitos. `1567580382377803786` la comparten todas las presencias. |
| `author` | texto | no | | Quién la escribió, hasta 64 caracteres. |
| `contributors` | lista de textos | no | `[]` | Hasta 10 personas, de 64 caracteres cada una. |
| `color` | texto | no | | Color de la tarjeta, como `#RRGGBB`. |
| `icon` | texto | no | | Un archivo `.png` o `.webp` de la carpeta, o una dirección `https://`, de hasta 512 caracteres. |
| `tags` | lista de textos | no | `[]` | Hasta 8 palabras de la lista de abajo. Las que la aplicación no conoce se ignoran. |
| `other_names` | lista de textos | no | `[]` | Hasta 8 nombres más con los que se la busca, como una forma corta o un nombre viejo, de hasta 64 caracteres cada uno y distintos de `name`. |

Etiquetas: `video`, `streaming`, `movie`, `series`, `anime`, `live`, `sport`, `music`, `radio`,
`podcast`, `game`, `multiplayer`, `reading`, `manga`, `news`, `wiki`, `blog`, `code`, `design`,
`study`, `social`, `chat`, `forum`, `shopping`, `ai` y `tool`.

## Comportamiento

| Campo | Tipo | Obligatorio | Por defecto | Qué hace |
| --- | --- | --- | --- | --- |
| `kind` | `game`, `video`, `music` u `other` | no | según la actividad | Fija la prioridad por defecto cuando corren varias presencias a la vez. |
| `interval` | número | no | `15` | Segundos entre dos llamadas al script, de 1 a 300. |
| `hide_when_paused` | booleano | no | `true` | Oculta la tarjeta mientras `meta` dice `paused`. La gente puede cambiarlo. |
| `entry` | texto | no | `main.js` | El archivo del script, una ruta `.js` relativa dentro de la carpeta. |
| `domains` | lista de textos | no | `[]` | Hasta 32 nombres de host en minúsculas, para que la gente pueda excluirlos en Privacidad. |

`kind` en el manifiesto no es `kind` en la actividad. El del manifiesto fija la prioridad:
`game` 80, `video` 60, `music` 40, `other` 20. Si falta, decide el `kind` de la actividad:
`playing` y `competing` cuentan como `game`, `listening` como `music`, `watching` como `video`.

## Acceso

| Campo | Tipo | Obligatorio | Por defecto | Qué hace |
| --- | --- | --- | --- | --- |
| `page` | objeto | no | | Lo que la extensión lee de los sitios web. Mirá [Extractores](09-extractors.md). |
| `permissions` | objeto | no | ninguno | Lo que el script puede leer de la computadora. Mirá [Host](11-host.md). |

Un script arranca sin acceso a nada. La gente ve estos dos campos antes de activar la presencia,
y una actualización que pide más espera hasta que lo acepten.

### page

| Campo | Tipo | Obligatorio | Qué hace |
| --- | --- | --- | --- |
| `matches` | lista de patrones | sí, salvo con `user_sites` | Direcciones que lee la presencia, hasta 16, como `*://www.example.com/*`. |
| `user_sites` | booleano | no | `true` para un servicio que cada uno instala en su propio servidor: cada persona agrega su dirección en la aplicación. |
| `extract` | objeto | no | Hasta 32 valores, cada uno con un nombre de letras, números y `_`. |
| `frames` | objeto | no | Frames de otros sitios para leer dentro de esas páginas: `matches` (de 1 a 8) y `extract`. |

Un patrón empieza con `*://`, `http://` o `https://`, nombra un host y tiene una ruta. `*`
coincide con cualquier cosa.

## Opciones

`options` es una lista de hasta 16 ajustes que la gente puede cambiar. El script los lee en
`context.options`, cada etiqueta sale de la clave `option.<clave>` del archivo de idioma, y una
línea que la explica, de `help.<clave>`.

| Campo | Tipo | Obligatorio | Qué hace |
| --- | --- | --- | --- |
| `type` | `boolean`, `number`, `text` o `choice` | sí | Qué control dibuja la aplicación. |
| `key` | texto | sí | El nombre en `context.options`. Único en la presencia. |
| `default` | según `type` | sí | El valor antes de que alguien lo cambie. |
| `when` | texto u objeto | no | La clave de una opción booleana que tiene que estar activa, o hasta 4 claves de opciones con el valor o los valores que tiene que tener cada una. |
| `min`, `max` | número | no | Límites de un `number`. |
| `max_length` | número | no | El `text` más largo: 64 por defecto, 256 como máximo. |
| `choices` | lista de textos | para `choice` | Hasta 16 valores. Cada etiqueta sale de `option.<clave>.<valor>`. |

`when` no puede nombrar a la misma opción ni formar un ciclo. Cómo usar las opciones:
[Opciones e idiomas](05-options.md).
