# Devdeck

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: light)" srcset="docs/screenshots/devdeck-shell-light.png">
    <img src="docs/screenshots/devdeck-shell-dark.png" alt="Devdeck — one native workspace for the terminal, editor, Git, API, database, Docker and agents" width="100%">
  </picture>
</p>

<h1 align="center">Devdeck 🃏</h1>

<p align="center">
  <strong>One deck. Everything dev.</strong><br>
  A native desktop workspace that aims to replace the tab explosion — terminal, editor, Git client, API client, database manager, Docker tools, toolchain manager, secrets manager and AI agent hub — with one fast, extensible, multilingual app.
</p>

<p align="center">
  <a href="https://github.com/hungpt99-dev/Devdeck/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/hungpt99-dev/Devdeck/ci.yml?branch=main&style=flat-square&label=CI" alt="CI status"></a>
  <img src="https://img.shields.io/badge/Rust-1.100%2B-dea584?style=flat-square&logo=rust&logoColor=white" alt="Rust 1.100+">
  <img src="https://img.shields.io/badge/platforms-Linux%20%7C%20macOS%20%7C%20Windows-4b5563?style=flat-square" alt="Linux, macOS and Windows">
  <img src="https://img.shields.io/badge/i18n-en%20%7C%20vi-5b6ee1?style=flat-square" alt="English and Vietnamese">
  <img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-3178c6?style=flat-square" alt="MIT or Apache-2.0">
</p>

<p align="center">
  <a href="#why-devdeck">Why Devdeck</a> ·
  <a href="#what-works-today">What works today</a> ·
  <a href="#run-it">Run it</a> ·
  <a href="#documentation">Docs</a>
</p>

---

Devdeck is early, and it says so plainly: Phase 0/1 is built and verified, and Phase 2 has landed config v2, a real PTY engine, a task runner and read-only tool detection as backends. The window in the picture above is the **actual app**, captured from `main` at `16f2594` under Xvfb in both dark and light themes — not a design mockup. UI panels for the Phase 2 engines are still being wired up. See [docs/progress.md](docs/progress.md) for the exact verified state and gaps.

**Local-first, by construction.** Config lives in `~/.devdeck`, telemetry is off unless you opt in, and every capability is a typed tool behind a permission decision with a hash-chained, redacted audit log. Extensions and agents are meant to join through the same contribution points as built-ins — no private back doors — but the extension host and MCP server are not shipped yet.

## Why Devdeck

- **One native core, not a web view.** 100% Rust, GPU-rendered with Iced, `#![forbid(unsafe_code)]` across the workspace. No Electron, Tauri or JavaScript in the host.
- **A tool boundary, not a pile of features.** One typed registry supplies each tool's schema, permission level, CLI command, MCP tool, palette entry and localized label.
- **Your machine, your data.** Versioned, atomic config writes under an app-owned directory, deny-by-default permissions, append-only audit with secret redaction.
- **Multilingual from day one.** English and Vietnamese switch at runtime; dark, light and high-contrast themes are first-class.
- **Built for the agent era.** The plan is authenticated localhost-only MCP, agent clients with permission prompts and diff review — designed, not yet implemented.

## What works today

| Area | In the tree now | Still ahead |
| --- | --- | --- |
| Native shell | Header, sidebar, tab/content area, status bar; command palette (Ctrl/Cmd+K), theme and language buttons | Six-region layout, resizable splits, true overlay palette |
| Tools & safety | Typed `files.read`, deny-by-default policy, audit log, JSON Schema export via CLI | Extension host, WebAssembly isolation, permission prompts in UI |
| Config & projects | Schema v2 with atomic writes, bounded recents, en/vi | Cross-project workspace state |
| Terminal & tasks | PTY engine with bounded scrollback, redaction and typed `terminal.*` tools; `devdeck.toml` task runner with `tasks.list`/`tasks.run` | Terminal panes, task UI, approval dialog |
| Detection | Read-only `tools.detect` / `tools.list_detected` with typed failure reasons | Install controls (consent-gated, later phase) |
| i18n & themes | Fluent en/vi bundles, runtime switch, high-contrast tokens | RTL and full ICU4X formatting |
| Editor, Git, API, DB, Docker, agents | Not implemented | Phase 3+ roadmap |

## Run it

Linux needs X11/Wayland, xkbcommon, fontconfig, freetype and ALSA development packages.

```bash
cargo build --workspace
cargo test --workspace
cargo run -p devdeck-app
cargo run -p devdeck-cli -- doctor --lang vi
cargo run -p devdeck-xtask -- i18n-lint
```

The screenshots in `docs/screenshots/` were captured from the running binary under `xvfb-run`; the capture is reproducible from the build above.

## Documentation

- [VISION.md](VISION.md) — product vision, constraints and roadmap
- [docs/progress.md](docs/progress.md) — verified status, gaps and next steps
- [docs/adr/](docs/adr/) — architecture decision records
- [docs/security/threat-model.md](docs/security/threat-model.md) — threat model v1
- [CONTRIBUTING.md](CONTRIBUTING.md) · [SECURITY.md](SECURITY.md) · [CHANGELOG.md](CHANGELOG.md)

## License

Dual-licensed MIT OR Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
