---
title: Empezar
description: Escribí presencias para Rich Presence Plus en JavaScript, para sitios web y programas de Windows, sin compilar nada ni instalar herramientas.
---

# Empezar

Una presencia muestra en Discord lo que alguien hace en un sitio web o en un programa de
Windows. Es una carpeta con un manifiesto y un archivo de JavaScript. Cada pocos segundos la
aplicación le pregunta a tu script qué está pasando, y ella se encarga de Discord.

```js
function read(context) {
  const page = bestPage(context.pages);
  if (!page) return null;
  return { title: page.title };
}

function activity(seen) {
  return { details: cut(seen.title), kind: "watching" };
}
```

Ese es un script completo. `read` junta lo que necesita y `activity` dice qué muestra la
tarjeta. No necesitás Rust, ni Node, ni compilar: guardás el archivo y la aplicación lo carga.

## Por dónde seguir

| Querés | Leé |
| --- | --- |
| Hacer tu primera presencia, paso a paso | [Tu primera presencia](02-first-presence.md) |
| Mostrar un programa de Windows o un juego | [Mostrar un programa de Windows](04-desktop.md) |
| Sacar un dato de un sitio | [Leer un sitio](03-read-a-site.md) |
| Arreglar una presencia que no muestra nada | [Cuando no aparece nada](06-troubleshoot.md) |
| Consultar un campo o una función | [Manifiesto](08-manifest.md), [Script](10-script.md), [Ayudantes](12-helpers.md) |
| Entender qué pasa detrás de la tarjeta | [Cómo funciona](14-how-it-works.md) |

## Cómo están ordenadas estas páginas

- **Empezar** enseña con un ejemplo guiado.
- **Guías** resuelven una tarea cada una.
- **Referencia** lista cada campo, función y límite, con la misma forma en todas las páginas.
- **Cómo funciona** explica el diseño detrás de las reglas.

## Qué necesitás

- Windows 10 u 11, con la aplicación de escritorio de Discord abierta.
- Rich Presence Plus instalado.
- Un editor de código. Visual Studio Code además explica cada función mientras escribís.
- Para sitios web, la extensión del navegador, conectada desde la página Navegador de la
  aplicación.
