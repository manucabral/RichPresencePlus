---
title: Instalación
description: Prepará lo necesario para escribir una presencia de Rich Presence Plus, creala desde una plantilla y mirá cómo cambia cada vez que guardás.
---

# Instalación

Al terminar esta página vas a tener una presencia funcionando, y vas a ver cómo cambia cada
vez que guardás el archivo.

## Requisitos

- Windows 10 u 11, con la app de escritorio de Discord abierta.
- Rich Presence Plus instalado.
- Un editor de código, como Visual Studio Code.
- Para presencias web, la extensión del navegador conectada. La página Navegador de la
  aplicación explica cómo.

## Crear una presencia

1. Abrí la aplicación y andá a la página Desarrollo.
2. En "Crear una presencia", escribí `Mi Sitio`.
3. Dejá la plantilla en "Página web" y tocá Crear.

La aplicación escribe una carpeta y pasa a la pestaña Programar. Tocá "Abrir en VS Code", o "Abrir
carpeta" si usás otro editor. Vas a encontrar una presencia que ya funciona: las páginas siguientes
explican cada archivo.

## Elegir el nombre en Discord

La plantilla ya funciona en Discord. Usa una aplicación compartida por todas las presencias, y
su script pone el título con `name`:

```js
function activity(site) {
  return { name: "Mi Sitio", details: cut(site.title), kind: "watching" };
}
```

No necesitás crear nada en Discord. Creá tu propia aplicación en discord.com/developers solo si
querés imágenes propias: lo que subas en Rich Presence, Art Assets se puede usar por nombre en
`large_image`. Después pegá su Application ID en `client_id`.

## Verla en vivo

Desde ahora vas a trabajar así.

1. En la página Desarrollo, abrí la pestaña Probar, elegí Mi Sitio y tocá Vigilar.
2. Abrí `main.js`, cambiá algo y guardá.

La aplicación carga la versión nueva en menos de un segundo. La página corre tu script y
muestra lo que devolvió `read`, la actividad, las líneas de `console.log` y una vista previa
de la tarjeta como la dibuja Discord.

Si guardás un error, un mensaje en rojo dice qué falló y la última versión que funcionaba
sigue corriendo. Corregilo y guardá de nuevo. Nada de lo que escribas acá puede romper la
aplicación.

Para eliminar una presencia tuya, abrí la pestaña Programar y tocá "Eliminar presencia". Va a la
Papelera de Windows, así que podés recuperarla desde ahí. También podés agregar o borrar carpetas
desde el Explorador de archivos: la aplicación lo nota en unos segundos.

## Cuando algo no se ve

| Qué ves | Por qué | Qué hacer |
| --- | --- | --- |
| La vista previa está vacía | La extensión no puede leer el sitio, o `matches` no coincide con la dirección. | Revisá `matches`, y después abrí la extensión y permití el sitio. |
| Un error rojo de permiso | El script llama a algo que el manifiesto no pide. | Agregalo en `permissions`. |
| La actividad desaparece | `read` devolvió `null`, o falló. | Agregá `console.log` y mirá el bloque Consola. |
| Un texto se ve como su clave | Falta la clave en `locales/en.json`. | Agregala ahí. |
| Discord muestra otro nombre | `activity` no devuelve `name`. | Agregá `name` a lo que devuelve. |
| Guardar no cambia nada | La presencia vino con la app o del catálogo. | Copiala en la página Desarrollo y editá la copia. |
