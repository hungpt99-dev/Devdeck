# Devdeck Product Requirements Document (PRD)

**Status:** Draft for research review — 2026-11-22
**Scope:** Research only. This document specifies what a product *would* require; it authorizes no implementation.
**Project inputs (local, not externally cited):** `VISION.md`, `docs/progress.md`, `docs/adr/0001`–`0004`, `docs/security/threat-model.md`.
**Reading convention:** externally sourced facts carry inline numbered citations `[n]` resolved in **Sources** at the end. Product choices, priorities, scope decisions and numeric targets are labelled **DESIGN**; they are recommendations informed by the evidence, not facts reported by it. Requirement IDs are stable; priority is `P0` (revised MVP), `P1` (next wave), `P2` (later / 1.0+).

**Requirements index:** 66 functional requirements (`FR-`, 10 families A–I plus cross-cutting X) and 30 non-functional requirements (`NFR-`), 96 total. Priority split (functional): 15 P0, 36 P1, 15 P2; non-functional priorities are marked per item.

---

## 1. Purpose

Devdeck is proposed as one native Rust desktop application that consolidates the tools a developer opens every day: editor, terminal, Git, containers, databases, API client, toolchain management, configuration and workspace automation. The vision also makes Devdeck a platform with safe extensions and first-class AI agents, but this PRD covers only the explicit requirement families A–I supplied for this exercise; agent-hub and extension-registry details remain referenced from `VISION.md` and are out of scope here (DESIGN).

This PRD answers four questions: who the product is for, what it must do, what the smallest credible first release is, and what evidence would make us stop.

## 2. Evidence base and competitive context

### 2.1 What the outside world shows

- VS Code remains the dominant development environment at 75.9% of all survey respondents, with IntelliJ IDEA at 27.1%, Cursor at 17.9% and Zed at 7.3%; VS Code was also the most desired environment[18]. Any replacement must therefore beat a deeply entrenched incumbent rather than an empty field (DESIGN).
- Container tooling is likewise entrenched: Docker was used by 71.1% of respondents[18], and its usage rose about 17 percentage points to 71% between 2024 and 2025[19]. PostgreSQL was the most wanted database technology for the next year[19]. These two facts drive the priority of requirements D and E (DESIGN).
- Fragmented work is a real cost: developers in one field study switched tasks about 13 times an hour and spent roughly 6 minutes per task, with individual activities often lasting 20 seconds to 2 minutes[26]; knowledge workers deliberately switch applications because tools do not carry context across a task, and describe a merged application as a way to preserve rhythm[27]. This is the user-value premise for an all-in-one deck (DESIGN).
- VS Code and Visual Studio held the top two environment spots for a fourth year, according to the survey home page[20].
- Language intelligence is protocol-standardized: LSP defines JSON-RPC communication between an editor and a reusable language server for completion, go-to-definition and references[21], and VS Code's own guide describes the language-client architecture[1].
- Debugging is similarly decoupled through the Debug Adapter Protocol, with an adapter translating an existing debugger to the protocol[22][2]. A native host can integrate these without reimplementing every language runtime (DESIGN).
- API clients have set a broad expectation: Postman advertises HTTP, GraphQL, gRPC, WebSocket and MQTT in one interface with collections, environments and variables[12][11].
- OpenAPI is the widely used formal standard for describing HTTP APIs[23], and gRPC defines services through `.proto` files and protocol buffers[24]. Matching that breadth is a large scope commitment (DESIGN).
- Database tools show the same breadth problem: TablePlus lists PostgreSQL, MySQL, SQLite, MongoDB and Redis among its supported databases[13][14], while DBeaver advertises 350+ drivers in its community edition[15]. A five-database scope is a credible subset, not a differentiator on breadth alone (DESIGN).
- Native does not automatically mean cheap to build: Zed wrote its own Rust GPU UI framework (GPUI) specifically to hit high frame rates[3][4].
- The IntelliJ Platform is a component-driven, JVM-based host with a mature PSI and plugin marketplace[7][8][9]. Devdeck's all-native constraint carries a real engineering tax that this PRD prices into the MVP (DESIGN).
- Toolchain managers such as mise already solve per-project tool versions through `mise.toml`, lockfiles and registry backends[16]; asdf-style plugins are shell scripts that run with the user's permissions, so installer supply-chain trust is a known problem[17]. This argues for read-only detection before any installation feature (DESIGN).
- Container-desktop overhead is user-visible: a widely discussed macOS issue reports memory far exceeding configured limits[5], and analysis distinguishes Linux Engine overhead from the extra VM and filesystem boundary of Docker Desktop on macOS and Windows[6]. Docker Compose remains the standard way to define multi-container applications[25]. Devdeck should not embed a daemon in the first release (DESIGN).
- Two enabling technologies de-risk later phases: Tree-sitter provides incremental parsing fast enough for every keystroke and tolerates syntax errors[29], and AccessKit offers a cross-platform accessibility abstraction for toolkits that render their own widgets[28]. Local ADR 0001 records that Iced 0.14 lacks upstream AccessKit support, so accessibility is a gap to close rather than a property to assume.

### 2.2 What the evidence does not establish

