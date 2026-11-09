# Política de privacidad

Última actualización: 2026-11-04. Se aplica a la aplicación de escritorio Rich Presence Plus, su extensión de navegador, su catálogo y este sitio web.

## En resumen

Rich Presence Plus muestra en tu perfil de Discord lo que jugás, ves o escuchás. No tiene cuentas, ni analíticas, ni servidores propios, y el proyecto no recibe ninguno de los datos descritos acá. Funciona en tu computadora. Lo que sale de ella es la actividad que publicás en Discord y las solicitudes que se listan abajo.

## Qué lee la aplicación

La aplicación lee solo lo que cada presencia que activás declara en su manifiesto. Podés ver esa lista antes de activarla.

| Fuente | Qué lee | Quién lo usa |
| --- | --- | --- |
| Páginas del navegador | La dirección, el título y los valores que la presencia declara, como el título de un video o la posición de reproducción. Algunas presencias toman uno de esos valores de lo que la página carga de su sitio, como el nombre de una canción; el resto de lo que la página carga no se guarda | Presencias web |
| Programas abiertos | Sus nombres | Presencias que lo declaran |
| Ventana activa | Su programa y título | Presencias que lo declaran |
| Sesión multimedia | El título, el artista y el estado que informa Windows | Presencias que lo declaran |
| Archivos | Archivos de las carpetas que lista el manifiesto, como los registros de un juego | Presencias que lo declaran |
| Líneas de comando | Solo las opciones que lista el manifiesto, de un programa. Otras opciones, como contraseñas, nunca se devuelven | Presencias que lo declaran |
| Discord | Tu nombre de usuario, por la conexión local, para mostrar qué cuenta está en uso | La aplicación |

Las presencias corren aisladas y no pueden leer nada fuera de su lista declarada.

## Qué lee la extensión del navegador

La extensión funciona en los sitios que permitís, de a uno, para las presencias que activás. No lee nada en ventanas privadas, ni mientras la pausás.

Lo que lee lo envía solo a la aplicación en tu computadora, por `127.0.0.1`, en una conexión que ambos lados autentican y que está cifrada. La extensión no envía nada a ningún servidor.

## Qué sale de tu computadora

- **Discord.** La aplicación le da tu actividad a la app de Discord en tu computadora, y Discord la muestra en tu perfil a quien tu configuración de Discord permita. Primero se aplican tus reglas: sitios y programas excluidos, títulos, botones, enlaces y contadores ocultos, el modo "solo el servicio", la pausa y ocultar mientras estás ausente. Discord trata estos datos según su propia [política de privacidad](https://discord.com/privacy). Las imágenes y enlaces de una actividad son direcciones que Discord puede cargar.
- **GitHub.** La aplicación le pide a GitHub las versiones nuevas y el catálogo de presencias, y descarga lo que instalás. GitHub ve tu dirección IP, como cualquier sitio al que te conectás, según su [declaración de privacidad](https://docs.github.com/site-policy/privacy-policies/github-general-privacy-statement). Los contadores de descargas son totales públicos que no dicen quién descargó. Tu actividad no se envía.
- **Hosts que declara una presencia.** Una presencia puede pedir datos públicos a los hosts de su manifiesto, como el nombre de un juego, o pedirle a un servidor de juego su información pública. Esos servidores ven tu dirección IP y la solicitud.
- **Imágenes.** Los logos de las presencias son archivos de la aplicación. La ventana también puede mostrar la imagen de una actividad, como la miniatura de un video, que se carga del sitio que la aloja. Ese sitio ve tu dirección IP y la dirección de la imagen.

No se vende nada, y nada se usa para publicidad ni para perfilar. La aplicación no toma decisiones automatizadas sobre vos.

Si activás Overlays y Stream Deck en Ajustes, los programas de tu computadora pueden leer lo que muestra Discord por una dirección local. Otras computadoras y los sitios web no pueden.

## Qué queda en tu computadora

| Dato | Dónde | Cuánto tiempo |
| --- | --- | --- |
| Ajustes, favoritos, estados personalizados y perfiles | Carpeta de datos de la aplicación | Hasta que los borres o desinstales |
| Clave de emparejamiento | Carpeta de datos de la aplicación y almacenamiento de la extensión | Hasta que desinstales o quites la extensión |
| Registros, sin direcciones de páginas, títulos ni actividades | Carpeta de datos de la aplicación | Siete archivos diarios |
| Presencias instaladas | Carpeta de datos de la aplicación | Hasta que las quites; se guardan dos versiones anteriores |
| Versiones de desarrollo, casos de prueba y exportaciones | Carpeta de datos de la aplicación | Hasta 30 versiones por presencia; exportaciones hasta que las borres |
| Tiempo a la vista de cada presencia, solo si lo activás en Tu semana: nombres y minutos | Carpeta de datos de la aplicación | 56 días, o hasta que lo borres |
| Sesiones de navegador que muestra la aplicación | Memoria | Hasta cerrar la aplicación |
| Caché de la interfaz | Carpeta de datos de la aplicación, por WebView2 | Hasta desinstalar |

El instalador también puede agregar entradas en tu registro de usuario: iniciar con Windows, si lo elegís, y las entradas que permiten a los navegadores encontrar la aplicación. Desinstalar las quita. Tus datos se borran si lo elegís en el desinstalador. Quitar la extensión borra sus datos del navegador.

## Este sitio web

El sitio no usa cookies, ni analíticas, ni publicidad, y no carga nada de otros servidores. Está alojado en GitHub Pages, que guarda registros técnicos, incluidas direcciones IP, según la declaración de GitHub enlazada arriba. Los enlaces a otros sitios siguen las políticas de esos sitios.

## Tus derechos

El proyecto no tiene datos personales tuyos, así que no hay nada que pueda consultar, corregir o borrar. Vos controlás todo lo que se guarda en tu computadora y podés borrarlo cuando quieras. Para los datos que tienen Discord o GitHub, usá sus herramientas.

Si te aplica una ley de protección de datos, conservás tus derechos, incluido el de reclamar ante tu autoridad de control. En Argentina es la Agencia de Acceso a la Información Pública ([AAIP](https://www.argentina.gob.ar/aaip/datospersonales)).

## Menores

Rich Presence Plus no está dirigida a menores. Usarla requiere una cuenta de Discord, así que tenés que cumplir la edad mínima que Discord fija en tu país.

## Chrome Web Store

El uso de la información recibida por la extensión cumple la [Política de datos de usuario de Chrome Web Store](https://developer.chrome.com/docs/webstore/program-policies/), incluidos los requisitos de Uso Limitado. Los datos se usan solo para mostrar tu actividad, y ninguna persona los lee.

## Cambios y contacto

Los cambios se publican acá con una fecha nueva, y los importantes se listan en las notas de versión. Para consultas, abrí un issue en [github.com/manucabral/RichPresencePlus](https://github.com/manucabral/RichPresencePlus/issues).
