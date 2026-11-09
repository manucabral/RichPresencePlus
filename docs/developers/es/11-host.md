---
title: Host
description: Los permisos que puede pedir una presencia, y las funciones de host que cada uno le agrega al script.
---

# Host

`host` es cómo un script lee la computadora. Arranca vacío, y cada permiso de `permissions` en el
manifiesto le agrega funciones. Llamar a una que no se pidió falla con `permission denied` y el
nombre del permiso.

```json
"permissions": { "process": true, "window": true, "files": ["%LOCALAPPDATA%/MyGame/logs"] }
```

| Permiso | Valor | Agrega |
| --- | --- | --- |
| `process` | `true` | `host.process.list()` |
| `args` | objeto | `host.process.args(name)` |
| `window` | `true` | `host.window.foreground()` |
| `media` | `true` | `host.media.current()` |
| `files` | lista de carpetas | `host.files` |
| `http` | lista de hosts | `host.http.get(url)` |
| `query` | lista de protocolos | `host.query(protocol, address, port)` |

La gente ve esta lista antes de activar la presencia. Una actualización que suma un permiso
espera a que lo acepten, y mientras tanto sigue corriendo la versión instalada.

## process

`host.process.list()` da los nombres de los programas abiertos, como `["explorer.exe",
"Game.exe"]`. Los nombres llegan con las mayúsculas que les da Windows, así que comparalos en
minúsculas.

```js
const running = host.process.list().some((name) => name.toLowerCase() === "game.exe");
```

## args

`host.process.args(name)` lee opciones elegidas de la línea de comandos de un programa abierto.
El manifiesto nombra el programa y hasta 16 opciones para cada uno de hasta 16 programas. El
script nunca ve el resto de la línea.

```json
"permissions": { "args": { "gta_sa.exe": ["-h", "-p"] } }
```

```js
host.process.args("gta_sa.exe");   // { "-h": "127.0.0.1", "-p": "7777" }
```

**Devuelve** un objeto con cada opción encontrada: su valor, o `true` si no tiene. `null` cuando
el programa no está abierto.

## window

`host.window.foreground()` da `{ process, title }` de la ventana que está al frente, o `null`. El
título suele tener el nombre de un archivo o proyecto: si alcanza con el nombre del programa, no
lo uses.

## media

`host.media.current()` da lo que Windows dice que se está reproduciendo: `{ app, title, artist,
album, status, position, duration }`, o `null`. `position` y `duration` son segundos.

## files

Lee archivos de log dentro de las carpetas que lista el manifiesto. Una carpeta puede empezar con
una variable de entorno como `%LOCALAPPDATA%`; una que no existe en la computadora se ignora.

| Miembro | Qué hace |
| --- | --- |
| `host.files.roots` | Las carpetas que lista el manifiesto, como rutas de esta computadora. |
| `host.files.latest(dir, suffix)` | El archivo más nuevo de `dir` cuyo nombre termina en `suffix`, o `null`. |
| `host.files.tail(path, lines)` | Las últimas `lines` líneas de un archivo, hasta 500. |

```js
const log = host.files.latest(host.files.roots[0], ".log");
const lines = host.files.tail(log, 50);
```

Leé el final de un archivo y no todo, y guardá la ruta por un minuto en vez de buscar en cada
llamada: una carpeta puede tener cientos de archivos.

## http

`host.http.get(url)` trae una dirección `https://` de los hosts que lista el manifiesto, tal como
están escritos: `api.example.com` no permite `www.example.com`.

```json
"permissions": { "http": ["api.example.com"] }
```

**Devuelve** `{ status, body }`, donde `body` es texto. No sigue redirecciones, espera hasta 5
segundos y lee hasta 1 MB.

## query

`host.query(protocol, address, port)` le pregunta a un servidor de juegos por sí mismo. El único
protocolo es `samp`, para servidores de SA-MP y open.mp. `port` va de 1 a 65535.

```json
"permissions": { "query": ["samp"] }
```

**Devuelve** `{ password, players, max_players, hostname, gamemode, language }`, o `null` cuando
el servidor no responde.
