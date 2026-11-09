---
title: Opciones e idiomas
description: Dejá que cada persona cambie cómo se comporta una presencia con opciones, y mostrá sus textos en varios idiomas.
---

# Opciones e idiomas

## Agregar una opción

Las opciones dejan que cada persona cambie cómo se comporta la presencia. La aplicación dibuja
el control en la tarjeta de la presencia, y tu código lee el valor.

1. Declarala en el manifiesto:

   ```json
   "options": [
     { "type": "boolean", "key": "show_title", "default": true }
   ]
   ```

2. Ponele una etiqueta en `locales/es.json`, bajo `option.` y su clave:

   ```json
   "option.show_title": "Mostrar el título del video"
   ```

3. Leela en `read`:

   ```js
   function read(context) {
     const page = bestPage(context.pages);
     if (!page) return null;
     return { title: context.options.show_title ? page.title : t("private") };
   }
   ```

Guardá y abrí la página Presencias: el interruptor está en la tarjeta de la presencia. Los otros
tipos, `number`, `text` y `choice`, están en [Manifiesto](08-manifest.md#opciones).

## Mostrar una opción solo cuando otra está activa

Agregá `when` con la clave de una opción booleana:

```json
"options": [
  { "type": "boolean", "key": "show_title", "default": true },
  { "type": "number", "key": "title_length", "default": 64, "min": 16, "max": 128, "when": "show_title" }
]
```

`title_length` se oculta mientras `show_title` está apagada. Tu script igual recibe su valor.

## Mostrar una opción solo para algunos valores

Dale a `when` las claves de otras opciones y el valor que tiene que tener cada una, o una lista
de valores:

```json
"options": [
  { "type": "choice", "key": "image", "default": "logo", "choices": ["logo", "cover", "none"] },
  { "type": "text", "key": "caption", "default": "", "when": { "image": ["cover", "logo"] } }
]
```

`caption` se muestra mientras `image` es `cover` o `logo`. Con varias claves, tienen que cumplirse
todas. Cada valor tiene que ser uno que la otra opción puede tomar, y `when` mira de 1 a 4
opciones.

## Explicar una opción

Una línea debajo de la opción dice qué hace. Se escribe en los archivos de idioma, bajo `help.` y
su clave:

```json
"help.caption": "El texto que se ve al pasar el mouse por la imagen."
```

## Ofrecer una lista de opciones

```json
{ "type": "choice", "key": "line", "default": "channel", "choices": ["channel", "views"] }
```

Cada valor toma su etiqueta de `option.<clave>.<valor>`:

```json
"option.line": "Segunda línea",
"option.line.channel": "Canal",
"option.line.views": "Reproducciones"
```

## Escribir los textos por clave

El script no escribe sus textos directamente. Los pide por clave con `t`, así que el mismo
código habla todos los idiomas que tenga la carpeta.

```json
{
  "description": "Muestra el video que estás viendo.",
  "by": "De {name}"
}
```

```js
t("by", { name: "Manu" });   // "De Manu"
```

`description` también es la descripción de la presencia en ese idioma.

## Agregar un idioma

1. Copiá `locales/en.json` a `locales/es.json`.
2. Traducí los valores. Dejá las claves y las partes `{name}` como están.

La aplicación elige el archivo del idioma de la persona. Una clave que falta ahí se muestra en
inglés, y una que falta en inglés se muestra como la clave misma.
