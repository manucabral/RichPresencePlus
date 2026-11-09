---
title: Cómo funciona
description: Qué pasa entre una presencia y Discord, y por qué una presencia está hecha como está hecha.
---

# Cómo funciona

## Una vuelta

Cada `interval` segundos, para cada presencia activa:

1. La aplicación llama a `read` con la hora, las páginas que informó la extensión y las opciones.
2. Si `read` devolvió `null`, la presencia no tiene nada para mostrar en esta vuelta.
3. Llama a `activity` y a `meta` con lo que devolvió `read`.
4. Revisa la actividad contra los límites de Discord y aplica las reglas de privacidad de la
   persona.
5. Compara todas las presencias que tienen algo para mostrar y manda una a Discord.

`read` junta y `activity` presenta. Cuando un sitio cambia su diseño, tocás `read` y el
manifiesto. Cuando querés un botón nuevo, tocás `activity`.

## Por qué el script no corre en la página

La extensión lee solo lo que nombra el manifiesto, con código propio que nunca cambia. Tu script
recibe los valores en la aplicación, lejos de la página. Así una página nunca puede ejecutar tu
código, y tu código nunca puede ver un campo de contraseña, un mensaje o una cookie que el
manifiesto no nombró. Quien revisa puede leer el manifiesto y saber todo lo que lee una
presencia.

## Por qué un script arranca sin nada

El script corre en un motor de JavaScript chico dentro de la aplicación, sin red, sin archivos y
sin `eval`. Cada permiso le abre una puerta a `host`, y la aplicación revisa cada llamada contra
el manifiesto. La gente ve la lista antes de activar una presencia, y una actualización que pide
más espera su respuesta.

## Qué presencia gana

Se muestra una sola actividad a la vez. Cada presencia tiene una prioridad de 0 a 100: la que
eligió la persona, o la que corresponde a su `kind` (`game` 80, `video` 60, `music` 40, `other`
20). Un estado que la persona escribe a mano gana siempre.

- Una presencia en pausa le deja lugar a una que suena.
- `foreground` sube una presencia mientras su ventana tiene el foco.
- Ante un empate se queda la actividad que ya se ve, así la tarjeta no salta.

## Qué hace la privacidad

Antes de que algo llegue a Discord, las reglas de la persona pueden excluir un sitio o un
programa, ocultar los títulos o los botones y enlaces de una presencia, mostrar solo el nombre
del servicio, u ocultar todo después de un rato sin usar el teclado. Excluir un sitio o un
programa mira `domains` y `application` en `meta`, por eso una presencia los declara.

## Por qué los límites

En una computadora que además corre un juego pueden funcionar muchas presencias a la vez. Los
límites de tiempo y memoria evitan que un script lento les cueste a los demás, y un script que
pasa un límite solo pierde esa vuelta.
