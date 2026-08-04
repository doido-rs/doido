# {doido_name}

A web application built with the [Doido](https://github.com/) framework —
Rails-inspired, Rust-powered (axum + sea-orm).

## Requirements

- Rust — the pinned version is in `mise.toml` (run `mise install`)
- The `doido` CLI

## Getting started

From the project root, use the globally installed `doido` CLI or the local Cargo
alias (`cargo doido` is equivalent to `cargo run --bin {doido_name}`):

```bash
# Create the database and run any pending migrations
cargo doido db create
cargo doido db migrate

# Boot the HTTP server on http://0.0.0.0:3000
cargo doido server
```

Visit <http://0.0.0.0:3000> — `GET /` answers with JSON from `HelloController`:

```json
{ "message": "Hello word!" }
```

## Common commands

| Command | Description |
|---------|-------------|
| `cargo doido server` | Start the web server |
| `cargo doido routes` | Print the route table |
| `cargo doido console` | Start an interactive console |
| `cargo doido db migrate` | Run pending migrations |
| `cargo doido worker` | Run the background job worker |
| `cargo doido generate <gen>` | Run a code generator (run with no args to list them) |

## Layout

```
{doido_name}/
├── config/          ← application.toml, per-env *.yml, routes.rs
├── app/
│   ├── controllers/
│   ├── models/
│   └── views/
├── db/
│   ├── migration/   ← SeaORM migration crate
│   └── schema/
└── tests/
```

## Configuration

Configuration is layered: `config/application.toml` provides the base, and
`config/<env>.yml` (development / test / production) overrides per environment.
Encrypted credentials and `SECTION__KEY` environment variables override on top.

Secrets (`config/master.key`, `config/credentials.yml.enc`) and local databases
are git-ignored by default.

## Testing

```bash
cargo test
```

## Docker

```bash
# Dev stack (web + database [+ redis/memcache when configured])
docker compose up --build

# Production image (distroless runtime)
docker build -t {doido_name} .
```

When using `docker compose`, the `web` service overrides `DATABASE__URL` (and
cache/jobs endpoints when applicable) to reach backends by Docker service name.
Outbound mail uses SMTP to the bundled Mailpit service (`MAILER__SMTP__ADDRESS`);
open the inbox at <http://localhost:8025>. With `cargo doido server` on the host,
`config/development.yml` points SMTP at `localhost:1025` — start Mailpit via
`docker compose up mailpit` (or the full stack).
Run migrations first if needed: `cargo doido db create && cargo doido db migrate`.
{doido_cable_readme}
