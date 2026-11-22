# Dodo (MrGru/dodo) — evidence-based repository research

**Purpose.** Input to the Devdeck research initiative only. No product code was changed. This report is based on a scratch clone at pinned commit `b7b964d7916c059789cbceefb7d9941569dadd5c` (2026-11-16, `chore: bump version`), on Cargo manifests, actual source files, checked-in architecture docs, repository-local agent instructions, and GitHub metadata retrieved on 2026-11-22. GitHub metadata: 7 stars, 2 forks, 0 open issues, MIT declared, created 2026-07-15, last push 2026-11-16, archived=false [34]. Releases include v0.2.1 … v0.2.11, with v0.2.11 published 2026-11-16 [52]. All four GitHub issues are closed pull requests (#1 Mermaid, #2 Docker layout fix, #3 IME window fix, #4 database local-container prefill) [36]. Local shallow-clone measurements: 764 tracked files, 581 `.rs` files, ~220,262 Rust lines, 2,985 `#[test]` attributes, 51 commits in the depth-50 history, 16 workspace packages, four contributor names. The build and test suite were **not** executed here; counts and measurements are from source and checked-in records, not a fresh test run.

## 1. Executive assessment

Dodo is a pre-1.0, single-author-led, macOS-first native Rust developer toolbox that is unusually strong as an **architectural reference** for Devdeck: it has a real GPUI/gpui-component application, a disciplined workspace of 15 feature/kernel crates plus the binary, a documented pure-core/UI-boundary convention enforced by tests, and unusually candid records of what is implemented, what is a placeholder, and what was deliberately rejected [2][5][7]. Its weaknesses are equally relevant: it is young (created July 2026, 7 stars), its platform matrix is CI-built but largely untested outside macOS, its documentation contains internal contradictions, and its dependency graph contains GPL-3.0-or-later code reached unavoidably through GPUI, with binary redistribution terms explicitly left unresolved [2][4][6].

The single most transferable idea is not a widget or a crate but a rule: **split by purity and by seam, not by file count or build speed**, with cross-crate globals placed where every user can name the same type, and outside-world I/O confined to a `services` layer called only from GPUI's background executor [7][9][18].

## 2. GPUI architecture and how it is used

### 2.1 Dependency shape and version pinning

The app does not depend on raw `gpui` directly. It depends on `gpui-kit` 0.6 with `default-features = false, features = ["component"]`, described in the manifest as "the 0.6 facade [that] owns the compatible GPUI, platform and component versions"; disabling defaults keeps upstream's asset bundle out because `src/assets.rs` is the sole asset source [5] (Cargo.toml L209–L212). The repository-local GPUI recipe pins `gpui-component` to git revision `3c270ed` from `Cargo.lock` and warns that upstream GPUI skills are stale on three APIs: `window.open_dialog` (not `open_modal`), `gpui_component::setting` (singular), and `SelectState::selected_index(cx)` / `selected_value()` (no `selected_item()`) [30] (L6–L15). Feature crates repeat the exact `gpui-kit` dependency line, so all feature code compiles against the same facade [17][21][23].

### 2.2 Application shell, entities and render layers

`src/main.rs` constructs the application with `gpui_kit::application().with_assets(Assets).with_quit_mode(QuitMode::LastWindowClosed)`, calls `gpui_kit::init(cx)` as the first statement inside `app.run`, then opens a single window whose first-level view is `DodoApp` wrapped in GPUI Kit's `Root` [12] (L264–L300, L438–L449). `DodoApp` holds one `Entity<Layout>`, and its `Render` implementation explicitly calls `Root::render_dialog_layer(window, cx)` and `Root::render_notification_layer(window, cx)` as children beside the layout: `window.open_dialog` only pushes dialog state, so omitting those calls produces a dialog that exists in state but never paints [13] (L18–L36). The same contract is repeated in the recipe document and in standalone feature-crate launchers [30][25].

`Layout` is the shell: it holds `active: View`, a user-ordered `Features` list, a `FocusHandle`, retained `Subscription`s for window bounds / update notifications / macOS activation, and one live `Entity` per tool in the macro-generated `Panes` struct [15] (L196–L240). All tool panes are constructed once in `Layout::new` and never rebuilt; the one special case is Docker, whose 5-second polling must be started manually when the restored session opens directly on it [15] (L248–L285). The shell deliberately draws a custom title bar that reserves room for native OS window controls, an icon-only 56 px sidebar rail and a main pane, with `window_min_size()` derived from those constants [15] (L113–L145).

### 2.3 Actions, key contexts and focus

Actions are declared with GPUI macros (`actions!(dodo, [CloseWindow])`, `actions!(database, [DatabaseCopyCell, DatabaseCopyRow])`, `#[action(namespace = dodo, no_json)] struct ActivateSection(usize)`), and `cx.bind_keys` registers `KeyBinding`s scoped to `quick_nav::KEY_CONTEXT` [12][14][15]. The ordering rule is load-bearing: every feature's `init` runs **after** `gpui_kit::init` because equal-depth ties go to the later binding; API Explorer, Docker, Database, Flow, Mermaid, quick navigation, updater and section-switch inits are all called in that order in `run_app` [12] (L289–L336).

The recipe document records the underlying GPUI semantics with source-file references: a binding with `None` context counts as deepest and therefore beats focused-widget bindings; all matching bindings are sorted deepest-first and dispatched until one stops propagation; `!Foo` is evaluated against the whole focus path; and with nothing focused the dispatch path is only the window root, so a view that needs a binding before any click must own a `FocusHandle`, call `track_focus` and take focus in its constructor [30] (L17–L55). Dodo's `Layout` does exactly that for vim-style normal mode [15] (L212–L222). A second trap is documented from real debugging: `track_focus`'s bubble-phase mouse-down listener runs after a handler and can steal focus back, so `window.prevent_default()` must be called and tests must dispatch real events rather than call handlers [30] (L57–L75).

### 2.4 Background work, globals, subscriptions

Every blocking external call is "blocking by contract": it may only run on GPUI's background executor, never on the UI thread. This is stated for the API Explorer `Transport`, the Database `Driver`, and the Docker `DockerEngine` [24][22][18]. Session and settings are loaded in a `cx.spawn(async move |cx| …)` before the window opens, and session saves are coalesced by dropping/replacing `Task`s plus a `flush_on_quit` hook registered with `App::on_app_quit` [12][16]. Docker list views poll with `cx.background_executor().timer(POLL_INTERVAL).await` and a 5-second constant; the polling section is deactivated when the user leaves Docker [18][60].

Cross-cutting app state uses GPUI `Global`s identified by type: `Session`, `QuickNav`, `Tray`, `InputMethod`, `Updater`, `AvailableUpdate`, `ScriptPolicy`, `ActiveLanguage`, and the generic `OnScreen<K>` single-dialog slot [16][24][29]. `ActiveLanguage` lives in `dodo-i18n` [56]. Because a `Global` is keyed by its Rust type, two crates each defining "the same" global would be two unrelated instances; that is why the active-language global and `t()` live in `dodo-i18n` and the dialog slot in `dodo-dialog-slot` rather than in the binary [29][7]. `dodo-dialog-slot::claim` checks `window.has_active_dialog` against the believed flag and has a pure `decide_open` transition for stale flags [29] (L24–L41, L71–L87).

### 2.5 GPUI-specific UI recipes worth reusing

- **Flex semantics:** `h_flex()` bakes in `items_center()`, so a row whose children must fill height needs `.items_stretch()` on the row; `v_flex()` sizes height through `flex_1()`; keep `min_h_0()` / `min_w_0()` on panes to remove flex automatic-minimum-size floors [30] (L115–L136).
- **Scroll containment:** GPUI measures a scroll container from its direct children; a `size_full()` child clips tool content, so dodo uses a flex child with `flex_grow(1.)`, `flex_shrink_0`, `min_w`/`min_h` inside a column `overflow_scroll()` container [15] (L147–L194).
- **Editors:** `InputState::new(window, cx)` needs `&mut Window`, hence all view constructors thread the window from `Layout::new`; `code_editor(lang)` must be called before `line_number()` or debug builds panic; highlighting is a compile-time feature, not a runtime fallback [30] (L138–L217).
- **Trait imports:** `ButtonVariants` lives in `gpui_component::button`, `Sizable`, `ActiveTheme`, `StyledExt` in `gpui_component`, `FluentBuilder` in `gpui::prelude` — "method not found" is usually a missing trait import [30] (L97–L113).
- **Platform values, not `cfg` items:** value choices use `HostOs` or `cfg!` so both answers type-check on every target; only genuine platform API calls remain behind `#[cfg]` [26][28][7].

## 3. Native Rust UI patterns

Dodo is explicitly "No browser tabs. No Electron." [2] (L7–L12). Its native patterns include: per-platform data directories resolved from the compile-time target triple rather than `#[cfg]`, so all three branches are unit-testable from any host [28] (L10–L21). `cfg!(target_os = "windows")` in a `windows_subsystem = "windows"` attribute only for release builds, with `attach_parent_console` for `--version`/`--build-info` [12] (L1–L10). `QuitMode::LastWindowClosed` overridden to `Explicit` at runtime only once the tray icon exists, avoiding an unquittable windowless process [12] (L269–L284). AppKit/objc2 used for exactly one call (the Dock icon of a directly-run binary) because GPUI exposes no dock-icon API [5] (L277–L300). Target-gated `windows-sys` for `GetStdHandle`/`AttachConsole` [5] (L269–L276).

Cleanup safety is implemented natively: application icons are bounded to 32 KiB behind an `Arc` because the previous `NSImage TIFFRepresentation` path measured 73,949,448 bytes per application and could not be decoded [25] (L51–L62). Platform-specific scanners are modules under `crates/dodo-cleaner/src/{macos,windows,linux}`, with pure inventory policy compiled under `#[cfg(any(target_os = ..., test))]` so fixture tests run on the development host [25] (L90–L109).

## 4. Workspace and module organization

`cargo metadata --no-deps --offline` on the clone reports **16 workspace packages**: the root binary `dodo` plus 15 crates. The root manifest uses `members = ["crates/*"]`, an explicit `default-members` list containing all 16 packages, and `exclude = ["tools/update-manifest", "patches/gpui-pre-macos", "repro"]` [5] (L43–L68). The manifest comments state that `default-members` is load-bearing (a bare `cargo test` silently stops covering a crate omitted from it) and that `workspace.exclude` matches paths, not globs, so `tools/*` would exclude nothing [5][7].

The documented grouping is [5][7]:

- **Kernel crates (4):** `dodo-app-icon` (icon enum), `dodo-i18n` (string catalogue; pure by default, optional `gpui` feature for `t()` and active language), `dodo-paths` (dependency-free platform path rules), `dodo-dialog-slot` (single dialog per marker type). They do not depend on each other.
- **Feature crates (9):** `dodo-cleaner`, `dodo-docker`, `dodo-database`, `dodo-api-explorer`, `dodo-updater`, `dodo-input-method`, `dodo-json-formatter`, `dodo-encoder-decoder`, `dodo-flow`. `dodo-mermaid` is a tenth tool crate added after the doc was written and is present in `default-members` and as a workspace dependency [5].
- **Engine crate:** `dodo-ime-core`, the pure Vietnamese Telex/VNI input engine behind `dodo-input-method`.
- **Excluded helpers:** `tools/update-manifest` (release engineering, own lockfile, zero bytes in the shipped binary), `patches/gpui-pre-macos` (vendored GPUI package for a window-lifecycle reproduction), `repro/` (standalone GPUI reproduction) [5].

`src/` is only the shell and cross-feature services; `src/main.rs` aliases each feature crate back to its former module name (`use dodo_cleaner as cleaner;`) so call sites on both sides of the extraction stayed unchanged [8][7]. `src/tools.rs` is a **table, not a registry**: a `tools!` macro generates the `View` enum, platform filter, sidebar metadata, paste-detector mapping and the `Panes` struct from one row per tool; the compiler makes an unclaimed detector or a detector claimed twice an error [14] (L90–L318, L320–L400). Adding a tool is one table row plus its crate. The table currently has nine rows, with the Input method gated to macOS/Windows and Diagram added last [14]. Crate extraction was measured and **did not improve build time** (clean build −1.2%, touching a leaf file 7.9% slower, touching `src/layout.rs` 5.3% faster against a noise floor of 32%), so the stated reason for crates is boundaries, testability and visible public surfaces [7] (L155–L171).

**Documentation drift observed:** the manifest header says "fourteen members", the architecture page says "fifteen packages", and `default-members` plus `cargo metadata` show 16 packages (root + 15 crates) [5][7]. `dodo-database/src/lib.rs` still says the local-container prefill was "dropped in every round", while the 2026-11-16 commit, `src/main.rs` L302–L305/L414–L436 and `crates/dodo-database/src/models/local.rs` show it implemented through an injected function pointer [22][12][54]. `dodo-docker`'s local skill calls the crate "feature-complete as of round 6" while `lib.rs` documents a seventh round (host runtime detection) and explicitly named placeholders [31][18]. Treat prose in this repo as a dated record and prefer source plus tests.

## 5. Docker and Podman integration

### Implemented

`crates/dodo-docker` is a four-list-page engine client plus a fifth host-runtime page; 16 of 43 files name a UI framework and the other 27 are pure `models`/`services`/`state` [18][17]. `bollard` 0.21 is allowed only in `services::engine`, with a single multi-thread tokio runtime driven by `Runtime::block_on` from GPUI's background executor; TLS features are deliberately off so the dependency stays OpenSSL-free [17].

Connection discovery is hand-rolled because bollard's defaults are insufficient. The documented order is `DOCKER_HOST` → `/var/run/docker.sock` → macOS Docker Desktop `~/.docker/run/docker.sock` → macOS running `podman machine` socket → `connect_with_podman_defaults()` for Linux rootless/system sockets [19] (L137–L193). Two encoded traps: the standard socket is often a dangling symlink on a Mac that once ran Docker Desktop, and bollard's Podman defaults only probe Linux paths, missing `$TMPDIR/podman/podman-machine-default-api.sock`; the latter is discovered by reading that directory, never by shelling out to `podman` (which may not be on a Finder-launched app's PATH), with a deterministic default-machine/sort rule [19] (L195–L233).

Implemented surfaces: Containers with compose grouping, multi-filter popover, bulk actions and keyboard row navigation; Images, Volumes and Networks as real list pages over a generic resource store; 5-second background polling with incremental merges; read-only Inspect for all four resource types and a bounded non-following container log tail, presented as two tabs of one `window.open_dialog`; and a Runtimes page that detects Docker, Podman Machine, Kubernetes and containerd by invoking each CLI and offers Start/Stop where meaningful [18][31][20]. Kubernetes is deliberately read-only because no Start/Stop command is correct for every local-cluster provider, and platform gaps are pure functions of `HostOs` (Podman Machine has no meaning on Linux; containerd has no standalone service on macOS/Windows) [20]. Compose project labels honour Docker's labels first and Podman's `io.podman.compose.*` conventions second [18]. Docker running containers, with only published ports, are exposed through `docker::running_containers()` and injected into the Database Explorer by `main.rs` [12][55].

### Planned / explicitly not implemented

`crates/dodo-docker/src/lib.rs` L106–L130 lists disabled "Coming soon" controls: interactive **Exec/Open Terminal** (needs `create_exec`/`start_exec` and a bidirectional stream, which breaks the blocking-by-contract shape), **Create container**, **Pull** and **Build** images (plus a progress stream for pull), **Stats beyond live CPU%** (memory/network/block IO and a history ring buffer), **Favorites** (persisted ids; predicate seam `state::filters`, persistence seam the API Explorer's `DiskCollectionStore`), plus log following, log filtering and ANSI colour parsing [18]. README also lists "additional Docker operations" as a known gap [2].

## 6. Database management

`crates/dodo-database` implements a `Driver` trait with four backends: PostgreSQL, SQLite (bundled C amalgamation with `column_metadata` and `column_decltype`), MySQL/MariaDB through one shared-protocol client, and Redis/Valkey as the first non-SQL store [21][22]. Blocking clients were chosen over `sqlx` because every backend exposes real out-of-band cancellation (`Client::cancel_token`, `sqlite3_interrupt`, `KILL QUERY`) and `sqlx` does not; rustls-only configuration avoids OpenSSL [21] (L35–L87). The `postgres` crate secretly builds a private current-thread tokio runtime per client, so Docker's runtime is the only one dodo *constructs*, not the only one that exists [22][31].

Implemented workflows: saved connections and lazy object tree; query tabs with protocol-level cancellation and streamed CSV/JSON export; table/view detail surfaces (Data, Columns, Indexes, Constraints, DDL); pending table-data edits with add/delete/duplicate row and explicit Commit/Rollback for SQL backends; connection-scoped saved queries, bounded persisted history and an in-memory catalog search that walks `Driver::children` off-thread (Redis uses `SCAN … TYPE`, never `KEYS`) [22] (L49–L82). Editing safety is a model-level boundary: wire column origins plus catalog primary/unique metadata yield `EditableSource`; SQL text is never parsed for identity; joins, unions, aggregates, computed columns, missing keys and Redis stay read-only; each driver runs one bound-parameter batch in a single transaction and rolls back unless every statement matched exactly one row [22] (L84–L104). Concurrent lost updates are explicitly not detected [22].

Persistence: `connections.json` stores **passwords in plain text**, `query-data.json` stores query text intentionally plain; both have explicit `"version"` fields, are read/written on the background executor, and refuse a higher-version file [9]. The connection form can be filled from a pasted URI for all four engines [22]. The local-container prefill is now implemented through the callback seam `database::observe_local_containers(local_database_containers)`, preserving the no-compile-time-edge rule between the Docker and Database crates [12][54].

Not implemented: favourites, saved-query folders/tags/sharing/import/export, tab restore, autocomplete, a background catalog daemon, and column sorting (server-side sorting was out of scope and sorting one bounded page would be dishonest) [22][2].

## 7. API Explorer

`crates/dodo-api-explorer` supports seven request body shapes, four authorization schemes, persistent collections/environments, a response viewer with a JSON tree, HTML text preview and Set-Cookie parsing [24] (L1–L18). The send pipeline is one blocking, unit-testable function in `services/send.rs` with the exact order **pre-request script → `{{name}}` substitution → `prepare` → `Transport::execute` → post-response script**; a failed pre-request script stops the send, while a failed post-response script preserves the response and becomes a Console line plus a Tests-tab error [33][61]. `reqwest` 0.13 is blocking, rustls-only, and confined to `services::http`; scripts run in QuickJS through `rquickjs` with no module loader and no `quickjs-libc.c` (so no file, process or socket capability), a positive intrinsic allowlist, 2 s deadline, 16 MiB and 256 KiB caps, and one fresh runtime per run [23][33][59]. `pm.test`/`pm.expect` are a JavaScript prelude, not Rust bindings; `pm.sendRequest` is denied by not existing [33]. Script provenance is attached to the request as `Imported` and cannot be laundered by editing; one consent key covers both hooks, approvals persist in `script-consent.json`, and the global "Run scripts" policy resets to `AskImported` at every launch [24][33][9]. No file is read on the UI thread: picks and `stat` are off-thread, and body bytes are read once at send time inside `prepare` [24][33].

Implemented extras: cURL paste-to-rebuild (pure parser plus a new tab), code generation to cURL/fetch/axios/XMLHttpRequest from one normalized request with a round-trip property test, and a secret-variable masking policy for generated code (a variable marked `secret` stays as `{{name}}`; credentials typed directly into the Auth tab are **not** masked, and the dialog says so) [33]. The crate's 77 files are split so 50 name no UI framework [24].

Not implemented: OAuth 2.0 (needs redirect flow and token store), `pm.sendRequest`, drag-and-drop reordering of collections (the model supports it), plus the README's "OAuth 2.0 in API Explorer" gap [24][2].

## 8. Component architecture

Each large feature crate follows the same five layers: `models/` (plain, GPUI-free, unit tested), `services/` (the only layer naming outside-world crates), `state/` (plain stores over traits/entities), `components/` (small reusable elements missing from the widget library), `views/` (rendering and GPUI imports) [7][18][22]. The API Explorer repeats the same split [24]. File-level counts from the checked-in docs: Cleaner 90 of 93 files name no UI framework; Docker 27 of 43 [25][18]. Database 35 of 52; API Explorer 50 of 77 [22][24]. Flow enforces the boundary with a test that scans an explicit list of ~65 pure files for any `gpui` token in code (comments excluded) [26].

Concrete reusable components include Docker's generic `StatusBadge`, `SearchBar`, `Toolbar`, `LoadingSkeleton`, `EmptyState`, `ErrorState` and one generic resource store reused by Images/Volumes/Networks [18]; Database's small widgets and a shared detail/commit/row-editor dialog family [22]; and API Explorer's five `components/` elements [24]. Flow's renderer has a node-renderer registry where three generic kinds register through the same public path a third party would use, plus a byte-bounded viewport-scoped geometry cache [26]. The shell's tool table is itself a component factory: `Panes::new` and `Panes::place` are generated from the same rows as the sidebar [14].

## 9. State management

Three distinct mechanisms are used deliberately:

1. **GPUI `Entity` + `Context` for view-local state** — one entity per tool, held for the process lifetime; `cx.update`/`cx.read`/`cx.notify()` and `cx.subscribe`/`cx.observe_global` drive repaint, with `Subscription` fields retained so they are not dropped and unsubscribed [14][15][16].
2. **GPUI `Global`s for app-wide policy** — session, active language, script policy, updater availability, input method, dialog slots; deliberately not duplicated across crates [16][24][29]. `ActiveLanguage` is the i18n example [56].
3. **Versioned JSON documents behind store traits for persistence** — eleven files under one `data_dir()`, including `session.json`, `collections.json`, `environments.json`, `script-consent.json`, `connections.json`, `query-data.json`, `quick-nav.json`, `cleaner-ignored-items.json`, `updater.json`, `input-method.json`, `flow.json` [9]. Every file except `collections.json` has an explicit version that is stamped on load and refuses a newer version; session saves are debounced 400 ms with a quit flush, and an unreadable file disables saving for the run so a newer file is never overwritten [9][16].

Not every setting is persisted: the "Run scripts" policy intentionally resets each launch while per-script approvals persist, and Docker's selected rail page plus Database/API Explorer tabs are not restored [16][9].

## 10. Performance characteristics

**Build and binary size (measured, macOS arm64, commit 9f88c698+changes):** stock release 31,237,488 B; shipped profile (`opt-level=3`, `codegen-units=1`, `strip="symbols"`, `lto="fat"`, `panic="unwind"`) 20,513,488 B, −34.3%; `panic="abort"` would save another 3,472,704 B but was rejected because unwind containment of background-task panics is worth more than 3.5 MB; fat LTO costs 27% more clean build time (5m42s vs 4m29s) and 2.26 GiB peak link RSS; `--no-default-features` (drop SQL/JS/JSON/HTML/YAML grammars) saves only ~1.5 MB from the shipped build [10]. A separate `release-debug` profile emits a 277 MB dSYM with matching UUID for symbolicated crashes [10]. `--no-default-features` disables the syntax highlighter entirely (stub returning no highlights), SQL highlighting is a separate feature because `tree-sitter-sequel`'s `parser.c` is 17.4 MB, and text editors degrade to plain text [5][30][10].

**Not measured by the project:** startup time, frame time and runtime memory on Linux/Windows; LTO runtime effects; Windows linker flags; `cargo test` was not run for this report either [10].

**Canvas performance (measured by the project, Apple M1, 2026-08-19):** uniform-grid viewport query 2.3 µs at 100,000 nodes / 300,000 edges versus 1,863 µs scanning, and flat at 5,000 nodes; one node moved in a 300,000-edge graph 0.51 µs; tessellating a dense frame 3.08 ms (19% of a 16.7 ms frame); worst scene 132,888 painted vertices (5.5% of a 2.4 M ceiling) and 1 path batch (budget 64); memory 128.8 MB runtime + 23.9 MB index, 205 MB resident [26]. A hairball graph defeats culling (61,104 visible edges, estimated 147 M vertices), which is why LOD is mandatory; a paint-cost model was deleted after two failed attempts to measure `Window::paint_path`, with the windowed harness committed for a human to run [26]. The renderer counts vertices before painting and degrades to a simpler representation: exceeding the Metal instance-buffer ceiling produces a uniformly black window with no panic, reproduced at 13,000 200-vertex edges [26][37]. Windows/Linux budget rows are explicitly `Unmeasured` with a safety discount, not assumed equal to macOS [37]. Instrumentation is a 10-probe enum with counters; off by default (one bool test), about 40 ns per probe when on, and a view field rather than a global or cargo feature so benchmarks exercise the shipping program [38].

**Concurrency discipline:** blocking DB/HTTP/Docker calls only on GPUI's background executor; file writes and session loads off the UI thread [12][7]. Polling runs only while a Docker page is visible; the root binary names no async runtime [18][22][23].

## 11. Reusable modules and crates for a Devdeck-like app

- `crates/dodo-paths/` — dependency-free, target-triple-based platform path rules with all branches testable anywhere [28].
- `crates/dodo-i18n/` — pure `Str`/`Text` catalogue by area with exhaustive language files; `gpui` feature adds the single active-language global and `t()` [27][56].
- `crates/dodo-dialog-slot/` — typed global enforcing one dialog per marker type, with a pure `decide_open` state transition [29].
- `crates/dodo-app-icon/` — one icon enum over embedded `assets/icons`, consumed by 39 modules [5].
- `src/tools.rs` — declarative tool table generating enum, metadata, detector mapping and pane storage (a pattern, not a dependency) [14].
- `crates/dodo-docker/src/services/engine.rs` — production-grade engine/socket discovery order for Docker Desktop and Podman Machine [19].
- `crates/dodo-database/src/models/identity.rs` + `models/statement.rs` — safe, metadata-driven row-edit boundary (paths named in the crate doc) [22].
- `crates/dodo-api-explorer/src/services/{send.rs,script/,curl.rs,codegen/}` — ordered blocking send pipeline, sandboxed scripting, pure cURL parse and normalized codegen [33][61].
- `crates/dodo-flow/src/{budgets.rs,instrument.rs,spatial/,runtime/,render/plan.rs}` — measured render ceilings with provenance, named probes, uniform grid, SoA graph stores and a paint plan that cannot express offscreen paths [26][37][38].
- `.claude/skills/gpui-component-recipes/SKILL.md` — an empirically verified GPUI/gpui-component API cookbook (key contexts, focus, dialogs, flex, editors, trait imports) [30].
- Release engineering: `deny.toml`, `release`/`release-debug` profiles, `SHA256SUMS` sidecars, CI workflows for analysis/ci/commit-lint/release [6][10][2].

Reuse caveat: all of these crates are MIT-licensed by their own manifests but sit in a dependency graph with the unresolved GPL question below, and several are tightly coupled to `gpui-kit` 0.6's exact API [5][4].

## 12. License and third-party dependency risks

Dodo's own source is MIT (LICENSE, `Cargo.toml` L70–L79), but the project explicitly records an **unresolved redistribution question**: the statically linked dependency graph contains three GPL-3.0-or-later crates — `ztracing` 0.1.0, `zlog` 0.1.0, `ztracing_macro` 0.1.0 — reached through `gpui` → `sum_tree` → `ztracing`; they are not behind a feature dodo controls, so every build links them [3][4]. README repeats that the implications for redistributing built binaries are an open question [2]. `deny.toml` is deliberately **advisory, not a gate**: its CI job is `continue-on-error: true`, and GPL-3.0-or-later is kept out of `allow`/`exceptions` so the finding is reported on every run; cargo-deny 0.20.2 reportedly rejects exactly those three crates under `check licenses` [6].

Other recorded risks: `nucleo-matcher` and `option-ext` are MPL-2.0; two crates (`gpui_shared_string`, `gpui_util`) declare no license field; `rquickjs-sys` vendors MIT QuickJS-NG C sources into the binary; `aws-lc-sys` carries a complex conjunction of ISC/Apache/MIT/BSD terms; the locked graph has ~466 packages including 54 other permissive, 19 Unicode-3.0, 2 MPL-2.0 and 1 `bzip2-1.0.6` entry [4]. Plain-text database passwords in `connections.json` and secrets in generated code are product-security issues adjacent to licensing [9][33]. No legal conclusion is asserted here; the upstream documents deliberately decline to reach one [4][6].

## 13. Implemented vs planned — consolidated

| Area | Implemented (source evidence) | Planned / not built (source evidence) |
|---|---|---|
| Shell | One window, custom title bar, icon rail, nine-row tool table, session restoration, tray on macOS/Windows, updater, quick navigation [12][14][15][16] | Linux input-method integration; interactive window lifecycle tests on Windows [2][8] |
| Docker/Podman | 4 resource pages + Runtimes page, polling, compose grouping, filters, bulk actions, Inspect, bounded logs, modal detail, engine discovery incl. Podman Machine [18][19][20] | Exec/Terminal, Create/Pull/Build, extended stats, Favorites, log follow/filter/ANSI [18] |
| Database | 4 drivers, lazy tree, query tabs, cancel, export, table-data edits with Commit/Rollback, saved queries, history, search, local-container prefill [22][12] | Autocomplete, column sorting, favourites, query folders/tags/sharing/import/export, tab restore, catalog daemon [22] |
| API Explorer | HTTP send pipeline, variables, 4 auth schemes, collections, environments, both script hooks in QuickJS, consent, codegen, cURL paste, response views [24][33] | OAuth 2.0, `pm.sendRequest`, drag-and-drop collection reorder [24] |
| Diagram/Flow | Infinite canvas, pan/zoom/box-select, graph engine, LOD, caches, text, rich nodes, images, editing, rotation, groups, align, persistence, app seam [26] | Human-only validation of rotation feel, drill-in ergonomics, control legibility; Windows/Linux render budgets unmeasured [37] |
| Cleaner | 14 macOS / 8 Windows+Linux categories, incremental scan, move-to-trash, review dialog, bounded icons [25] | Platform-specific rough edges outside macOS [2] |
| Packaging | Release archives for macOS arm64/x64, Windows x64, Linux x64; SHA256SUMS; `dodo --build-info` [2] | macOS arm64 is the only real-device tested target; other platforms are CI build only [2] |

## 14. Caveats and open questions

1. **Documentation drift is real** (three examples in §4); when using Dodo as a reference, verify against source and tests, not prose. The internal `.claude/skills` documents are unusually useful but dated in places [30][31][22].
2. **No fresh build/test execution** was performed in this research; measured numbers are the project's own checked-in measurements and local counts from the clone [10][26].
3. **Platform confidence is asymmetric:** README says macOS arm64 is tested and Windows/Linux are CI-built only, and the build-optimization document says dodo had never been built on Linux or Windows (a statement that may itself now be stale given CI) [2][10].
4. **The GPL/binary-distribution question remains open upstream** and would need a decision before reusing the GPUI stack in a distributed Devdeck [4][6].
5. **Persistence security:** plain-text saved DB passwords and a per-launch script-consent policy are explicit design choices worth a separate Devdeck decision [9][24].
6. **API maturity:** 7 stars, 2 forks, 0 non-PR issues, 4 PRs total, 51 commits in the shallow history and a single dominant contributor make this a young, lightly peer-reviewed project despite high internal engineering quality [34][36].

## Sources

[2] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//README.md
[3] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//LICENSE
[4] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//THIRD-PARTY-NOTICES.md
[5] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//Cargo.toml
[6] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//deny.toml
[7] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//docs/architecture/workspace-layout.md
[8] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//docs/architecture/app-shell.md
[9] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//docs/architecture/persistence.md
[10] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//docs/build-optimization.md
[12] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//src/main.rs
[13] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//src/app.rs
[14] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//src/tools.rs
[15] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//src/layout.rs
[16] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//src/session/mod.rs
[17] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-docker/Cargo.toml
[18] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-docker/src/lib.rs
[19] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-docker/src/services/engine.rs
[20] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-docker/src/models/runtime.rs
[21] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-database/Cargo.toml
[22] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-database/src/lib.rs
[23] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-api-explorer/Cargo.toml
[24] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-api-explorer/src/lib.rs
[25] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-cleaner/src/lib.rs
[26] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-flow/src/lib.rs
[27] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-i18n/src/lib.rs
[28] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-paths/src/lib.rs
[29] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-dialog-slot/src/lib.rs
[30] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//.claude/skills/gpui-component-recipes/SKILL.md
[31] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//.claude/skills/dodo-docker-internals/SKILL.md
[33] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//.claude/skills/dodo-api-explorer-internals/SKILL.md
[34] https://api.github.com/repos/MrGru/dodo
[36] https://api.github.com/repos/MrGru/dodo/issues?state=all&per_page=30
[37] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-flow/src/budgets.rs
[38] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-flow/src/instrument.rs
[52] https://api.github.com/repos/MrGru/dodo/releases?per_page=5
[54] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-database/src/models/local.rs
[55] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-docker/src/models/running.rs
[56] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-i18n/src/ui.rs
[59] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-api-explorer/src/services/script/quickjs.rs
[60] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-docker/src/views/containers.rs
[61] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//crates/dodo-api-explorer/src/services/send.rs
