# Devdeck progress — verified status

Last updated: 2026-11-22. Status-first summary: **Phase 0 and the Phase 1 slice are implemented and locally verified.** The app builds, tests pass, fmt/clippy are clean, and the native window opens under Xvfb. Most tool crates are intentional skeletons; MCP, extension host, terminal, editor and all P0 tools are NOT implemented.

## What actually ran (exit codes)

| Command | Result |
| --- | --- |
| `cargo test --workspace` | PASS — 21 tests, 0 failed (3 tools, 2 audit, 5 core, 2 extension-api, 6 i18n, 2 ui, 1 xtask) |
| `cargo fmt --all -- --check` | PASS (exit 0) |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (exit 0) |
| `cargo run -p devdeck-xtask -- i18n-lint` | PASS — en/vi parity OK (25 keys) |
| `cargo build -p devdeck-app --bin devdeck-app` | PASS — binary at `target/debug/devdeck-app` |
| `cargo build --workspace --release` | PASS — `target/release/devdeck-app` is 15 MB (stripped, thin LTO) |
| `cargo audit` | PASS, exit 0 — 6 advisory warnings (5 unmaintained transitive crates: paste, ttf-parser, zbus_names, zvariant, zvariant_derive; 1 unsound: `lru 0.16.4` via `cryoglyph`/`iced_wgpu`, no compatible fixed version yet) |
| Xvfb smoke of `devdeck-app` | Window `Devdeck` 1200x800 mapped; `~/.devdeck/audit.jsonl` created; only benign libEGL/DRI3 warnings |
| Xvfb screenshot + visual inspection | PASS — header, sidebar (8 items), content, status bar rendered; status reads "Ready · 1 tool loaded" (singular plural verified). Screenshot: `docs/screenshots/phase1-shell.png` |
| `cargo run -p devdeck-cli -- doctor --lang vi` | PASS — locale `vi`, 1 tool, telemetry off |
| `cargo run -p devdeck-cli -- --json mcp --list-tools` | PASS — JSON with typed `files.read` schema (MCP server itself is Phase 3.5) |
| Iced spike `cargo test` | PASS 2/2; Xvfb window mapped |
| GPUI spike `cargo build --example hello_world` | FAIL — `xattr v0.2.3` vs libc `ENOATTR` on linux-gnu; full spike not completed |

`cargo deny check`: PASS (advisories ok, bans ok, licenses ok, sources ok; exit 0) after adding BSL-1.0 for `clipboard-win` and exact-pinning direct dependencies. The 3-OS build runs only in GitHub Actions and has NOT been verified locally.

## What works

- Cargo workspace: 28 skeleton crates + app/cli/xtask, `#![forbid(unsafe_code)]` everywhere.
- Native window shell: header, sidebar, tab/content area, status bar; dark/light/high-contrast tokens; Ctrl/Cmd+K palette with arrow/Enter/Escape; language and theme buttons.
- i18n: Fluent en/vi bundles, typed keys generated at build time, runtime switch + persistence, ICU4X locale fallback, plural/variable test, diacritic-insensitive search.
- Typed tools: `files.read` with generated JSON Schema, permission/risk levels, palette/CLI/MCP metadata; registry exported as MCP `tools/list`.
- Permissions + audit: deny-by-default policy with dangerous-operation escalation; SHA-256 hash-chained append-only JSONL with secret redaction and tamper test.
- Extension API: manifest parsing (semver, contributions, permissions, permission-diff), contribution-point types, draft WIT world.

## Partial / missing

- AccessKit/screen-reader support: absent upstream in Iced 0.14 (ADR 0001). Keyboard navigation works; a11y tree is a gap.
- ICU4X currently used for locale canonicalization only; date/number/collation formatting comes later.
- Command palette is a modal panel, not a true overlay; keybinding presets and resizable splits are Phase 1+ work.
- No terminal, editor, Git, API, DB, Docker, toolchain, agent, extension host, registry, MCP server or updater yet.
- GPUI spike incomplete due to upstream dependency break; decision documented in ADR 0001.
- Performance numbers not measured; docs/performance.md is intentionally empty of claims.

## Risks

- `wgpu` on headless/software EGL emits DRI3 warnings; window still maps. Real Windows/macOS behavior unverified until CI runs.
- Iced 0.14 exact pin: upstream API churn is slower than GPUI but still pre-1.0-adjacent.
- Debug binaries are ~421 MB unstripped; release profile enables strip + thin LTO; size target not measured.

## Next action

Verify the 3-OS CI build on GitHub, commit and push Phase 0/1, then start Phase 2: terminal (portable-pty + alacritty_terminal), task runner and extension-host MVP loading a WASM command/tool with permission enforcement and crash isolation.
