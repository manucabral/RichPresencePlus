---
title: Cuando no aparece nada
description: Descubrí por qué una presencia no muestra nada o muestra otra cosa, desde la pestaña Probar hasta los mensajes de la aplicación.
---

# Cuando no aparece nada

Abrí la página Desarrollo, elegí la presencia en la pestaña Probar y tocá Vigilar. Ahí se ve la
mayoría de los problemas: lo que devolvió `read`, la actividad, la consola y cualquier error en
rojo.

## Encontrar dónde se corta

Recorré la cadena desde el principio hasta el final:

1. **La página.** ¿Hay páginas debajo de `read`? Si no, la extensión no está leyendo el sitio.
2. **Los valores.** ¿Las páginas traen los valores que pediste? Si uno está vacío, su selector o
   su ruta no coincide.
3. **`read`.** ¿Devuelve un objeto? `null` significa que no se muestra nada, a propósito.
4. **`activity`.** ¿La vista previa muestra la tarjeta? Un error en rojo nombra el campo que
   pasó un límite.
5. **Discord.** ¿Tu perfil la muestra? Si la vista previa está bien y Discord no, puede que otra
   presencia tenga más prioridad, o que una regla de privacidad la oculte.

Agregá `console.log(...)` en cualquier parte del script y leé las líneas en el bloque Consola.

## Causas comunes

| Qué ves | Por qué | Qué hacer |
| --- | --- | --- |
| No hay páginas debajo de `read` | `matches` no coincide con la dirección, o el sitio no tiene permiso. | Revisá `matches`, y después abrí la extensión y tocá "Permitir este sitio". |
| Un valor siempre llega vacío | El selector o la ruta no coincide con la página. | Probalo en la Consola del navegador con `document.querySelector`. |
| Un reproductor dentro de un frame no lee nada | El frame es de otro sitio. | Leelo con `page.frames`: mirá [Leer un sitio](03-read-a-site.md#leer-un-reproductor-dentro-de-un-frame). |
| Un error rojo de permiso | El script llama a algo que el manifiesto no pide. | Agregalo en `permissions`. |
| La tarjeta desaparece | `read` devolvió `null`, o falló. | Registrá lo que ve `read`. |
| La tarjeta se oculta en pausa | `meta` dice `paused`. | Poné `"hide_when_paused": false` para mantenerla. |
| Un texto se ve como su clave | Falta la clave en `locales/en.json`. | Agregala ahí. |
| Discord muestra otro nombre | `activity` no devuelve `name`. | Agregá `name`. |
| Falta una línea de texto | Tenía menos de 2 caracteres. | Discord los rechaza, así que la aplicación los saca. |
| Guardar no cambia nada | La presencia vino con la aplicación o del catálogo. | Copiala en la página Desarrollo y editá la copia. |

Cada mensaje de error, y qué significa, está en [Límites y errores](13-limits.md).

## Cuando la vista previa está bien y Discord no

- La aplicación de escritorio de Discord tiene que estar abierta. La versión web no muestra
  actividades.
- El Panel dice qué cuenta recibe la actividad.
- Cuando corren varias presencias a la vez, gana la de mayor prioridad. La página Presencias
  muestra y cambia cada prioridad.
- Una regla de Privacidad puede ocultar un sitio, un programa o un campo.
