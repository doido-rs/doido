+++
title = "Presentando el manual de Doido"
description = "Un hogar para la documentación del framework y las novedades del proyecto — hecho con Zola y publicado por GitHub Actions."
date = 2026-07-28

[taxonomies]
tags = ["announcements", "docs"]
+++

Doido ya tiene un hogar en la web: un único sitio que es, a la vez, el **manual
del framework** y el lugar donde publicamos las **novedades del proyecto**. Estás
leyendo su primera entrada.

## Por qué un sitio de manual

El diseño de Doido siempre se ha guiado por especificaciones detalladas en la
carpeta `docs/` del repositorio. Esas especificaciones son la fuente de verdad
para la *intención de diseño* — pero nunca se pensaron como la puerta de entrada
para quien solo quiere instalar la CLI, generar un scaffold y publicar. Este sitio
llena ese vacío: la [sección de documentación](/es/docs/) es un manual curado,
escrito a mano, que enlaza de vuelta a las especificaciones cuando quieras
profundizar.

## Cómo está construido

El sitio es deliberadamente aburrido de operar:

- **[Zola](https://www.getzola.org/)** — un generador de sitios estáticos escrito
  en Rust (usa el motor de markdown `pulldown-cmark`). Un binario, sin runtime.
- **Markdown de principio a fin.** Cada página de doc y entrada es un archivo `.md`.
- **GitHub Actions** construye el sitio y lo publica en GitHub Pages en cada push a
  `master` que toca `docs-blog/`.
- **Un tema configurable.** Colores, fuentes, navegación y el modo claro/oscuro por
  defecto están todos en `config.toml` — restilizar nunca significa editar una
  plantilla.

## Publicar es un archivo

Añadir una entrada es deliberadamente trivial. Coloca un archivo Markdown en
`docs-blog/content/blog/` con el nombre `AAAA-MM-DD-tu-slug.md`:

```markdown
+++
title = "Título de mi entrada"
date = 2026-08-01

[taxonomies]
tags = ["release"]
+++

Tu contenido aquí.
```

Haz push a `master`, y GitHub Actions construye y publica. Ese es todo el flujo.

## Qué sigue

Más guías de subsistemas, notas de release a medida que los crates llegan a
crates.io, y lo que valga la pena documentar. Sigue el proyecto por el
[feed Atom](/es/blog/atom.xml) o en [GitHub](https://github.com/doido-rs/doido).