No source was found that measures demand for an all-in-one native Rust developer desktop app, that validates the VISION.md performance budgets on real hardware, or that shows users will replace an incumbent editor with Devdeck. Those claims are assumptions to test, not facts (DESIGN). `docs/progress.md` states the GPU/RAM targets are not yet measured.

## 3. Personas

All five personas are DESIGN syntheses of the requirements and the evidence above, not researched user interviews.

| ID | Persona | Context and goals | Primary requirement families |
|---|---|---|---|
| PER-1 | Polyglot application developer | Works across several languages and repositories in a day; wants one keyboard-driven workspace, fast feedback and no context loss when switching projects | X, A, B, C, G, H |
| PER-2 | Backend and data developer | Builds services backed by PostgreSQL/MySQL/SQLite and caches; runs queries while debugging code and inspecting API responses; needs safe destructive-action guards | E, F, D, A |
| PER-3 | Platform and DevOps engineer | Owns local Compose stacks, tool versions and repeatable environments; cares about reproducibility and resource cost | D, G, I, B, H |
| PER-4 | Tech lead / onboarding owner | Onboards teammates and wants a project to open with the right tools, versions, settings and tasks without a wiki | I, H, G, C |
| PER-5 | Localized and assistive-technology developer | Works in Vietnamese or another non-English locale, or relies on screen reader, keyboard-only navigation, high contrast or reduced motion; cannot accept a UI that ignores i18n or a11y | X, all families as a constraint |

## 4. Jobs to be done

| ID | Job statement | Maps to |
|---|---|---|
| JTBD-01 | When I open a repository, I want the editor, terminal and Git state to come up together in the right project context, so I can start work without reconfiguring anything | X, A, B, C |
| JTBD-02 | When I edit code, I want completions, diagnostics and go-to-definition without leaving the app, so I can keep my train of thought | A |
| JTBD-03 | When something fails at runtime, I want to set a breakpoint and inspect variables without switching to another IDE | A |
| JTBD-04 | When I need a shell command, I want a real terminal in the project directory with the project's environment, so I do not lose shell context | B |
| JTBD-05 | When I finish a change, I want to review the diff and commit from the same window, so version control stays in the flow | C |
| JTBD-06 | When a bug depends on a service, I want to start or inspect its Compose stack and read logs in place | D |
| JTBD-07 | When data looks wrong, I want to inspect the schema and run a bounded query without opening a separate database client | E |
| JTBD-08 | When an endpoint misbehaves, I want to send a request with the right environment and inspect the response next to my code | F |
| JTBD-09 | When a project requires a specific tool version, I want the app to detect what is available and let me pin versions explicitly, so my environment is reproducible | G, I |
| JTBD-10 | When settings or config drift, I want one editor with validation and a versionable diff, so mistakes surface before they reach a teammate | H |
| JTBD-11 | When I move between two projects, I want a named profile to restore tools, env, tasks and layout, so switching costs seconds rather than an afternoon | I |
| JTBD-12 | When I onboard a teammate, I want to hand them a workspace snapshot that restores our setup, so the first day is productive | I, H, G |

## 5. User journeys

**J1 — Cold start to first commit (PER-1).** Launch Devdeck → open a repository → editor and terminal open in that directory → edit a file → `git status` view shows the change → stage, write message, commit → success shown in log. Failure points: missing project context in the terminal, unreadable diff, accidental destructive Git action. Touches FR-X01, FR-A01, FR-B01, FR-C01, FR-C02.

**J2 — Debugging a failing API path (PER-2).** Read code → run a test in the terminal → inspect a query in the database panel → send the same request in the API client → inspect container logs → adjust code and repeat. Failure points: interleaved panels losing state, secrets leaking into request history, unbounded query on production data. Touches FR-A01, FR-B01, FR-E03, FR-F01, FR-F02, FR-D02, FR-E06.

**J3 — Switching projects (PER-1, PER-3).** Close project A → open project B → profile B applies its tool versions, env and task list; A's layout and unsaved state are preserved → no PATH changes leak between projects. Failure points: global PATH mutation, profile conflict, loss of unsaved edits. Touches FR-X01, FR-B03, FR-G04, FR-I01, FR-I02, FR-X06.

**J4 — Onboarding from a snapshot (PER-4).** Receive a local snapshot → preview manifest and permissions → restore into a new workspace → verify tools, settings and tasks → run the project's task. Failure points: unsigned or tampered archive, OS-specific absolute paths, hidden secrets. Touches FR-I03, FR-I04, FR-I05, FR-H05, FR-X04.

**J5 — Localized, keyboard-only workflow (PER-5).** Switch UI language at runtime → open the command palette → complete J1 using only the keyboard with a screen reader → confirm labels, contrast and focus order. Failure points: untranslated strings, missing accessibility tree, focus traps. Touches FR-X02, FR-X03, NFR-I18N-01/02, NFR-A11Y-01/02/03.

## 6. Functional requirements

ID scheme: `FR-<family>-<nn>`. Every requirement is a DESIGN decision. Acceptance criteria are written to be testable by unit test, integration test, CLI command, or scripted UI/manual check; "test" below means an automated check in CI unless noted.

### 6.1 Cross-cutting workspace platform (X)

