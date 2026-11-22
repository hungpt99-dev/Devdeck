# Devdeck

**One deck. Everything dev.**

Devdeck is one native desktop application that replaces the collection of tools a developer installs every day: terminal, code editor, Git, API client, database manager, Docker, Kubernetes, toolchain manager, env/secrets manager, debugging and testing tools, utilities, and an AI agent hub.

Devdeck is also a **platform**: a small, fast, stable core and a safe extension system. Anyone can add tools, languages, themes, integrations and agents without forking the app. It is multilingual from day one and built for the AI-agent era: agents are first-class users, humans supervise, extensions are first-class citizens.

## Non-negotiable constraints

1. 100% Rust core. No Tauri, Electron, webview or JavaScript in the host. Extensions may compile to WebAssembly components.
2. GPU-rendered UI; target cold start < 1 s, idle core RAM < 150 MB, 60+ fps, keystroke-to-screen < 16 ms. No I/O on the UI thread: tokio + channels/events.
3. Design tokens, dark/light/high-contrast themes, reduced-motion support, Ctrl/Cmd+K command palette, keyboard-first, resizable splits, HiDPI and multi-monitor.
4. Windows 10/11, macOS (current + previous two majors), Linux X11 and Wayland. OS code in isolated modules; software-rendering fallback.
5. All user-facing text localized; runtime language switch; Unicode/IME/RTL; en + vi in the first public release.
6. Everything Devdeck installs lives under an app-owned directory (`~/.devdeck/...`). No permanent PATH changes without explicit consent.
7. MIT/Apache-2.0-compatible dependencies only; `cargo-deny` enforced; new dependencies justified in PRs.
8. Local-first: versioned, Git-friendly config; telemetry OFF unless opted in.
9. A crashing, slow or malicious extension or agent must never crash, freeze or compromise the host.
10. Atomic writes, autosave/recovery, versioned config migrations — never lose user data.

## Architecture

`crates/core` (state, config, event bus) → `crates/tools` (typed tool schemas, single source of truth) → `crates/extension-api` (contribution points) → feature crates → `crates/ui` → `app`/`cli`. Core never depends on UI. Built-in modules register through the same contribution points as extensions; no private back doors. Every capability is a typed tool with permission level, risk level, CLI subcommand, MCP tool, palette entry and localized display text.

Extension runtime: WebAssembly components (wasmtime + versioned WIT worlds), capability-based permissions, deny by default, crash isolation; out-of-process native sidecars over JSON-RPC/stdio for LSP/DAP/MCP adapters; Rhai for small automations.

AI: built-in authenticated localhost-only MCP server, headless CLI, ACP/MCP agent clients, permission prompts, checkpoints, diff review, append-only audit log, secret redaction, cost meter and prompt-injection defenses.

## Tool tiers

- P0: terminal, editor, Git, API client, database, Docker, toolchains, task runner, env/secrets, agent hub, extensions manager.
- P1: Kubernetes, SSH/SFTP, DAP debugger, test runner, logs, linters, Git hosting integrations, process/resource monitor, network tools, hosts/certificates.
- P2: offline developer toolbox, cloud helpers, observability, profiler views.

## Roadmap

Phase 0 spike/ADRs → 1 foundation → 2 terminal/task runner/extension host MVP → 3 Git/declarative UI → 3.5 MCP → 4 Docker/toolchains/secrets/SDK → 4.5 API client → 5 database → 6 editor/LSP → 6.5 agent panel → 7 hardening/public beta → 7.5 registry/parallel agents → 8 P1 → 9 P2/1.0.

Current verified status is in [docs/progress.md](docs/progress.md). Decisions are in [docs/adr/](docs/adr/).
