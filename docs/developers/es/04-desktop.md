---
title: Mostrar un programa de Windows
description: Armá paso a paso una presencia para un programa o juego de Windows, con permisos para leer procesos, ventanas y archivos de log.
---

# Mostrar un programa de Windows

Una presencia de escritorio lee la computadora con el objeto `host`, que arranca vacío. Cada
permiso del manifiesto le agrega funciones, y la gente ve esa lista antes de activar la
presencia. Esta guía arma una presencia para un juego, de a una parte. Creala con la plantilla
"Programa de escritorio" de la página Desarrollo y dejá la pestaña Probar vigilándola mientras
avanzás.

## 1. Pedir lo que necesitás

Para saber si un juego está abierto, la presencia necesita la lista de programas en ejecución:

```json
"permissions": { "process": true }
```

Ahora existe `host.process.list()`. Sin el permiso, llamarla falla y la página Desarrollo muestra
el error.

## 2. Encontrar el programa

```js
const PROGRAM = "game.exe";

function read(context) {
  const running = host.process.list().some((name) => name.toLowerCase() === PROGRAM);
  if (!running) return null;
  return { playing: true };
}
```

Los nombres de programa pueden venir en mayúsculas o minúsculas, por eso el script los compara en
minúsculas. Devolver `null` cuando el juego está cerrado es lo que hace que la presencia
desaparezca. Para ver el nombre exacto de un programa, abrí la pestaña Detalles del Administrador
de tareas.

## 3. Contar el tiempo

Discord dibuja el contador a partir de `start`, así que el script tiene que recordar cuándo se
abrió el juego. Una variable fuera de las funciones se conserva entre llamadas.

```js
let since = 0;

function read(context) {
  const running = host.process.list().some((name) => name.toLowerCase() === PROGRAM);
  if (!running) {
    since = 0;
    return null;
  }
  if (since === 0) since = context.now;
  return { since };
}
```

## 4. Mostrarlo

```js
function activity(game) {
  return {
    details: "My Game",
    state: t("playing"),
    kind: "playing",
    start: game.since,
    large_image: "logo",
  };
}

function meta() {
  return { application: PROGRAM };
}
```

`"logo"` es el nombre de una imagen subida a tu aplicación de Discord. `meta` da el nombre del
programa para que la gente pueda excluirlo en Privacidad.

## 5. Saber cuándo está al frente

Pedí también el permiso `window`, y el juego puede subir por encima de otras presencias mientras
lo jugás:

```json
"permissions": { "process": true, "window": true }
```

```js
function read(context) {
  // ...el código de arriba...
  const window = host.window.foreground();
  return { since, focused: Boolean(window) && window.process.toLowerCase() === PROGRAM };
}

function meta(game) {
  return { application: PROGRAM, foreground: game.focused };
}
```

`foreground` también da el `title` de la ventana, que muchas veces tiene el nombre de un archivo o
proyecto. Si alcanza con el nombre del programa, no toques el título.

## 6. Leer más sobre el juego

Los archivos de log suelen decir qué pasa dentro de un programa. Esto lee el final del log más
nuevo:

```json
"permissions": { "process": true, "files": ["%LOCALAPPDATA%/MyGame/logs"] }
```

```js
function read(context) {
  // ...el código de arriba...
  const log = host.files.latest(host.files.roots[0], ".log");
  const lines = host.files.tail(log, 50);
  const level = lines.map((line) => /Entered level (.+)/.exec(line)).findLast(Boolean);
  return { since, level: level ? level[1] : null };
}
```

Después mostralo con `state: game.level || t("playing")` en `activity`.

Leé el final del archivo y no todo, y guardá la ruta del log durante un minuto en vez de buscarla
en cada llamada: una carpeta puede tener cientos de archivos.

## Otros permisos

Los programas también se pueden leer por su línea de comandos, por lo que Windows está
reproduciendo, por APIs web y por servidores de juegos. [Host](11-host.md) lista cada permiso y
lo que agrega. Pedí solo lo que usa el script: una actualización que suma un permiso espera a
que la persona lo acepte.

## Un caso real

`presences/ready-or-not` son los pasos 1 a 4 para un juego real. `presences/roblox` va más lejos
con un archivo de log y una API web, y `presences/creator-tools` mira solo el programa que está al
frente. Las tres están en el repositorio.

## Sin código

Para mostrar un programa sin código, abrí la página Presencias y usá "Agregar un programa". Elegilo
de la lista, escribí las líneas y la aplicación crea la carpeta por vos.