| ID | Pri | Requirement | Acceptance criteria |
|---|---|---|---|
| FR-X01 | P0 | Workspace model: opening a folder creates an isolated workspace with recents, layout and per-workspace state under the app-owned directory | Test: open three folders, change state in each, restart; each workspace restores its own state with no cross-contamination |
| FR-X02 | P0 | Every capability registers as a typed tool with permission level, risk level, palette entry, CLI subcommand and localized display text | Test: enumerate registry; a capability missing any required metadata fails the registry test |
| FR-X03 | P0 | Ctrl/Cmd+K command palette with fuzzy search over tools, workspaces and recent actions | Test: a keyboard-only script opens the palette, finds and runs a named tool within three keystroke groups |
| FR-X04 | P0 | Deny-by-default permissions, confirmation for dangerous operations, and an append-only SHA-256 hash-chained audit log with secret redaction | Test: unapproved call is denied; tampering with a log line is detected; a secret-shaped token never appears in the log |
| FR-X05 | P1 | Contribution points let built-in modules and extensions add tools identically, with no private host back door | Test: a sample extension adds a palette command and is discoverable through the same registry API |
| FR-X06 | P1 | Autosave, versioned config migration and crash recovery | Test: kill -9 during an edit and during a config write; restart restores last committed content and a valid config version within 5 s |

### 6.2 A — Editor, LSP and debug

| ID | Pri | Requirement | Acceptance criteria |
|---|---|---|---|
| FR-A01 | P0 | Minimal editor: tabs, open/save/close, UTF-8 with BOM handling, LF/CRLF preservation, virtualized rendering of large files | Test: open a 10 MB file, edit at line 100,000, save; bytes outside the edited region are unchanged and rendering stays bounded |
| FR-A02 | P0 | Incremental syntax highlighting with Tree-sitter grammars, with graceful output on syntax errors | Test: a keystroke on a 10,000-line file reparses only the changed region; malformed file still renders tokens; cites [29] |
| FR-A03 | P0 | In-file find/replace and go-to-line | Test: case-sensitive and insensitive search, replace-all count, and jump to line 10,000 all match expected offsets |
| FR-A04 | P1 | LSP client over JSON-RPC with a sidecar process, supporting completion, go-to-definition, references and diagnostics | Test: with rust-analyzer and pyright, each of the four operations returns expected results in a fixture workspace; cites [21][1] |
| FR-A05 | P1 | Per-workspace LSP server discovery and explicit server configuration | Test: workspace A uses rust-analyzer and B uses pyright; no server from A is attached to B |
| FR-A06 | P1 | DAP client via debug adapters: breakpoints, step over/into/out, stack frames, variables, REPL | Test: one runtime debug session completes a breakpoint → step → inspect → continue sequence; cites [22][2][10] |
| FR-A07 | P1 | Debug launch configurations stored with the workspace and editable as structured config | Test: launch config round-trips through the config editor and reproduces the same debug session |
| FR-A08 | P2 | Code actions, refactorings and inlay hints from LSP | Test: rename symbol updates all references in a fixture project |
| FR-A09 | P2 | Split editor, multi-cursor and minimap | Test: two splits show independent cursors and scroll positions |

### 6.3 B — Terminal

| ID | Pri | Requirement | Acceptance criteria |
|---|---|---|---|
| FR-B01 | P0 | Real pseudo-terminal with spawn of the user's shell, UTF-8 output, IME input and resize propagation | Test: on each target OS, spawn the default shell, run a Unicode-heavy command, resize the pane; output and cursor positions are correct |
| FR-B02 | P0 | Multiple named terminals per workspace with persisted working directory | Test: open three terminals in different directories, restart, restore all three with correct cwd |
| FR-B03 | P0 | Workspace environment injection with explicit consent; secret values never rendered in terminal text, history or audit | Test: an injected token does not appear in UI, scrollback, log files or audit entries |
| FR-B04 | P1 | Shell integration: prompt/cwd tracking, command exit status, clickable file paths | Test: `cd /tmp` updates the pane's cwd and a path in output opens the file in the editor |
| FR-B05 | P1 | Task runner executing declared tasks in the terminal panel with exit status | Test: a failing task surfaces a nonzero status and a localized error without blocking the UI thread |

### 6.4 C — Git

| ID | Pri | Requirement | Acceptance criteria |
|---|---|---|---|
| FR-C01 | P0 | Repository detection, status and staged/unstaged diff | Test: a fixture repo with modified, staged, untracked and deleted files matches `git status --porcelain` and `git diff` output |
| FR-C02 | P0 | Stage, unstage, commit with message, and explicit amend | Test: a commit appears in `git log` with the exact message; amend is refused without confirmation |
| FR-C03 | P0 | Branch listing, creation and checkout with dirty-worktree warning | Test: checkout on a dirty worktree warns and requires confirmation; clean checkout succeeds |
| FR-C04 | P1 | Commit log graph, file history and blame | Test: log order and blame attribution match the CLI on a fixture repo |
| FR-C05 | P1 | Fetch, pull and push using OS-keychain credentials | Test: no credential material is written to config, logs or audit; an expired credential produces a localized recoverable error |
| FR-C06 | P2 | Merge-conflict resolution and interactive rebase UI | Test: a simple conflict resolves to the expected file content and a clean status |

### 6.5 D — Docker and Compose

