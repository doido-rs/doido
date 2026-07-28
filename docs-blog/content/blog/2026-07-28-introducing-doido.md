+++
title = "Introducing the Doido manual"
description = "A home for the framework's documentation and project news — built with Zola and deployed by GitHub Actions."
date = 2026-07-28

[taxonomies]
tags = ["announcements", "docs"]
+++

Doido now has a home on the web: a single site that is both the **framework
manual** and the place where we publish **project news**. You are reading its
first post.

## Why a manual site

Doido's design has always been driven by detailed specs in the repository's
`docs/` folder. Those specs are the source of truth for *design intent* — but
they were never meant to be the front door for someone who just wants to install
the CLI, generate a scaffold, and ship. This site fills that gap: the
[documentation section](/docs/) is a curated, hand-written manual that links back
to the specs whenever you want to go deeper.

## How it is built

The site is intentionally boring to operate:

- **[Zola](https://www.getzola.org/)** — a static-site generator written in Rust
  (it uses the `pulldown-cmark` markdown engine). One binary, no runtime.
- **Markdown all the way down.** Every doc page and blog post is a `.md` file.
- **GitHub Actions** builds the site and deploys it to GitHub Pages on every push
  to `master` that touches `docs-blog/`.
- **A configurable theme.** Colours, fonts, navigation, and the default light/dark
  mode all live in `config.toml` — restyling never means editing a template.

## Publishing is one file

Adding a post is deliberately trivial. Drop a Markdown file into
`docs-blog/content/blog/` named `YYYY-MM-DD-your-slug.md`:

```markdown
+++
title = "My post title"
date = 2026-08-01

[taxonomies]
tags = ["release"]
+++

Your content here.
```

Push to `master`, and GitHub Actions builds and deploys it. That is the whole
workflow.

## What's next

More subsystem guides, release notes as the crates land on crates.io, and
whatever else is worth writing down. Follow along via the
[Atom feed](/blog/atom.xml) or on [GitHub](https://github.com/doido-rs/doido).
