# MToken GUI Tauri + Vanilla

MToken is the main adminstrative GUI for mtoken to manage configuration with FPGA control.


Tauri + Vanilla will help get you started developing with Tauri in vanilla HTML, CSS and Javascript.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) 
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)


npm create tauri-app@latest

> npx
> create-tauri-app

✔ Project name · mtokengui
✔ Identifier · com.mtoken.mtokengui
✔ Choose which language to use for your frontend · Rust - (cargo)
✔ Choose your UI template · Vanilla

Template created!

Your system is missing dependencies (or they do not exist in $PATH):
╭───────────┬─────────────────────────────────────────────────────────╮
│ Tauri CLI │ Run `cargo install tauri-cli --version ^2.0.0 --locked` │
╰───────────┴─────────────────────────────────────────────────────────╯

Make sure you have installed the prerequisites for your OS: https://tauri.app/start/prerequisites/, then run:
  cd mtokengui
  cargo tauri android init
  cargo tauri ios init

For Desktop development, run:
  cargo tauri dev

For Android development, run:
  cargo tauri android dev

For iOS development, run:
  cargo tauri ios dev


## Redis and PostgreSQL

Settings are updated in the Rust memory cache immediately. A bounded background
queue then commits each settings snapshot to Redis first and PostgreSQL second.
Configure these variables in the shell before starting Tauri:

```sh
export REDIS_URL=redis://127.0.0.1:6379/
export DATABASE_URL=postgresql://postgres:postgres@127.0.0.1:5432/mtoken
cargo tauri dev
```

The PostgreSQL worker creates the `settings` table automatically. Redis stores
the current JSON snapshot under the `mtoken:settings` key. If either service is
unavailable, the cache remains available and the worker retries the queued
snapshot three times.


## rust_hdl

We use rust_hdl framework to load config into FPGA 