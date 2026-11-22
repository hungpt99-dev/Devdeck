# Deliverable 4 — Repository reuse assessment

## Decision rule

Reuse code only when the licence is compatible and the component is decoupled from a webview/GPL stack; otherwise study the design and reimplement the interface clean-room. "Implemented" means verified in source, not README claims.

| Repository | Verified implementation | Licence / redistribution | Verdict |
| --- | --- | --- | --- |
| OpenIDEA[1] | Tauri 2 shell + React 19 + CodeMirror 6 + xterm.js[3][8]; 47 Rust crates with own LSP/DAP clients over `ide-jsonrpc`; wasmtime plugin components | Apache-2.0 with JetBrains/VS Code attributions; one MPL-2.0 dependency | **Selective reuse of Rust backend crates** (`ide-jsonrpc`, `ide-lsp`, `ide-dap`, `ide-terminal`, `ide-vcs`, `ide-db`, `ide-http`) as reference or extracted crates after decoupling from Tauri/Specta IPC; **no UI code reuse** |
| Dodo[37] | Native GPUI via gpui-kit 0.6; 16 workspace packages; Docker/Podman, DB (Postgres/SQLite/MySQL/Redis), API explorer with QuickJS sandbox; purity-based layering enforced by test[43] | MIT source but GPL-3.0-or-later chain (gpui → sum_tree → ztracing/zlog) linked into binaries; upstream leaves redistribution unresolved[40][41] | **Study patterns, do not copy GPUI-linked code.** Reuse ideas: pure-layer tripwire, typed global state, dialog reconciliation, engine socket discovery, bounded logs, build profiles. Legal review required before any code reuse |
| Navop[98] | Genuine native GPUI app (59 crates) with framed JSON-RPC DB drivers, russh SSH/SFTP, agent runtime, four-tier tool permissions; README "No WebView" contradicted by `wry` use in AI HTML blocks | Apache-2.0 + Navop Supplementary Licence: no fee-based distribution, no redistribution as component of commercial suite, explicit no-competing-products clause[103] | **Clean-room architecture study only — no code copying.** Patterns worth studying: typed `DbValue`, permission matrix, loopback MCP token auth, transfer scheduler |

## Third-party libraries to reuse directly

| Need | Preferred crate / engine | Why |
| --- | --- | --- |
| UI | Iced 0.14.0 (MIT) | builds today, IME preedit, tiny-skia fallback; exact pinned[302] |
| WASM plugins | wasmtime component model[345] | reference implementation, capability model |
| LSP / DAP | `lsp-types`/tower-lsp, DAP spec clients[361][364] | protocol standards; external servers |
| Git | `gix` with git CLI fallback | GitKraken itself migrated to core Git; reuse the engine |
| Containers | `bollard` + Docker/Podman CLI | mature engine adapters |
| Databases | `rusqlite`/`sqlx`, `tokio-postgres`, `mysql_async`, Redis client | protocol drivers instead of hand-written wire code |
| API | `reqwest`, `tonic`, `tokio-tungstenite`, Rhai | transport + scripting engines |
| Text/editing | `ropey`, `tree-sitter`, `cosmic-text` | proven editing/shaping primitives |
| PTY | `portable-pty`, `alacritty_terminal` | terminal emulation without webview |
| i18n | `fluent`, `icu` | typed keys, plurals, locale data |
| Measurement | `criterion`, `hyperfine`[609][612] | reproducible benchmarks |

## Build ourselves

Typed tool schema and registry; permission policy and approval UX; hash-chained audit; config schema/migrations and single-writer store; workspace/profile model; contribution-point types and extension host; adapter orchestration and provenance ledger; domain-specific workspaces that make tools usable (diff review, result grids, request builders, container lifecycle).

## Do not build

Package managers (winget/Homebrew/APT/npm/Cargo adapters only)[485][496]; container engines; database servers; LSP/DAP servers; webview/JS UI; custom text shaper; deep clones of category leaders.

## Licensing risks to resolve before 1.0

1. Legal decision on Dodo-style GPUI stacks if GPUI is ever reconsidered[40].
2. Navop supplementary licence must not taint architecture work[103].
3. External-process-only policy for GPL/non-permissive CLI tools; keep `cargo-deny` green[459].

Full file-level evidence: `research/repos/openidea.md`, `research/repos/dodo.md`, `research/repos/navop.md`.

## Sources

[1] https://github.com/l0ng-ai/openidea
[3] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/package.json
[8] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/src-tauri/tauri.conf.json
[37] https://github.com/MrGru/dodo
[40] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//THIRD-PARTY-NOTICES.md
[41] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//Cargo.toml
[43] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//docs/architecture/workspace-layout.md
[98] https://github.com/Alno0/navop
[103] https://github.com/Alno0/navop/blob/78dba5c/NAVOP_LICENSE
[302] https://crates.io/api/v1/crates/iced
[345] https://docs.wasmtime.dev/api/wasmtime/component
[361] https://github.com/microsoft/language-server-protocol/blob/main/indexFormat/specification.md
[364] https://microsoft.github.io/debug-adapter-protocol
[459] https://embarkstudios.github.io/cargo-deny/checks/licenses
[485] https://learn.microsoft.com/en-us/windows/package-manager/winget
[496] https://docs.brew.sh/Brew-Bundle-and-Brewfile
[609] https://criterion-rs.github.io/book/user_guide/analysis.html
[612] https://github.com/sharkdp/hyperfine