| ID | Pri | Requirement | Acceptance criteria |
|---|---|---|---|
| FR-D01 | P1 | Discover a local or explicitly configured remote Docker daemon and report reachability | Test: with daemon stopped, the panel shows a localized disconnected state and no blocking I/O on the UI thread |
| FR-D02 | P1 | List containers and start, stop and restart them; stream logs | Test: lifecycle transitions match `docker inspect` state; log stream ends cleanly on stop |
| FR-D03 | P1 | List images and prune unused images behind an explicit confirmation | Test: dry-run count matches the CLI; without confirmation nothing is deleted |
| FR-D04 | P1 | Compose project up, down and restart with per-service status | Test: a two-service Compose file reaches healthy state and `down` removes both containers; cites [25] |
| FR-D05 | P2 | Volume and network inspection with guarded deletion | Test: a volume in use cannot be deleted without force confirmation |
| FR-D06 | P1 | Resource guardrails: warn when daemon memory or disk exceeds a configurable threshold; no embedded daemon in this release | Test: a simulated high-memory daemon triggers the warning; no local daemon process is started by Devdeck; rationale [5][6] |

### 6.6 E — Databases

| ID | Pri | Requirement | Acceptance criteria |
|---|---|---|---|
| FR-E01 | P1 | Connection profiles with passwords/URLs stored only in the OS keychain | Test: connection strings are absent from config files, logs and audit; a profile survives restart |
| FR-E02 | P1 | Schema browsing and bounded read-only queries for PostgreSQL, MySQL and SQLite | Test: against fixture databases, table/column lists and a `SELECT` result set match the CLI; writes are refused in read-only mode |
| FR-E03 | P1 | Query editor with enforced LIMIT/timeout, result grid and CSV export | Test: a query without LIMIT is bounded; a timeout produces a localized error; exported CSV has expected rows and escaping |
| FR-E04 | P1 | MongoDB database/collection browsing and read-only document queries | Test: a filter query returns the expected documents and no write operation is exposed in MVP |
| FR-E05 | P2 | Redis key browse and value inspection | Test: key types are rendered correctly and TTL is shown |
| FR-E06 | P1 | Destructive statements require typed confirmation and are audited | Test: `DROP`/`DELETE`/`TRUNCATE` without confirmation is blocked; with confirmation an audit entry is written |
| FR-E07 | P2 | Saved query snippets and query history with redaction | Test: a snippet containing a literal secret is redacted on save and in history |

### 6.7 F — API client

| ID | Pri | Requirement | Acceptance criteria |
|---|---|---|---|
| FR-F01 | P1 | REST request builder: method, URL, headers, query params and body types | Test: a fixture server receives the exact method, headers and body configured |
| FR-F02 | P1 | Environments and variables with keychain-backed secrets | Test: switching environments rewrites only the intended variables; secret values never appear in history or logs |
| FR-F03 | P1 | OpenAPI import and request generation from an operation | Test: importing a fixture OpenAPI 3 document produces one request per operation with correct paths; cites [23] |
| FR-F04 | P2 | GraphQL schema introspection and visual query builder | Test: introspection populates fields and a built query returns expected data from a fixture server |
| FR-F05 | P2 | WebSocket session with send/receive and message filtering | Test: a bidirectional echo session displays both directions and can be closed cleanly |
| FR-F06 | P2 | gRPC calls from `.proto` definitions or reflection, supporting unary and streaming | Test: a unary call and a server-streaming call each return expected messages; cites [24] |
| FR-F07 | P1 | Response viewer with status, timing, headers, formatted body and search | Test: a fixture response shows correct status, duration bucket and parsed JSON tree |
| FR-F08 | P2 | Import from cURL, HAR and Postman collections with size limits and no script execution | Test: a malicious fixture containing a script is imported as inert text; fuzz suite passes; threat model requires this |

### 6.8 G — Developer tool management

| ID | Pri | Requirement | Acceptance criteria |
|---|---|---|---|
| FR-G01 | P0 | Read-only detection of installed tools and versions from PATH and the app-owned directory | Test: detection makes no network calls, writes nothing and does not modify PATH |
| FR-G02 | P1 | Manifest-backed tool catalog with checksum and signature verification | Test: a manifest with a bad checksum or signature is rejected and reported; rationale [16][17] |
| FR-G03 | P1 | Install and uninstall into the app-owned directory; no global PATH changes without explicit consent | Test: after install, `command -v` is unchanged unless consent was granted; uninstall removes all files under the app directory |
| FR-G04 | P1 | Per-workspace tool version pinning with a lockfile | Test: two workspaces pin different versions of the same tool and each resolves its own binary |
| FR-G05 | P2 | Opt-in, reversible shell activation integration | Test: enabling then disabling activation restores the original shell startup file byte-for-byte |
| FR-G06 | P1 | Permission diff shown and approved before a tool or extension update | Test: an update that adds a permission is blocked until the diff is accepted |

### 6.9 H — Configuration studio

