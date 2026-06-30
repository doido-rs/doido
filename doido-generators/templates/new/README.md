# {doido_name}

A web application built with the [Doido](https://github.com/) framework —
Rails-inspired, Rust-powered (axum + sea-orm).

## Requirements

- Rust (stable) — see `.tool-versions`
- The `doido` CLI

## Getting started

```bash
# Create the database and run any pending migrations
doido db create
doido db migrate

# Boot the HTTP server on http://0.0.0.0:3000
doido server
```

Visit <http://0.0.0.0:3000> — `GET /` answers with JSON from `HelloController`:

```json
{ "message": "Hello word!" }
```

## Common commands

| Command | Description |
|---------|-------------|
| `doido server` | Start the web server |
| `doido routes` | Print the route table |
| `doido console` | Start an interactive console |
| `doido db migrate` | Run pending migrations |
| `doido worker` | Run the background job worker |
| `doido generate <gen>` | Run a code generator (run with no args to list them) |

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
