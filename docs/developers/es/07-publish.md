---
title: Publicar
description: Proponé tu presencia para el catálogo de Rich Presence Plus con un pull request, y que la pueda instalar cualquiera que tenga la app.
---

# Publicar

Una presencia que escribís funciona en tu computadora. Para que la pueda instalar todo el mundo,
proponela para el catálogo. Primero la lee una persona, y cuando se aprueba cualquiera que tenga
la aplicación la descarga desde la página Catálogo.

## Comprobar que esté lista

Abrí la pestaña Publicar de la página Desarrollo y elegí tu presencia. Comprueba estas reglas, las
mismas que controla GitHub:

- El nombre de la carpeta usa minúsculas, números y guiones.
- La versión tiene la forma `1.2.0`.
- Tiene descripción, autor y al menos una etiqueta de la lista.
- Tiene `locales/en.json`.
- Tiene hasta 128 archivos, 1 MB cada uno y 8 MB en total.
- Ningún archivo guarda una contraseña o una clave.

Corregí lo que esté marcado y tocá "Comprobar de nuevo".

## Enviarla

Necesitás una cuenta de GitHub.

1. Tocá "Preparar carpeta". La aplicación hace una copia limpia, sin los archivos que usa solo tu
   editor, y la muestra en el Explorador.
2. Tocá "Abrir GitHub". La primera vez, la página te pide hacer tu propia copia del proyecto.
3. Arrastrá la carpeta a la página.
4. Describí tu presencia y creá el pull request.

GitHub vuelve a controlar las reglas solo, en unos minutos.

## Qué mira la revisión

Una persona lee tu script y tus permisos antes de aprobarla. Una presencia se rechaza si:

- Pide un sitio, un programa o un permiso que no usa.
- Manda lo que lee a otro lugar que no sea Discord.
- Muestra datos privados, como mensajes, nombres de cuentas o direcciones.
- Copia el nombre o el aspecto de algo que no es.

La revisión puede pedir cambios en el pull request. Cuando se aprueba, la presencia llega al
catálogo con la siguiente publicación.

## Actualizar tu presencia

Cambiá tu carpeta, subí `version` y repetí los pasos. Quienes la instalaron reciben la versión
nueva desde la página Catálogo. Si pide más acceso que antes, primero lo tienen que aceptar, y la
versión anterior sigue funcionando hasta entonces.