| ID | Pri | Requirement | Acceptance criteria |
|---|---|---|---|
| FR-H01 | P0 | YAML/JSON editor with schema validation, formatting and line/column errors | Test: invalid YAML is flagged with correct line and column; valid JSON round-trips through the formatter |
| FR-H02 | P1 | Form-based configuration studio generated from typed schemas for Devdeck and workspace config | Test: editing a value in the form and in text both produce the same validated config; unknown keys are flagged |
| FR-H03 | P1 | Local schema registry with offline validation cache | Test: with network disabled, a cached schema still validates a document |
| FR-H04 | P1 | Config diff and history using Git-friendly text | Test: two config revisions produce a minimal readable diff and can be restored |
| FR-H05 | P1 | Environment/secrets editor with keychain backing and redacted display | Test: a secret is stored outside the config file and is masked in every view and export |
| FR-H06 | P2 | Templates for common YAML/JSON config files such as Compose and CI configs | Test: a generated Compose template passes schema validation |

### 6.10 I — Workspace automation, profiles and snapshots

| ID | Pri | Requirement | Acceptance criteria |
|---|---|---|---|
| FR-I01 | P1 | Named workspace profiles capturing tool versions, environment, tasks and layout | Test: two profiles serialize to distinct versioned documents with no secrets in plaintext |
| FR-I02 | P1 | Apply and switch profiles with conflict detection | Test: applying a profile over unsaved edits or conflicting env vars prompts before changing state |
| FR-I03 | P1 | Local snapshot capture and restore of workspace state | Test: a snapshot restores into a fresh workspace with the same tools, settings and task list |
| FR-I04 | P1 | Snapshot portability across OSes with path remapping | Test: a snapshot from Linux restores on Windows/macOS fixtures with absolute paths remapped |
| FR-I05 | P2 | Export/import of versioned, signed profile archives | Test: a tampered archive is rejected; a valid one imports with a permission preview |
| FR-I06 | P2 | Rhai automation scripts gated by permissions and resource limits | Test: an infinite-loop script is terminated at the fuel/time limit without affecting the host |
| FR-I07 | P2 | Triggered automations such as on-commit and on-container-start | Test: a trigger fires exactly once for one fixture event and is audited |

## 7. Non-functional requirements

All NFR targets are DESIGN; each is written with a verification method. Local `VISION.md` and `docs/progress.md` define the same budgets as unmeasured aspiration, which is why they are restated here as testable gates.

**Security (SEC)**

- NFR-SEC-01 (P0): Deny by default for every capability; unapproved access is impossible by construction. Verify: policy unit tests across all typed tools.
- NFR-SEC-02 (P0): Destructive operations (force push, DROP/DELETE, container prune, file deletion) require explicit confirmation. Verify: scripted negative tests for each family.
- NFR-SEC-03 (P0): Append-only SHA-256 hash-chained audit with redaction before write. Verify: tamper and redaction tests; local threat model marks this implemented in Phase 0/1.
- NFR-SEC-04 (P1): Secrets stored only in OS keychain; never in config, logs, history or audit. Verify: static scan plus runtime fixture with canary tokens.
- NFR-SEC-05 (P1): WASM extensions isolated with fuel/memory/time limits; native sidecars are not offered for untrusted code until an OS sandbox is designed. Verify: trap and resource-exhaustion tests; local threat model marks sidecar sandboxing as an open gap.

**Privacy (PRIV)**

- NFR-PRIV-01 (P0): Telemetry off by default and opt-in only. Verify: fresh install writes no telemetry flag and opens no telemetry socket.
- NFR-PRIV-02 (P0): Core workflows require no account, cloud login or network service. Verify: offline clean-room test from a fresh profile.
- NFR-PRIV-03 (P0): No user data leaves the device without an explicit user action. Verify: socket audit during all P0 journeys shows no unexpected egress.

**Offline (OFF)**

- NFR-OFF-01 (P0): Editor, terminal, Git, config editing and settings work fully offline. Verify: network namespace disabled during the automated journey suite.
- NFR-OFF-02 (P1): Schemas, grammars and tool metadata required for validation are cached locally. Verify: offline validation test with network disabled.
- NFR-OFF-03 (P1): Network-dependent features degrade with a localized explanation and never block offline ones. Verify: injected DNS failure leaves all P0 journeys passing.

**Internationalization (I18N)**

- NFR-I18N-01 (P0): All user-facing strings, errors and notifications go through the i18n layer; no hard-coded UI text. Verify: string-lint plus key parity check; local ADR 0003 requires en/vi parity.
- NFR-I18N-02 (P0): Runtime language switch without restart, with en and vi in the first public release. Verify: switch mid-session and assert translated labels and persisted setting.
- NFR-I18N-03 (P1): Diacritic-insensitive search with NFD normalization and explicit đ/Đ mapping. Verify: Vietnamese queries match expected entries.
- NFR-I18N-04 (P2): RTL display readiness and explicit bidi isolation around variables; CJK/IME stress coverage. Verify: RTL fixture screenshots reviewed; local ADR 0003 defers full bidi layout.

**Accessibility (A11Y)**

- NFR-A11Y-01 (P0): Every P0 journey completable with keyboard only, with visible focus order. Verify: automated keyboard-only traversal of J1, J3 and J5.
- NFR-A11Y-02 (P1): An accessibility tree exposes all interactive controls to screen readers; AccessKit is the chosen abstraction[28]. Verify: a11y-tree snapshot covers 100% of controls in tested panels. Local ADR 0001 records that upstream Iced 0.14 has no AccessKit integration, so this needs an upstream contribution or fork.
- NFR-A11Y-03 (P0): Dark, light and high-contrast themes meet WCAG AA contrast for text. Verify: automated contrast check across token pairs.
- NFR-A11Y-04 (P0): Reduced-motion preference honored. Verify: animation-duration assertions with the OS flag enabled.

