+++
title = "Apresentando o manual do Doido"
description = "Uma casa para a documentação do framework e as novidades do projeto — feita com Zola e publicada pelo GitHub Actions."
date = 2026-07-28

[taxonomies]
tags = ["announcements", "docs"]
+++

O Doido agora tem uma casa na web: um único site que é, ao mesmo tempo, o
**manual do framework** e o lugar onde publicamos as **novidades do projeto**.
Você está lendo o primeiro post dele.

## Por que um site de manual

O design do Doido sempre foi guiado por especificações detalhadas na pasta `docs/`
do repositório. Essas especificações são a fonte da verdade para a *intenção de
design* — mas nunca foram pensadas como a porta de entrada para quem só quer
instalar a CLI, gerar um scaffold e publicar. Este site preenche essa lacuna: a
[seção de documentação](/pt/docs/) é um manual curado, escrito à mão, que aponta
de volta para as especificações sempre que você quiser se aprofundar.

## Como ele é feito

O site é propositalmente sem graça de operar:

- **[Zola](https://www.getzola.org/)** — um gerador de sites estáticos escrito em
  Rust (usa a engine de markdown `pulldown-cmark`). Um binário, sem runtime.
- **Markdown do começo ao fim.** Cada página de doc e post é um arquivo `.md`.
- **GitHub Actions** faz o build do site e publica no GitHub Pages a cada push na
  `master` que toca em `docs-blog/`.
- **Um tema configurável.** Cores, fontes, navegação e o modo claro/escuro padrão
  ficam todos no `config.toml` — restilizar nunca significa editar um template.

## Publicar é um arquivo

Adicionar um post é deliberadamente trivial. Solte um arquivo Markdown em
`docs-blog/content/blog/` nomeado `AAAA-MM-DD-seu-slug.md`:

```markdown
+++
title = "Título do meu post"
date = 2026-08-01

[taxonomies]
tags = ["release"]
+++

Seu conteúdo aqui.
```

Faça push na `master`, e o GitHub Actions faz o build e publica. Esse é o fluxo
inteiro.

## O que vem a seguir

Mais guias de subsistemas, notas de release conforme os crates chegam ao
crates.io, e o que mais valer a pena registrar. Acompanhe pelo
[feed Atom](/pt/blog/atom.xml) ou no [GitHub](https://github.com/doido-rs/doido).
