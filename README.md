# artcraftCloud Web

The website for [artcraftCloud](https://github.com/gheiler/artcraft-cloud) at
[artcraftcloud.com](https://artcraftcloud.com): what it is, the apps, a comparison of European VPS
hosts, a self-hosting guide, and a deploy wizard that boots your own artcraftCloud.

Rust everywhere, one binary: **Axum** serves server-rendered pages and the deploy API, and a small
**Leptos (Wasm)** app runs only on `/deploy`. Nothing is stored: no database, no accounts, no
cookies, no analytics. See [plan.md](plan.md) for the product plan.

## Layout

```
crates/
├── types/    # shared structs: hosts, DeployRequest, validation, DNS records
├── site/     # the public pages (/, /apps, /hosts, /guide) as Leptos markup, rendered to HTML
│   └── static/assets/   # site.css, fonts, app icons, screenshots, apps.json
├── server/   # Axum: pages, /assets, /deploy, the API, Hetzner client, cloud-init filler
│   └── templates/artcraft-cloud.yaml   # copy of artcraft-cloud's deploy/cloud-init recipe
└── wizard/   # Leptos CSR app for /deploy only (built with trunk)
```

## How deploying works

1. The visitor picks Drive or Drive + Mail, a host, and a domain (or none: the server then
   answers at `https://<ip>.sslip.io` with a real certificate).
2. **One-click (Hetzner today):** they paste a Read & Write API token. The server creates the VM
   with the filled-in cloud-init recipe, trying cheaper types and other EU locations when Hetzner
   has no capacity, waits until it runs, and shows the address, DNS records and a generated admin
   login once. The browser then waits for `https://<domain>/healthz` to answer.
3. **Every other host:** `/api/cloud-init` returns the same recipe filled in, to paste into the
   host's "user data" / "cloud-init" box when ordering.

The recipe is the one in artcraft-cloud (`deploy/cloud-init/artcraft-cloud.yaml`), so people who
install by hand run the same thing. After it changes there, run `scripts/sync-cloud-init.sh`.

**Tokens:** the API token lives only inside the task that creates the server and is dropped when
it ends; it is never written to disk or logged (`DeployRequest`'s `Debug` redacts it). Jobs, and
the generated admin password, stay in memory for an hour and are gone on restart.

## API

| Route | |
| --- | --- |
| `POST /api/cloud-init` | `DeployRequest` JSON -> `{ yaml, url }` |
| `POST /api/deploy` | `DeployRequest` JSON with `api_token` -> `{ job_id }` (one-click hosts only) |
| `GET /api/deploy/{job_id}` | status: `creating`, `booting`, `installing` (server up), `failed` |
| `GET /healthz` | `ok` |

## Prerequisites

```sh
# one-time (rust-toolchain.toml installs the wasm target and components on first use)
cargo install trunk --locked
```

## Run it (development)

```sh
# terminal 1: the wizard, rebuilt on every change
cd crates/wizard && trunk watch

# terminal 2: the site on http://127.0.0.1:3000
ACW_ALLOW_DEMO=1 cargo run -p server
```

Pages and CSS are read on each request (CSS from `crates/site/static`), the wizard from
`crates/wizard/dist`. With `ACW_ALLOW_DEMO=1`, the API token `demo` runs a pretend Hetzner deploy,
handy for trying the wizard without creating a server.

| Variable | Default | |
| --- | --- | --- |
| `ACW_LISTEN` | `127.0.0.1:3000` | address to listen on (`0.0.0.0:8080` in the image) |
| `ACW_STATIC_DIR` | `crates/site/static` | CSS, fonts and images, served at `/assets` |
| `ACW_WIZARD_DIR` | `crates/wizard/dist` | the trunk build, served at `/deploy` |
| `ACW_ALLOW_DEMO` | `0` | `1` enables the `demo` token |
| `ACW_HETZNER_API` | `https://api.hetzner.cloud/v1` | for tests against a mock |
| `RUST_LOG` | `info` | log filter |

## Docker

```sh
docker build -t artcraft-cloud-web .
docker run -d -p 8080:8080 artcraft-cloud-web
```

Put it behind any TLS reverse proxy (Caddy, Traefik). The image runs as non-root on distroless.

## Debugging for Rust newcomers

### 1. Read the compiler — it is your best friend

`cargo build` / `cargo check` print the file, line, and often the exact fix.
`cargo check` is faster than `cargo build` because it skips code generation:

```sh
cargo check          # fast type-check of the whole workspace
cargo build          # full build
cargo clippy         # extra lints / suggestions
cargo fmt            # auto-format
```

Warnings are not errors. Each one points at code that is unused or suspicious.
Fix them before they pile up (this repo currently builds with **zero warnings**).

### 2. Backend logs

The server uses `tracing`. Control verbosity with `RUST_LOG`:

```sh
RUST_LOG=info  cargo run -p server   # default
RUST_LOG=debug cargo run -p server   # verbose
RUST_LOG=tower_http=debug,info cargo run -p server
```

Logs look like:

```
INFO artcraft_cloud_web: listening on http://127.0.0.1:3000
INFO server: deploy started job=3f9c… provider=hetzner mode=Drive demo=true
```

### 3. Panics and backtraces

When Rust panics you get a message + line. For a full stack trace:

```sh
RUST_BACKTRACE=1 cargo run -p server
```

In the browser, the wizard calls `console_error_panic_hook::set_once()` in
`crates/wizard/src/main.rs`, so panics appear as readable errors in the
browser devtools console instead of `unreachable executed`.

### 4. Step-through debugging (breakpoints)

Use **VS Code** with these extensions:

- `rust-analyzer` — autocomplete, inline errors, go-to-definition
- `CodeLLDB` — native debugger

Then add a `.vscode/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug server",
      "cargo": { "args": ["build", "-p", "server"] },
      "cwd": "${workspaceFolder}",
      "env": { "RUST_LOG": "debug" }
    }
  ]
}
```

Set a breakpoint by clicking left of a line number, press **F5**.

For the **frontend**, use the browser devtools (F12): Sources → set breakpoints
in the generated `.wasm` (with debug symbols in a dev build), Console for logs,
Network for `/api` calls.

### 5. Test the API without a browser

```sh
# a filled-in recipe for a guided host
curl -s -X POST http://127.0.0.1:3000/api/cloud-init \
  -H 'content-type: application/json' \
  -d '{"provider":"euronodes","mode":"drive","domain":"cloud.example.eu","email":"me@example.eu"}'

# a pretend one-click deploy (needs ACW_ALLOW_DEMO=1), then poll it with the returned job_id
curl -s -X POST http://127.0.0.1:3000/api/deploy \
  -H 'content-type: application/json' \
  -d '{"provider":"hetzner","mode":"drive","domain":"","skip_domain":true,"api_token":"demo"}'
curl -s http://127.0.0.1:3000/api/deploy/<job_id>
```

### 6. Tests

```sh
cargo test           # unit + integration tests
cargo test -p types  # one crate only
```

## License

MIT OR Apache-2.0, like artcraftCloud.