**Cross-platform (CROSS)**

- NFR-CROSS-01 (P0): Windows 10/11, macOS current plus previous two majors, Linux X11 and Wayland. Verify: CI build and smoke test on all three OS families; local progress shows a green hosted CI across ubuntu, macOS and Windows.
- NFR-CROSS-02 (P0): Software-rendering fallback when GPU is unavailable. Verify: headless/software renderer smoke test maps a window.
- NFR-CROSS-03 (P0): OS-specific code isolated in modules; core never depends on UI and no blocking I/O on the UI thread. Verify: dependency-crate test plus tokio task audit.
- NFR-CROSS-04 (P1): HiDPI scaling and multi-monitor placement. Verify: synthetic DPI change and dual-monitor fixture keep widgets legible and on-screen.

**Performance (PERF)**

- NFR-PERF-01 (P0): Cold start < 1 s p95 on the reference machine. Verify: 100 launches, report p95.
- NFR-PERF-02 (P0): Idle core RAM < 150 MB. Verify: 10-minute idle sample with RSS sampled per second.
- NFR-PERF-03 (P0): Keystroke-to-screen < 16 ms p95 in a 10,000-line file. Verify: instrumented input-to-present benchmark.
- NFR-PERF-04 (P1): 60+ fps during scroll, terminal output burst and panel resize. Verify: frame-time trace in CI benchmark job.

**Reliability and data integrity (REL)**

- NFR-REL-01 (P0): Atomic config writes, schema version and forward migration with backup before destructive migration. Verify: interruption injection at each write boundary; local threat model marks the v0→v1 migration and atomic save implemented.
- NFR-REL-02 (P0): No user data loss across crashes. Verify: 100 kill -9 sessions with canary content; zero lost edits.
- NFR-REL-03 (P0): Full quality gate green on all supported OSes: fmt, clippy, tests, dependency/license check, i18n parity. Verify: CI job output.

## 8. Acceptance criteria and definition of done

A feature is done when (a) its FR acceptance test passes in CI on all three OS families, (b) all strings are localized in en and vi, (c) the security/privacy/offline NFRs relevant to it are verified, (d) the threat model is updated for any new capability, tool or data flow, and (e) performance budgets are re-measured rather than assumed (DESIGN). For the revised MVP, the release gate is the full J1 and J5 journey suites passing offline, with zero open P0 security findings.

## 9. Proposed revised MVP (DESIGN)

**Name:** Devdeck Foundation — one credible, fast, local-first workspace.

### 9.1 In scope (P0)

1. Native shell: window, resizable tab/split layout, command palette, themes, settings, runtime en/vi switch (X, H).
2. Workspace/project model: open folder, recents, isolated per-workspace state and atomic config (X, I).
3. Minimal editor: tabs, large-file virtualization, Tree-sitter highlighting, find/replace, go-to-line (A01–A03). **No LSP, no DAP.**
4. Terminal: real PTY, multiple named panes, workspace env with consent and secret redaction (B01–B03).
5. Git essentials: status, diff, stage/unstage, commit, branch checkout with dirty-worktree warning (C01–C03).
6. YAML/JSON config editor with schema validation and formatting (H01).
7. Read-only tool detection and display; no installation, no registry (G01).
8. Permissions, audit, telemetry-off default, offline operation, keyboard-only J1/J5, accessibility and performance gates (X04, NFRs).

### 9.2 Explicitly out of scope for the revised MVP (P1/P2 or later)

LSP and DAP (A04–A09), task runner and shell integration (B04–B05), history/blame/remote Git (C04–C06), all Docker and Compose features (D), all database connectors (E), all API client features (F), tool installation/registry/version pinning (G02–G06), form-based configuration studio and schema registry beyond single-file validation (H02–H06), profiles/snapshots/automation (I), extensions host, registry, AI agent hub, Kubernetes, SSH and all VISION P1/P2 tools.

### 9.3 Rationale

The MVP buys the JTBD-01, JTBD-04, JTBD-05 and JTBD-10 loop — open, edit, shell, commit, configure — which is the smallest loop that makes an all-in-one deck more useful than three separate apps.
It deliberately avoids four high-dependency clusters: protocol sidecars (LSP/DAP), daemon-backed containers, database wire protocols and installer/registry supply chain.
The LSP protocol is standardized and reusable[21][1], but integration still requires a working extension/sidecar host, server discovery and a production editor widget before it has value; local ADR 0001 flags widgets as unbuilt and the roadmap places editor/LSP in Phase 6.
Container and database features sit behind daemons, credentials and destructive-action guards; the incumbent breadth is high[13][15][25].
Container-desktop resource costs are a known complaint[5][6]. Installation is the only item with an unresolved supply-chain trust model[17] and an explicit human-review gap in the local threat model.
Terminal-first sequencing matches `docs/progress.md`, which names terminal, task runner and extension-host MVP as the next phase.

**DESIGN trade-off:** this MVP will not by itself prove the "replaces everything" thesis. It proves the *foundation* thesis: one fast native workspace where the next five families can be added through the same typed-tool and contribution-point plumbing.

### 9.4 Sequencing and gates (DESIGN)

