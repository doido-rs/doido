# {doido_ext_name}

A [Doido](https://github.com/doido-rs/doido) extension crate — publishable library
with code generators apps install via `cargo add {doido_ext_name}`.

## Development

From this directory:

```sh
cargo test
```

When developing against a local Doido checkout, path dependencies in
`Cargo.toml` point at `{doido_path}`.

## Generators

| Generator | Description |
|-----------|-------------|
| `{doido_ext_snake}:install` | Emits a starter README in the consuming app |

Run from a Doido application after wiring this crate (see below):

```sh
cargo doido generate {doido_ext_snake}:install
```

## Integrating into a Doido app

1. Add the dependency:

```sh
cargo add {doido_ext_name}
```

2. Register generators on the `Doido` builder in `src/main.rs`:

```rust
doido::Doido::new()
    .router(routes::router())
    .register_generator(Box::new({doido_ext_name}::generators::DoidoGenerator))
    // or: {doido_ext_name}::generators::install_on(doido::Doido::new())...
    .run()
    .await;
```

3. Run the install generator:

```sh
cargo doido generate {doido_ext_snake}:install
```

## Publishing

```sh
cargo publish
```

Bump `version` in `Cargo.toml` before each release.
