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

`cargo deny check`: PASS (advisories ok, bans ok, licenses ok, sources ok; exit 0) after adding BSL-1.0 for `clipboard-win` and exact-pinning direct dependencies. Hosted CI run [45012466036](https://github.com/hungpt99-dev/Devdeck/actions/runs/45012466036) on commit `4e31d13` is GREEN: quality (fmt/clippy/tests/i18n/deny), dependency audit, and release builds on ubuntu-latest, macos-latest and windows-latest all passed. The earlier run 45012219795 had only the audit job fail because `rustsec/audit-check@v2` treated the six advisory warnings as fatal and lacked check-run permission; CI now runs `cargo audit` directly so warnings stay visible without hiding advisories.

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

Hosted CI is green. Next: start Phase 2: terminal (portable-pty + alacritty_terminal), task runner and extension-host MVP loading a WASM command/tool with permission enforcement and crash isolation.


## Research & architecture initiative (2026-11-22)

Ten independent research agents investigated OpenIDEA, Dodo, Navop, the competitive set, native UI frameworks (GPUI/Iced/Slint), modular architecture, tool-management orchestration, UX, PRD and performance. Their evidence lives in `research/` (3,527 lines; 635 unique sources consolidated in `research/.citations/master.json`; all 10 reports pass `sources.py verify`). The CEO synthesis and the 8 requested deliverables are in `blueprint/`, starting with `blueprint/00-ceo-recommendation.md`.

Status: research and architecture are COMPLETE and awaiting review. No product code was changed; the implementation tree remains at `7929181`. Implementation must not start until the blueprint is reviewed and approved.

## Phase 2 Foundation execution plan (IND-210)

Approved by the founder via approval 519ded45 (2026-11-22), parent IND-207. Sequencing authority: `blueprint/07-implementation-roadmap.md` row "2 Foundation" and `blueprint/08-mvp-definition.md` Gate A. Strategy: `blueprint/00-ceo-recommendation.md` — local-first, one boundary, reuse engines through adapters. Design input: VISION.md non-negotiables 3, 8, 9, 10; blueprint/02 and 05; ADRs 0001–0004; `docs/security/threat-model.md`; `docs/design/tokens.md`. Tech Lead owns the plan; QA is the reviewer on every child. Children are depth 1 under IND-210; there are no grandchildren.

| Child | Deliverable | Owner | Reviewer | Depends on | Exit evidence |
| --- | --- | --- | --- | --- | --- |
| IND-211 | Config v2 migration, fsync+atomic writes, backups, bounded project recents, `projects.open`/`projects.recent` tools | Developer | QA | — | 100-point crash-write test, v0→v1→v2 fixture preservation, 3-OS CI |
| IND-212 | Six-region native Iced shell (activity rail, tabbed center, optional context bar, bottom panel, status bar) | Art Director | QA | — | Xvfb smoke + dark/light/high-contrast/vi screenshots |
| IND-213 | Real PTY engine: portable-pty + vte grid, workspace cwd, bounded scrollback, secret redaction, permission decision | Developer | QA | IND-211 | Session/grid/redaction/policy tests, 3-OS build links |
| IND-214 | Terminal tabs/panes in the bottom panel, allow/deny prompt wiring, keyboard-first focus | Art Director | QA | IND-212, IND-213 | Pane/tab tests + dark/vi/approval screenshots |
| IND-215 | Task runner: named shell commands from workspace config, executed through IND-213 terminal tools | Developer | QA | IND-211, IND-213 | Parse/cwd/status/approval/audit tests |
| IND-216 | Read-only tool detection backend (`tools.detect`, `tools.list_detected`), no side effects | Developer | QA | IND-211 | Fake-PATH fixtures, read-only HOME test |
| IND-217 | Read-only Tool Center panel: detected/missing tool table, no install controls | Art Director | QA | IND-212, IND-216 | Filter/status tests + dark/vi screenshots |

Critical path: IND-211 → IND-213 → IND-214 → J1 integration gate. Parallel branches: IND-212 feeds IND-214; IND-211 → IND-216 → IND-217; IND-215 joins after IND-211 and IND-213. The Phase 2 exit gate opens only when IND-214, IND-215 and IND-217 have QA verdicts attached.

Exit criteria (all must have recorded evidence before Phase 3 starts):
- [ ] J1 — open project → edit file → run command → review diff → commit — completes fully offline on Linux, macOS and Windows. Editor and Git essentials land in Phase 3; Phase 2 proves the project/terminal/task portion and leaves the diff/commit hand-off wired to the Phase 1 typed tool boundary.
- [ ] Performance measured, not asserted: cold start p95 < 1 s, idle core RSS < 150 MB, keystroke-to-screen p95 < 16 ms, 60+ fps, using a reproducible harness recorded in `docs/performance.md`.
- [ ] Accessibility gate: full keyboard-only operation, visible focus, localized labels and high-contrast theme pass. Screen-reader tree remains an accepted ADR 0001 gap, not a Phase 2 gate.
- [ ] Crash/reliability: 100 crash sessions around config writes and terminal shutdown produce zero lost edits or truncated config; atomic-write tests from IND-211 are green.
- [ ] Quality gates: `cargo test --workspace`, fmt, clippy `-D warnings`, i18n lint, `cargo deny`, `cargo audit` and the hosted 3-OS CI are green.
- [ ] Security-sensitive children IND-211, IND-213 and IND-215 carry the founder human review note (Security Lead not yet approved); no child is marked done without QA's verdict.

Explicitly out of scope for Phase 2 Foundation: WASM extension host, LSP/DAP, mise consent-gated install slice (Phase 4 Gate C), editor/Tree-sitter, Git essentials (Phase 3), Docker/DB/API client, agents/MCP, extension registry and updater (Phase 5+). No design-review gate is invented: Product/UX Lead approval is pending, so UI children carry their own spec/mockup notes and screenshot requirements only.

Budget discipline: Tech Lead 2500 cents (0 spent), Developer 3000 (200 spent, 4 children), Art Director 1500 (0 spent, 3 children), QA 2000 (0 spent, reviewer only). All assigned agents are idle and under budget; no new agents were created (four hire approvals remain pending with the founder).

## IND-212 — six-region native shell (verified 2026-11-22)

Implemented in `crates/ui/src/shell.rs` and wired into `app/src/main.rs`. The shell renders header, 48 px activity rail, 200 px primary sidebar, tabbed center, optional 240 px context bar, collapsible 240 px bottom panel with Terminal/Problems/Output/Logs tabs, and the 28 px status bar. All colors come from `Tokens`; every label is resolved through `devdeck_i18n` (46 en/vi keys, lint passing). No webview and no new dependencies.

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | PASS (exit 0) |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (exit 0) |
| `cargo test --workspace` | PASS — 33 tests, 0 failed (12 new shell geometry/state tests; UI crate 14 total) |
| `cargo run -p devdeck-xtask -- i18n-lint` | PASS — en/vi parity OK (46 keys) |
| `cargo deny check` | PASS — advisories ok, bans ok, licenses ok, sources ok (exit 0) |
| Xvfb smoke | Window `Devdeck` mapped at 1200x800 (origin 0,0), screenshots captured with `import -window` |
| Keyboard/mouse interaction | Ctrl+K, Escape, Ctrl+J, Ctrl+`, Ctrl+Shift+B, F6/Shift+F6 and splitter mouse drag all produced the expected pixel diffs |

Real screenshots: `docs/screenshots/phase2-shell-dark.png`, `phase2-shell-light.png`, `phase2-shell-contrast.png`, `phase2-shell-vi.png`. Screenshot inspection note with measured region boundaries, theme diffs and interaction evidence: `docs/screenshots/phase2-shell-inspection.md`.

Also fixed at boot: the persisted locale from `config.json` is now applied to the translator (previously the host locale won), so the vi screenshot genuinely shows Vietnamese and startup matches runtime switching.

Hosted CI could not be triggered from this run: `gh auth status` reports no logged-in GitHub host. The five local gates above are the same checks the quality job runs; hosted 3-OS CI is still required before the child is closed. No design-review gate was invented for this child; QA remains the reviewer.

## IND-213 terminal engine (2026-11-22)

Implemented in `crates/terminal` on a dedicated branch from `main`: real PTY sessions (`portable-pty 0.9.0`), vte 0.15.0 grid with `rows + scrollback_lines` cap, bounded/coalesced tokio events, one shared redactor in `devdeck-audit`, and typed `terminal.spawn/write/resize/close` tools. `terminal.spawn` calls `devdeck_core::PermissionPolicy::decide`; dangerous commands return a decision and never auto-execute. Tests cover grid parsing/cap, redaction, policy decisions, localized missing-cwd errors, PTY lifecycle, idempotent resize/close and zero planted tokens in audit. en/vi bundles updated; no UI, task runner or approval dialog. Status: in review — founder human review (Security Lead not yet approved) and QA verdict still required; not done.