- Gate 0 — shell, projects, terminal, Git: J1 passes offline on three OSes; PERF and A11Y P0 gates met.
- Gate 1 — editor/config: J1 with editor and validated config; crash-recovery test green.
- Gate 2 — sidecar host: LSP/DAP and tool installation only after permission enforcement, crash isolation and a documented native-sidecar sandbox decision.
- Gate 3 — services: database, API and Docker families, each behind its own destructive-action and secret-handling tests.

## 10. Evaluation of the suggested 10-item MVP

Verdict key: **KEEP** = included in the revised MVP; **DEFER** = still planned, later phase; **CUT** = removed from the near-term product and replaced by a smaller capability. Complexity and value are DESIGN ratings (Low/Medium/High).

| # | Suggested item | Verdict | Complexity | Value | Reasoning |
|---|---|---|---|---|---|
| 1 | Shell | **KEEP** | Low | High | Already implemented and locally verified in Phase 1 (`docs/progress.md`); it is the host for every other family, so retaining it has near-zero incremental dependency risk |
| 2 | Projects | **KEEP** (reduced to open folder, recents, isolated state) | Low | High | Required before any tool can be scoped to a repository; Git, terminal cwd and config all depend on it; the full profile/snapshot model is deferred |
| 3 | Editor + LSP | **DEFER** (split: keep the minimal editor from FR-A01–A03, defer LSP/DAP) | High | High | The bundle hides a large dependency chain: a production editor widget, sidecar host, JSON-RPC protocol client and per-project server management[21][1]; local ADR 0001 flags widgets as unbuilt and the roadmap places editor/LSP in Phase 6. Editing alone delivers most of the visible "app" value; intelligence can land behind Gate 2 |
| 4 | Terminal | **KEEP** | Medium | High | Highest-leverage single tool: it unblocks shell work, Git and later tool/container workflows; portable-pty plus an established terminal emulator is a known quantity, and it is the documented next phase |
| 5 | Git | **KEEP** (status, diff, stage, commit, branch checkout only) | Medium | High | Completes J1 with the terminal; history, remotes and conflict resolution can follow. Credential handling must use the OS keychain from the start (NFR-SEC-04) |
| 6 | Docker | **DEFER** | High | High | Universal adoption makes it important[18], but it needs daemon discovery, Compose semantics and cross-platform resource management; reported desktop memory problems make guardrails a prerequisite[5][6], and the value only materializes once a service-backed debugging journey exists |
| 7 | PostgreSQL + Redis | **DEFER** | High | Medium-High | Two different wire protocols, connection secrets and destructive-action guards; mature native alternatives already cover this breadth[13][15]. Deferring loses less than shipping a shallow implementation; SQLite is a cheaper future entry point but is not part of this suggested item |
| 8 | Tool discovery/install | **CUT** (retain FR-G01 read-only detection) | High | Medium | Installers are a supply-chain trust problem: asdf-style plugins execute with user permissions[17], and the local threat model leaves registry signing and updater rollback as human-review gaps. Detection gives most of the "what do I have" value with no network writes; installation moves behind Gate 2 |
| 9 | YAML/JSON config editor | **KEEP** (narrow: validation, formatting, diff-friendly text) | Low-Medium | High | Self-contained, offline-capable, and immediately useful for every workspace; it is the entry point to the later configuration studio rather than requiring it |
| 10 | Settings | **KEEP** | Low | High | Required for themes, language, keybindings and telemetry consent; much of the plumbing already exists locally (`docs/progress.md`) and it is a hard prerequisite for the i18n and privacy NFRs |

**Net verdict (DESIGN):** keep 6, defer 3, cut 1. The user's list is a reasonable *feature* inventory but bundles three expensive clusters too early; the revised MVP keeps its tenor while moving complexity behind explicit gates.

## 11. Success criteria (measurable)

1. **Task completion:** at least 90% of 10+ usability participants complete J1 (open project → edit → terminal → commit) in under 5 minutes with no other application, tested offline (DESIGN).
2. **Performance:** cold start p95 < 1 s, idle core RAM < 150 MB, keystroke-to-screen p95 < 16 ms and ≥60 fps on the reference machine across Windows, macOS and Linux (DESIGN; budgets from `VISION.md`, currently unmeasured per `docs/progress.md`).
3. **Data integrity:** zero lost edits or corrupted configs across 100 crash-injection sessions; 100% of config migrations verified with a backup (DESIGN).
4. **Privacy and offline:** telemetry remains off and no unexpected network egress is observed in any P0 journey; all P0 journeys pass with the network disabled (DESIGN).
5. **Inclusive baseline:** 100% en/vi key parity, all P0 journeys keyboard-only, and 100% of interactive controls in tested panels present in the accessibility tree (DESIGN; AccessKit as the target abstraction[28]).
6. **Engineering health:** CI green on all three OS families for two consecutive releases, with zero open P0 security findings and all shipped dependencies within the permissive license policy of local ADR 0004 (DESIGN).

## 12. Kill criteria

