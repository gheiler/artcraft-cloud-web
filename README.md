# artcraftCloud Web

Marketing site + deployment wizard for [artcraftCloud](https://artcraftcloud.com).

Rust everywhere: **Axum + SQLite** backend, **Leptos (WASM)** frontend.

## Layout

```
crates/
├── types/    # shared structs (DeployRequest, …) used by server + wizard
├── server/   # Axum API, provider clients, cloud-init generator
└── wizard/   # Leptos CSR frontend (Trunk), mounted at /deploy
```

## Prerequisites

```sh
# one-time
rustup target add wasm32-unknown-unknown
rustup component add rustfmt clippy
cargo install trunk
```

## Run it (development)

You need **two terminals**.

**Terminal 1 — backend (API on http://127.0.0.1:3000):**

```sh
cargo run -p server
```

**Terminal 2 — frontend (http://127.0.0.1:8080):**

```sh
cd crates/wizard
trunk serve
```

`trunk serve` proxies `/api/*` to the backend, so the wizard works end to end.
Open http://127.0.0.1:8080/deploy.

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
INFO server: listening on http://127.0.0.1:3000
INFO server: provisioning started job=job-1 provider=hetzner domain=cloud.example.eu
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
# start a deploy job
curl -s -X POST http://127.0.0.1:3000/api/deploy \
  -H 'content-type: application/json' \
  -d '{"provider":"hetzner","domain":"cloud.example.eu","email":"me@example.eu","api_token":"fake","region":null}'

# poll its status
curl -s http://127.0.0.1:3000/api/deploy/job-1
```

### 6. Tests

```sh
cargo test           # unit + integration tests
cargo test -p types  # one crate only
```

## Architecture notes

- **API tokens are never persisted.** The provider token lives only inside the
  background task for the duration of provisioning.
- `/api/deploy` returns immediately with a `job_id`; the client polls
  `/api/deploy/:id`. This is the hook where SSE progress streaming and the
  "email me when ready" notification will plug in.
- The canonical server install lives in
  `../artcraft-cloud/deploy/cloud-init/artcraft-cloud.yaml` and is shared by
  the one-click wizard and manual CLI users.
