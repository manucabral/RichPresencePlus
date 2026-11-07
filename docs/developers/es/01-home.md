---
title: Inicio
description: Aprendé a escribir presencias para Rich Presence Plus en JavaScript, para sitios web y programas de Windows, sin compilar ni instalar herramientas.
---

# Inicio

Esta documentación te enseña a escribir una presencia para Rich Presence Plus, la app que
muestra en Discord lo que hacés en un sitio web o en un programa de Windows.

Una presencia es una carpeta chica con un archivo JavaScript. Su trabajo es responder una
pregunta una y otra vez: ¿qué está haciendo esta persona ahora? La aplicación pregunta cada
pocos segundos, tu código responde, y la aplicación se encarga de Discord.

Este es un script completo. Muestra el título de la página en la que estás:

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

`read` mira la página y se queda con lo que necesita. `activity` convierte eso en lo que
muestra Discord. Esas dos funciones son casi todo lo que vas a escribir.

No necesitás Rust, ni Node, ni compilar nada. Guardás el archivo y la aplicación lo carga.

## Para empezar

- [Instalación](02-setup.md)
- [Estructura](03-structure.md)
- [Funciones](04-functions.md)
- [Presencias web](05-web.md)
- [Presencias de escritorio](06-desktop.md)
- [Publicar](07-publish.md)