1. **Performance:** if two or more target OS families miss the <1 s start, <150 MB idle RAM or <16 ms keystroke budgets after two dedicated optimization sprints, stop expanding scope and re-architect the UI path or abandon the native-all-in-one thesis (DESIGN).
2. **Editor viability:** if the editor plus LSP sidecar cannot match a defined VS Code baseline on the four core LSP operations within two sprints of Gate 2, remove LSP from the 1.0 promise rather than ship an unreliable imitation; the protocol itself is standardized[21], so failure would be an implementation problem.
3. **Security:** if the native-sidecar sandbox and MCP/API authentication remain unresolved human-review gaps at public beta, ship with no untrusted extensions or remote API server; the local threat model flags both.
4. **Product pull:** if fewer than 60% of validated target users say they would replace at least one daily tool with Devdeck after the Foundation release, reposition it as a companion app and stop building replacement breadth (DESIGN).
5. **Accessibility:** if the Iced accessibility gap cannot be closed upstream or in a maintained fork before public beta, do not claim screen-reader support and restrict the 1.0 audience accordingly; local ADR 0001 documents the gap, while AccessKit is the intended remedy[28].

## 13. Assumptions, risks and open questions

**Assumptions (DESIGN):** users value consolidation enough to tolerate a less mature editor than VS Code; a minimal editor plus terminal plus Git is a compelling first release; the en/vi launch market is sufficient for early validation; tool installation can wait without blocking onboarding.

**Risks:** upstream Iced 0.14 API churn and missing accessibility integration (local ADR 0001); extension/sidecar isolation; daemon and credential complexity in D/E/F; protocol version churn for LSP/DAP/WIT (local ADR 0002); unmeasured performance budgets.

**Open questions:** (1) What reference hardware and OS versions define the performance targets? (2) How much editor capability is acceptable without LSP in the first public release? (3) Who owns the native-sidecar OS sandbox design, and before which gate? (4) Is repository signing a prerequisite for any tool installation, or only for third-party extensions? (5) Which single database and which single API protocol have the highest value for the first paid cohort? (6) Will the accessibility fork be contributed upstream or carried locally? (7) Does the phased plan need a user-visible preview channel to validate JTBD-12 before snapshots are built?

---

## Sources

[1] https://code.visualstudio.com/api/language-extensions/language-server-extension-guide — VS Code Language Server Extension Guide
[2] https://code.visualstudio.com/api/extension-guides/debugger-extension — VS Code Debugger Extension guide (DAP architecture)
[3] https://zed.dev/blog/videogame — Zed blog: Leveraging Rust and the GPU to render UIs at 120 FPS
[4] https://zed.dev/blog/zed-1-0 — Zed blog: Zed is 1.0
[5] https://github.com/docker/for-mac/issues/7749 — docker/for-mac issue 7749: com.docker.krun memory usage
[6] https://www.lihuanyu.com/en/posts/2025/rethinking-docker-development-linux-redis — Docker Performance: Linux vs Docker Desktop Overhead
[7] https://plugins.jetbrains.com/docs/intellij/intellij-platform.html — IntelliJ Platform Plugin SDK: The IntelliJ Platform
[8] https://jetbrains.com/help/idea/discover-intellij-idea.html — IntelliJ IDEA overview
[9] https://jetbrains.com/help/idea/managing-plugins.html — IntelliJ IDEA: Managing plugins / Marketplace
[10] https://code.visualstudio.com/blogs/2018/08/07/debug-adapter-protocol-website — VS Code blog: New home for the Debug Adapter Protocol
[11] https://www.postman.com/product/api-client — Postman API Client product page
[12] https://learning.postman.com/docs/use/send-requests/protocols/protocols — Postman Docs: Other API protocols in Postman
[13] https://tableplus.com/docs/overview — TablePlus Docs: Overview of supported databases
[14] https://tableplus.com — TablePlus homepage: native database client
[15] https://github.com/dbeaver/dbeaver — DBeaver GitHub README: universal database tool
[16] https://mise.jdx.dev/dev-tools — mise-en-place: Dev tools documentation
[17] https://mise.jdx.dev/dev-tools/comparison-to-asdf.html — mise: Migrating from asdf / plugin security
[18] https://survey.stackoverflow.co/2025/technology — 2025 Stack Overflow Developer Survey: Technology
[19] https://stackoverflow.co/company/press/archive/stack-overflow-2025-developer-survey — Stack Overflow 2025 Developer Survey press release
[20] https://survey.stackoverflow.co/2025 — 2025 Stack Overflow Developer Survey: home
[21] https://microsoft.github.io/language-server-protocol — Official page for Language Server Protocol
[22] https://microsoft.github.io/debug-adapter-protocol — Official page for Debug Adapter Protocol
[23] https://www.openapis.org — OpenAPI Initiative: What is OpenAPI?
[24] https://grpc.io/docs/what-is-grpc/introduction — gRPC: Introduction to gRPC and protocol buffers
[25] https://docs.docker.com/compose — Docker Compose documentation
[26] https://link.springer.com/chapter/10.1007/978-1-4842-4221-6_12 — Developers' Diverging Perceptions of Productivity: The Cost of Context Switching
[27] https://www.research.autodesk.com/app/uploads/2024/02/Task-Centric-Application-Switching_GI23.pdf — Task-centric Application Switching (Autodesk Research)
[28] https://accesskit.dev — AccessKit: Accessibility infrastructure for UI toolkits
[29] https://tree-sitter.github.io/tree-sitter — Tree-sitter: incremental parsing library
