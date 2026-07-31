# docs-blog — the Doido manual & blog

A static site (documentation + blog) for the Doido framework, built with
[Zola](https://www.getzola.org/) — a static-site generator written in Rust that
uses the `pulldown-cmark` markdown engine. It is deployed to GitHub Pages at
**https://doido-rs.github.io** by GitHub Actions.

```
docs-blog/
├── config.toml            # site + theme configuration (the control panel)
├── content/
│   ├── _index.md          # home page
│   ├── docs/              # documentation section (the manual)
│   │   ├── _index.md
│   │   ├── getting-started.md
│   │   ├── installation.md
│   │   ├── cli.md
│   │   └── guides/        # per-subsystem guides
│   └── blog/              # blog posts (one .md per post)
├── static/                # copied verbatim to the site root (CNAME, favicon, js)
└── themes/doido/          # the in-repo, swappable theme
```

## Run it locally

Install Zola (a single binary — see the
[install guide](https://www.getzola.org/documentation/getting-started/installation/)),
then:

```bash
cd docs-blog
zola serve          # live preview at http://127.0.0.1:1111
zola build          # production build into ./public
zola check          # validate internal links without writing output
```

## Add a blog post

Create `content/blog/YYYY-MM-DD-your-slug.md` — the date in the filename sets the
post date and is stripped from the URL (so `2026-08-01-hello.md` → `/blog/hello/`):

```markdown
+++
title = "Your title"
date = 2026-08-01
description = "One-line summary shown in listings and meta tags."

[taxonomies]
tags = ["release", "announcements"]
+++

Your Markdown content.
```

Push to `master` and GitHub Actions builds and deploys it. That's it.

## Add a documentation page

Create `content/docs/<name>.md` (or a new file under `content/docs/guides/`):

```markdown
+++
title = "Page title"
description = "Shown in the sidebar tooltip and meta tags."
weight = 5          # controls sidebar order (lower = higher up)
+++

Your content. Link to other docs with `[text](@/docs/other-page.md)`.
```

The sidebar and prev/next navigation update automatically.

## Translate a page (English · Português · Español)

The site is multilingual. English is the default (served at the root, e.g.
`/docs/`); Portuguese and Spanish are served under a prefix (`/pt/…`, `/es/…`). A
switcher in the header links between the translated versions of the current page.

Translate any page by adding a language-suffixed file next to the original:

```
content/docs/installation.md      # English (default)  → /docs/installation/
content/docs/installation.pt.md   # Português          → /pt/docs/installation/
content/docs/installation.es.md   # Español            → /es/docs/installation/
```

Keep the same front-matter keys (`weight`, `sort_by`, …). When linking between
docs **inside a translation**, use the language-suffixed target so the link stays
in that language:

```markdown
<!-- inside installation.pt.md -->
[Primeiros passos](@/docs/getting-started.pt.md)
```

To add or remove a language, edit the `[languages.*]` / `[*.translations]` tables
and the `[extra].languages` switcher list in [`config.toml`](./config.toml). UI
strings (nav labels, “min read”, the theme options, …) live in the
`[*.translations]` tables and are read in templates via `trans(key=…, lang=lang)`.

## Change the theme

The theme is **configurable** and **updatable**:

- **Light / dark:** every visitor picks Light or Dark from the selector in the
  header; the choice persists in `localStorage`. Until they choose, the theme is
  seeded by `theme_default` (`auto` follows the OS on the first visit, or set it to
  `light` / `dark`).
- **Restyle without touching templates:** edit the `[extra]` block in
  [`config.toml`](./config.toml) — `accent` / `accent_dark` (primary brand colour
  per mode), `accent2` / `accent2_dark` (secondary highlight), `font_sans` /
  `font_mono`, `theme_default`, the `logo_text`, and the `[[extra.nav]]` links. The
  default palette is the green of the Brazilian flag with a lemon-yellow highlight,
  and the font is **Ubuntu Mono** (loaded from Google Fonts in
  `themes/doido/templates/partials/head.html`). The syntax-highlighting theme is
  set with `highlight_theme` under `[markdown]`.
- **Deeper structural colours** (backgrounds, borders, text) live as CSS custom
  properties in [`themes/doido/sass/_variables.scss`](./themes/doido/sass/_variables.scss).
- **Swap the theme entirely:** drop another Zola theme into `themes/` and change
  `theme = "..."` in `config.toml`.

## Deployment

[`.github/workflows/docs-blog.yml`](../.github/workflows/docs-blog.yml) builds the
site with Zola and publishes the output to the organisation root-site repo
[`doido-rs/doido-rs.github.io`](https://github.com/doido-rs/doido-rs.github.io),
which GitHub Pages serves at the domain root **https://doido-rs.github.io**. It runs
on every push to `master` that touches `docs-blog/**` (and on manual dispatch).
`base_url` in `config.toml` matches that URL.

### One-time setup (already configured)

Done once, outside the codebase:

1. **Deploy key:** the `doido-rs.github.io` repo has a read-write deploy key whose
   private half is stored in this repo as the `ACTIONS_DEPLOY_KEY` secret; the
   workflow pushes the built site with it.
2. **Pages source:** on `doido-rs.github.io`, GitHub → Settings → Pages → Source:
   "Deploy from a branch" → `main` / `/` (root).
