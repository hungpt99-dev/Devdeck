# OpenIDEA (`l0ng-ai/openidea`) — Evidence-Based Research Report

**Research scope:** source-level investigation for the Devdeck "all-in-one native Rust desktop app" initiative. No product code was modified. Analysis is of a scratch clone at `/root/.hermes/cache/scratch/openidea-research`, pinned to `main` commit `c209e189e082fe4b59e00ee9d7b13a19d0ae6458` (22 Nov 2026). Every fileCitation below is a GitHub blob URL at that exact commit; those URLs were checked to return HTTP 200.

**Method:** `git clone`, direct reading of `Cargo.toml`, `package.json`, `src-tauri/`, all 47 `crates/`, `src/editor/`, `languages/`, `plugins/`, `perf/`, `docs/`, LICENSE/NOTICE/THIRD_PARTY_LICENSES, plus GitHub REST API for stars, releases, issues and commit history. README claims are labelled as claims; nothing was compiled, run or benchmarked locally.

---

## 0. Executive summary / verdict on the Devdeck requirement

| Question | Verdict | Evidence |
|---|---|---|
| Is it a serious, working IDE? | Yes — large, actively developed, released | [2][5][6] |
| Is the core Rust? | Yes — 47 Rust crates, ~442k lines of Rust | [2] |
| Is the **UI** 100% native Rust? | **No.** Tauri 2 webview running React 19 + TypeScript + CodeMirror 6 + xterm.js | [8][9][10] |
| Can Rust components be reused by Devdeck? | Potentially yes (Apache-2.0), but expect extraction work | [2][26] |
| License/redistribution risk | Low–moderate: Apache-2.0, but notice/attribution and trademark hygiene required | [19][26] |

**Bottom line:** OpenIDEA is a proof that a *Rust backend* can drive an IntelliJ-style IDE with excellent startup/memory characteristics, but it is explicitly a Tauri/webview app, not a native-Rust-UI app. Its backend crates (LSP, DAP, VCS, terminal, DB, HTTP, plugin runtime, text/index) are the reusable part for Devdeck; its UI layer is not reusable under a 100%-native-Rust requirement.[2][8][9]

---

## 1. Project identity, scale and maintenance

- GitHub repo `l0ng-ai/openidea`, described as "The IDE you know, rebuilt in Rust", created 2026-10-21, last pushed 2026-11-22, default branch `main`.[5]
- At research time: ~9,806 stars, 934 forks, `open_issues_count` 217 (this field includes PRs); a GitHub search restricted to issues gives **154 open issues** out of 1,790 total issues.[5][7][35]
- `main` has **3,415 commits** (GitHub commits API pagination) and the last 50 commits were all dated 21–22 Nov 2026 — extremely high velocity, with many small UI polish commits.[36]
- Releases are near-daily: v0.1.50 (2026-11-21), v0.1.49 (11-19), v0.1.48 (11-18), v0.1.47 (11-17), v0.1.46 (11-16), plus a `nightly` channel; first public release v0.1.0 was 2026-10-22.[6][20]
- Workspace version `0.1.50`, Rust edition 2024, MSRV 1.96, license Apache-2.0; 47 internal crates declared as workspace dependencies.[2]
- Rough source volume in the clone: 1,019 Rust files / ~442k lines, 2,779 TS/TSX files / ~592k lines — this is not a small prototype.[2][3]

**Implemented:** public releases, auto-updater, CI-quality docs site, contributor docs, release cadence.[5][6][20]
**Maturity caveat:** the project is roughly one month old with a `0.1.x` version and an explicitly "preview" plugin API; API and feature churn is high.[4][15][20]

---

## 2. UI technology and frontend dependency inventory (critical for Devdeck)

**The UI is not native Rust.** The desktop shell is Tauri 2 (`tauri = "2"`) using Wry, i.e. the OS webview: WebKit2GTK 2.0 on Linux, WKWebView via `objc2-web-kit` on macOS, WebView2 via `webview2-com` 0.39 on Windows.[9]
`tauri.conf.json` sets `beforeDevCommand: pnpm dev`, a Vite dev URL, the `../dist` frontend output, and a CSP with `'wasm-unsafe-eval'` and a `frame-src` for the `openidea-plugin:` scheme.[8]
`index.html` is a normal HTML document with a root div, a boot script, splash CSS and a TSX module entry point.[10]
The frontend editor is CodeMirror 6 (`src/editor/core/cm/setup.ts` and the `@codemirror/*` / `@lezer/*` packages).[34]

The project's own marketing is careful: "a native app with a Rust core" — "native" here means a packaged desktop app with a Rust backend, not a natively drawn UI.[4][29]

Frontend dependency inventory (from `package.json`, i.e. actual declared dependencies, not README):[3]

| Area | Packages |
|---|---|
| UI framework | `react` / `react-dom` ^19.1, `zustand` ^5, `immer` ^11, `clsx` |
| Editor | `@codemirror/*` (state, view, commands, language, search, lint, merge, autocomplete, lang-html/java/javascript/markdown/sql/vue/sass, legacy-modes), `@lezer/*`, `@replit/codemirror-vim`, `emmet`, `dompurify`, `marked`, `katex`, `mermaid` |
| Layout / i18n / fonts | `react-resizable-panels`, `@tanstack/react-virtual`, `@floating-ui/react-dom`, `@fontsource-variable/inter`, `@fontsource-variable/jetbrains-mono` |
| Terminal | `@xterm/xterm` 6.1.0-beta, `@xterm/addon-fit`, `@xterm/addon-search`, `anser` |
| Tauri bridge | `@tauri-apps/api` ^2, `@tauri-apps/plugin-dialog`, `@tauri-apps/plugin-opener` |
| Build/test | Vite 8, TypeScript 6, `tsrs`, `vitest` 5, Playwright 1.64, `oxlint`/`oxfmt`, `knip`, jsdom |

**Architectural consequence:** every pixel of editor chrome, tool windows and terminals is DOM/CSS in a webview; React/TS is a first-class, large part of the codebase. The Rust side is an IPC backend of commands and events (generated TypeScript bindings via `tauri-specta` into `src/bindings.ts`); `src-tauri/src/lib.rs` describes itself as "glue only", with no domain logic in the shell.[9]

---

## 3. Editor architecture

- **Split model:** Rust owns the authoritative document text; the UI sends CodeMirror change sets tagged with a version, and `ide-text::Document::apply` rejects stale edits with `ErrorKind::OutOfSync`. Text uses a rope (`ropey`), `\n`-normalized, with UTF-16 public coordinates to match CodeMirror and LSP.[31]
- **Syntax layer:** `ide-language` caches lazily-parsed, incrementally re-parsed tree-sitter trees per document; `ide-syntax` ("Structure service") answers navigation/usages/local-rename questions when an LSP server is unavailable or code does not parse — explicitly modelled on IDEA's "dumb mode" PSI, with deliberately conservative answers.[32]
- **Project-wide indexing:** `ide-index` and `ide-search` provide file/symbol navigation and search; a `[profile.dev]` build uses `opt-level = 0` for own crates but `opt-level = 2` for dependencies, with thin LTO and `panic = "unwind"` in release.[2]
- **Frontend editor:** CodeMirror 6 extensions implement keymaps, multi-caret, column mode, folding, sticky scroll, rainbow brackets, inline diffs, embedded/template languages, markdown rendering, Vim emulation, and IDE-style editing commands; LSP-facing TS modules cover completion, hover, diagnostics, inlay hints, refactorings, navigation, semantic tokens and workspace edits.[3][14][34]
- **Rendered/virtual documents:** `ide-text` plus TS modules support virtual docs, notebook cells and injected fragments (Angular/Vue/JS templating) for LSP requests.[11][31]

**Implemented:** rope document model with versioning, tree-sitter fallback, CodeMirror-based editor, IDE keymap parity work.[31][32][34]
**Not a native editor:** no native text widget; rendering and input handling live in CodeMirror in the webview.[3][10][34]

---

## 4. LSP implementation — which client, what works

**Client: OpenIDEA's own Rust LSP client**, not `tower-lsp`/`lsp-client` or a JS LSP client.[11]

- `ide-lsp` is a "language server manager: lifecycle, document sync, feature requests mapped to IDE types". It uses `lsp-types` 0.97 for protocol types and `ide-jsonrpc` for transport; the IPC surface exposes IDE types, never `lsp-types`.[2][11][28]
- `ide-jsonrpc` implements `Content-Length` framing over arbitrary async byte streams and a JSON-RPC 2.0 `Peer` with request/response correlation, `$/cancelRequest`, and newline-delimited JSON for the Agent Client Protocol.[28]
- Transports: servers spawn with **stdio** or are reached over **TCP** (`connect` servers, e.g. Godot GDScript); framing and session logic are shared.[11]
- Lifecycle: servers start lazily per `(server name, workspace root)` when a matching document opens; multiple servers may serve one language and their results are merged; `initialize` may take minutes for JDT LS / kotlin-lsp and shows "starting" meanwhile; timeouts are 5 s (completion), 10 s (default), 30 s (long).[11]
- Language definitions are data-driven: `languages/*.toml` contains 86 language files with **102 `[[language-servers]]` entries** and 28 `[debugger]` entries. Python, for example, uses basedpyright (with fallbacks pyright/ty/pyrefly), Ruff, and pylsp-rope limited to refactoring code actions, with install hints.[27]
- Extra machinery: file operations/watchers, virtual documents, notebook cells, injected documents, JS/TS file-move integration, a `tsserver` bridge, JDT LS extensions (hierarchy, extract interface, formatter, stream trace, JVM inspect), and 8 Criterion benches across the workspace (including an LSP `did_change` bench).[11][2]

**What works (documented features):** completion (basic, word, inline), parameter info, go to declaration/implementation/type, find usages, quick documentation/definition, inlay hints, diagnostics, formatting, code actions/quick fixes, rename, call/type hierarchy (server-dependent), structural search and duplicate detection from `ide-analysis`; certain-only fallback from tree-sitter when no server is running.[14][11][32]

**Implemented vs planned:** LSP client and servers are implemented.[11][14] Missing/conditional capability support is adapter/server-dependent (e.g. hierarchies need server support); no evidence of a from-scratch Rust language server for general languages — intellisense comes from external servers.[14]

---

## 5. Debugger architecture

**Client: OpenIDEA's own Rust DAP client** (`ide-dap`), reusing `ide-jsonrpc` framing; sessions emit IDE events and the UI pulls threads, frames, scopes and variables on demand, with 0-based coordinates converted at the adapter edge.[12][28]

- DAP adapters are external processes defined per language in `languages/<id>.toml` `[debugger]`; built-in launcher names include `codelldb`, `debugpy`, `js-debug`, `delve`, `netcoredbg`; java-debug runs inside JDT LS; `debugServer` can attach to a TCP port (`dlv --headless`, `debugpy --listen`).[12][13][27]
- Documented adapter coverage: Java (java-debug), Kotlin, C/C++/ObjC/Swift (`lldb-dap`), Rust/Zig (CodeLLDB), Go (Delve), Python (debugpy), JS/TS (vscode-js-debug), C#/F# (netcoredbg), PHP (Xdebug), shell (Bash Debug), Ruby (rdbg), Elixir, Dart/Flutter, OCaml, GDScript, plus GDB 14+ `gdb -i dap`; missing adapters get one-click install where possible.[13]
- Breakpoints: line, conditional, hit-count, logpoint/"evaluate and log", temporary, method (DAP function breakpoints), field watchpoints (DAP data breakpoints), exception filters; breakpoints persist per project and unverified ones are flagged.[12][13]
- Execution: resume/pause, step over/into/out, smart step into, force variants, run to cursor, evaluate in console, threads/frames/variables with library-frame folding; JVM-specific direct inspection exists via `ide-jdwp`, `ide-jvm-inspect` and `ide-heap` (heap/JVM memory inspection, exception classes, stream trace).[12][13][2]
- A very recent commit adds viewing pandas/numpy values as a table, showing active polish.[1]

**Limits (documented):** method breakpoints/watchpoints and exception filters require adapter support; some adapters cannot install automatically; macOS CodeLLDB/Delve may need Developer Mode; GDB is never installed by the IDE.[13]
**Implemented:** DAP client, breakpoints, stepping, evaluation, adapter installation; JVM deep inspection.[12][13]
**Planned:** not identified in the inspected sources beyond ongoing adapter additions (open issues include debugger fixes/features).[7][35]

---

## 6. Git integration

- `ide-vcs`: "Reads (status, diff, blame, log) use gitoxide in-process; writes (commit, checkout, push/pull, stash, merge) shell out to `git` so hooks, credentials and signing behave exactly as on the command line."[21]
- Uses `gix` (gitoxide 0.89) with status, blob-diff, revision, index, dirwalk and parallel features, plus `redb` for local caches.[2][21]
- Module inventory covers status, diff, blame, log/history (including JetBrains-derived graph layout and LinearBek linearization algorithms), changelists, shelf, patch, refs, remotes, ignore, external diff, index writes, preserve, and worktree roots.[19][21]
- UI offers commit/push/pull/stash/merge workflows, blame (also in diffs and as code-vision last-author lens), a Git Log tool window with graph, and a Branches dialog; open issues show active Git workflow improvements.[14][19][35]

**Implemented:** in-process reads + CLI writes, diff/blame/log/shelf/changelists.[21]
**Not verified:** performance numbers for Git operations (no published figures found beyond general budgets).[16]

---

## 7. Terminal implementation

- `ide-terminal`: "Integrated terminal sessions over a native PTY, or over a `TerminalChannel` such as an SSH shell. Rendering is xterm.js." Uses `portable-pty` with `native_pty_system`, `CommandBuilder`, resize (`PtySize`) and `ChildKiller`.[22][2]
- Output engineering: 4 ms batching or 64 KiB chunks, 250 ms post-exit drain, and a 16 MiB high-water byte budget that applies kernel pipe backpressure so a slow UI cannot accumulate unbounded output.[22]
- UTF-8 streaming decoder for split multibyte sequences; remote terminal sessions via SSH and remote projects.[22][2]
- Frontend terminal is `@xterm/xterm` 6.1.0-beta with fit/search addons.[3]

**Implemented:** native PTY backend + xterm.js frontend + SSH channel, backpressure.[22]
**Limitation:** the terminal renderer is JS/xterm.js in the webview — not a native Rust terminal widget.[3][22]

---

## 8. Database and HTTP tooling

**Database (`ide-db`):** data sources, connections, introspection, query execution, table-data paging/editing and statement splitting.[23]

- Drivers: `sqlx` from a pinned l0ng-ai fork (SQLite, PostgreSQL, MySQL/MariaDB; pinned for RUSTSEC-2026-0459), `tiberius` (SQL Server), `oracledb` 26 beta (pure-Rust thin Oracle), plus ClickHouse, Snowflake (account/browser/keypair auth) and JDBC driver support.[2][23]
- Also: SSH tunnels via `ide-ssh`, SSL modes, OS credential stores (`keyring`), catalog introspection, explain plans, query history, watch of sources, and an editable result grid (added in v0.1.39).[2][20][23]

**HTTP (`ide-http`):** IntelliJ-style `.http` request files: parser, `{{variable}}` environments from `http-client(.private).env.json`, cancellable/timed execution via `reqwest` + rustls + platform verifier, system proxy support, curl export, and response history.[24][2]

**Implemented:** both toolsets have real Rust backends and dedicated tool windows.[23][24]
**Not verified:** breadth of SQL dialect coverage in practice; no third-party benchmark found.

---

## 9. Plugin architecture

- Code plugins are **WebAssembly components (WASI preview 2) running in wasmtime 49.0.4**, with the component model and Cranelift; `wasmtime-wasi` p2 feature. The WIT contract is `openidea:plugin@1.7.0`; a typed Rust SDK (`sdk/rust/openidea-plugin`) provides the `Plugin` trait, `register_plugin!` and a mock host.[15][25][30]
- Packaging/contributions/grants live in `ide-extension`; runtime in `ide-plugin` (one wasmtime `Store` per plugin, one actor thread per active plugin); host implementation in `ide-host::plugins`, shared by desktop and `openidea-server`.[15]
- Sandbox: no preopened directories, env vars, network or inherited stdio; only WASI clocks and randomness; stderr captured. All FS/network/process/secrets access goes through gated host functions, with real-path checks to stop symlink escapes. Web views run in `<iframe sandbox="allow-scripts">` on an opaque `openidea-plugin:` origin.[15][30]
- Limits: ~5·10⁹ fuel instructions/call, 5 s wall clock for commands/events, 60 s for calls that open tasks, editor-provider results late at 50 ms dropped / interrupted at 500 ms, 256 MiB linear memory, idle reclaim after 5 min, crash backoff and "Failed" state after three crashes, per-plugin stats (p95, crashes, late results).[15]
- Official plugins ship in-tree (AI completion, Git toolbox, Checkstyle, CSV editor, GraphQL/Graphviz/Solidity languages, themes/material icons, test runner, etc.). Themes, icon themes and languages can be declarative with no code; the plugin API is described as "still a preview".[1][4][15]
- Remote projects: plugins execute in `openidea-server` next to the files; secrets stay in the desktop keychain.[15]

**Implemented:** wasmtime sandbox, capability grants, WIT 1.7 API, SDK, official plugins, remote execution.[15][25][30]
**Explicitly planned / not yet:** WASI 0.3 async components for API 2.0 (1.x stays synchronous via an adapter); project plugins (`.openidea/plugins`) do not run in remote projects yet; plugin storage on the remote server starts empty.[15]
**Architectural caveat for Devdeck:** the plugin runtime itself is Rust and reusable in principle, but plugin *UI* is web-based (iframe) and the host assumes the `ide-core`/`ide-host` ecosystem.[15][30]

---

## 10. Performance characteristics

- **Targets, not measurements:** `perf/budgets.json` defines upper bounds (e.g. initial JS 850 KiB, initial CSS 60 KiB, startup ready 400 ms / TBT 50 ms / heap 30 MB, typing p95 20 ms and max 33 ms, terminal TUI dropped frames 2 %, git log show 300 ms, leak heap growth 4 MB) with a 20 % regression tolerance. These are enforced budgets in `pnpm perf`, not observed results.[16]
- **Measurement tooling exists:** `pnpm perf` measures inside the webview; `pnpm perf:rust` runs Criterion benches (8 bench files across `ide-text`, `ide-index`, `ide-language`, `ide-search`, `ide-lsp`, `ide-host`, `ide-heap`); `perf/compare/bench.py` measures any IDE from the outside (window time, CPU-quiet "ready", RSS/USS/PSS over the whole process tree, CPU time, idle CPU, exit, caches), interleaving apps and rotating order with warmups and full attempt records.[16][17][2]
- **Published vendor claim (not independently reproduced here):** README reports opening freqtrade (~770 Python files) with warm caches, median of 5 runs: window 0.9 s vs IDEA 3.1 s; ready 1.1 s vs 13.2 s; memory (USS, all processes) 169 MB vs 2,011 MB; CPU time to ready 2.4 s vs 47.9 s; idle CPU 4 % vs 7 %; quit 0.3 s vs 5.9 s — on Windows 11, 32-core AMD, 62 GB RAM. The README itself notes IDEA analyses Python more deeply, so "ready" is not equivalent work, and points to `perf/compare` for reproduction.[4][17]
- **Design choices backing performance:** rope documents with versioned incremental edits, lazy/incremental tree-sitter parsing, in-process gitoxide reads, rayon/tokio parallelism, output batching and backpressure in the terminal, dev-profile dependency optimization with line-tables-only debug info, thin LTO release builds.[2][21][22]
- **Cost of the webview:** the benchmark's USS explicitly includes WebView2/WebKit helper processes; memory advantage is real but the app still depends on the platform webview and a large JS bundle (budget 850 KiB initial JS).[16][17]

**Not verified:** no local build, perf run or benchmark reproduction was performed; figures above are project-published claims and targets only.[4][16]

---

## 11. Reusable components for Devdeck

All internal crates are workspace path dependencies under the Apache-2.0 workspace license; no separate license per crate. Publication to crates.io could not be verified (crates.io API returned HTTP 403 from this environment), so treat them as source-reuse candidates, not drop-in published dependencies.[2][26]

| Component | What it gives Devdeck | Reuse notes |
|---|---|---|
| `ide-text` | Rope document model, UTF-16 coords, versioned changes, conflict detection | Strong candidate; coupled to `ide-core` types[31] |
| `ide-lsp` | Full LSP client: lifecycle, stdio/TCP, sync, requests, multi-server merge | Strong candidate; needs `ide-jsonrpc`, `ide-language`, `ide-vfs`, `ide-core`[11] |
| `ide-jsonrpc` | Content-Length JSON-RPC framing, cancellation, peer correlation | Nearly self-contained; shared by LSP/DAP[28] |
| `ide-dap` | DAP client, breakpoints, sessions, JVM deep inspection | Strong candidate; needs `ide-core`, `ide-language`, JVM crates[12] |
| `ide-terminal` | PTY sessions, batching/backpressure, SSH channels | Backend reusable; UI is xterm.js[22] |
| `ide-vcs` | gitoxide reads + git CLI writes, diff/blame/log/shelf | Backend reusable; needs `ide-core`, `redb`[21] |
| `ide-db` / `ide-http` | DB drivers/introspection/grid queries; `.http` client | Large dependency trees (sqlx fork, tiberius, Oracle, reqwest)[23][24] |
| `ide-plugin` / `ide-extension` / `ide-sdk` | wasmtime component sandbox, grants, WIT SDK | Reusable architecture; webview iframe UI not reusable[15][25][30] |
| `ide-index` / `ide-search` / `ide-syntax` / `ide-analysis` | Project index, search, tree-sitter fallback, structural search | Backend only; Criterion benches included[2][32] |
| `perf/` harness | Budgets + in-webview and external IDE comparison tooling | Methodology useful even if not reusing code[16][17] |

**Extraction caveat:** the crates are designed as one workspace with heavy cross-dependencies on `ide-core`/`ide-host` and Tauri adapters generated by `tauri-specta`; lifting any one crate means lifting a cluster, and all commands/events are typed for that IPC surface.[2][9]

---

## 12. Architectural limitations

1. **Not a native Rust UI** — Tauri 2 + React 19 + CodeMirror 6 + xterm.js; a 100%-native-Rust-UI requirement is not met.[8][9][10]
2. **Webview fragmentation** — separate platform code for WebKit2GTK, WKWebView and WebView2, including Windows webview key/recovery workarounds.[9]
3. **External toolchains required** — language servers, debuggers, JDKs, formatters are spawned as external processes (with one-click installers); a user without those tools gets reduced functionality.[14][13][27]
4. **Large surface area / coupling** — 47 crates, ~1,000 Rust files, ~2,800 TS files; internal path dependencies and generated IPC bindings make partial reuse non-trivial.[2][3][9]
5. **Rapid API churn** — plugin API advanced from 1.1 to 1.7 (and 1.15 declarative skills in a recent commit) within weeks; API 2.0 is planned.[15][1][20]
6. **Plugin UI is web-based** — sandboxed iframes with `postMessage`; no native plugin widget path.[15]
7. **Benchmark claims need reproduction** — vendor-published numbers with acknowledged apples-to-oranges caveats about "ready"; no independently verified results.[4][17]
8. **Young project / roadmap risk** — v0.1.x, ~1 month old, 154 open issues; not a stable dependency for a product requiring long-term API stability.[5][6][35]

---

## 13. License and redistribution risk

- Root license is **Apache License 2.0**; workspace, `src-tauri` and every crate declare `license = "Apache-2.0"`, and `package.json` also declares Apache-2.0.[2][26]
- `NOTICE` explicitly attributes Apache-2.0 software and icons from JetBrains' IntelliJ Platform (Copyright 2000-2026 JetBrains s.r.o.), MIT-licensed Simplified Chinese terms from Microsoft's VS Code language pack, and states OpenIDEA is **not affiliated with, endorsed by or sponsored by JetBrains**.[18]
- `THIRD_PARTY_LICENSES.md` documents unmodified JetBrains expui icons, JetBrains-derived diff/graph algorithms, Simple Icons (CC0), patched Lombok (MIT), dictionaries and more; redistribution must carry these notices and licence texts.[19]
- One **MPL-2.0** runtime dependency exists: `spellbook` 0.4 in `ide-spell` (weak file-level copyleft; unmodified crates.io use normally requires preserving notices and making any modifications available).[33][19]
- Trademark: IntelliJ IDEA is a JetBrains trademark; the project uses "inspired by IntelliJ IDEA" phrasing plus disclaimers in `NOTICE` and bundle metadata — Devdeck must not inherit that branding or imply affiliation.[8][18][29]

**Risk assessment: low–moderate.** Apache-2.0 permits commercial use, modification and redistribution with attribution and a patent grant; obligations are to preserve LICENSE/NOTICE/THIRD_PARTY_LICENSES, honour MPL-2.0 for `spellbook`, and avoid JetBrains trademark confusion. No GPL/AGPL/BSL/SSPL/proprietary notices were found in `THIRD_PARTY_LICENSES.md`.[19][26][33]

---

## 14. Implemented vs planned (explicit labels)

| Area | Status | Basis |
|---|---|---|
| Tauri shell, auto-update, native OS menus/window management | **Implemented** | [8][9] |
| React/CodeMirror editor with IntelliJ keymap, multi-caret, folding, Vim | **Implemented** (web UI) | [3][14][34] |
| LSP client, multi-server merge, 86 language configs | **Implemented** | [11][14][27] |
| DAP client, 15+ documented language adapters, advanced breakpoints | **Implemented** | [12][13] |
| Git via gitoxide + git CLI, blame/diff/log/shelf | **Implemented** | [21] |
| Native PTY terminal + SSH + xterm.js | **Implemented** | [22][3] |
| DB clients (SQLite/PG/MySQL/MSSQL/Oracle/ClickHouse/Snowflake/JDBC) | **Implemented** | [23] |
| `.http` client with env vars, proxy, history, curl export | **Implemented** | [24] |
| wasmtime plugin sandbox, WIT 1.7 API, official plugins | **Implemented (API preview)** | [15][25][30] |
| Remote projects via `openidea-server`, MCP, agents | **Implemented** | [9][15][20] |
| Perf harness, budgets, external comparison | **Implemented** | [16][17] |
| Plugin API 2.0 with WASI 0.3 async components | **Planned** | [15] |
| Project plugins (`.openidea/plugins`) in remote projects; persistent remote plugin storage | **Planned / not yet** | [15] |
| Native Rust UI widgets | **Not implemented / not a project goal** | [8][9] |
| Independent reproduction of published benchmarks | **Not verified in this research** | [4][17] |

---

## 15. Recommendation for Devdeck

Track OpenIDEA as a **backend architecture reference and possible Apache-2.0 component source**, not as a UI foundation. Its strongest reusable ideas are the LSP/DAP clients on a shared JSON-RPC core, rope-backed versioned documents with tree-sitter fallback, gitoxide-read/git-CLI-write VCS design, PTY backpressure, and the wasmtime capability sandbox. Any code reuse requires extracting clusters of `ide-*` crates around `ide-core`, and replacing all webview/React/CodeMirror/xterm.js surfaces with native Rust widgets.[2][11][12]

---

## Sources

[1] https://github.com/l0ng-ai/openidea — OpenIDEA GitHub repository
[2] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/Cargo.toml — OpenIDEA workspace Cargo.toml at c209e18
[3] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/package.json — OpenIDEA package.json at c209e18
[4] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/README.md — OpenIDEA README at c209e18
[5] https://api.github.com/repos/l0ng-ai/openidea — GitHub API repository metadata for l0ng-ai/openidea
[6] https://api.github.com/repos/l0ng-ai/openidea/releases?per_page=10 — GitHub API recent releases for openidea
[7] https://api.github.com/repos/l0ng-ai/openidea/issues?state=open&per_page=100 — GitHub API open issues for openidea
[8] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/src-tauri/tauri.conf.json — Tauri configuration at c209e18
[9] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/src-tauri/Cargo.toml — Tauri app Cargo.toml at c209e18
[10] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/index.html — Frontend index.html at c209e18
[11] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/crates/ide-lsp/src/lib.rs — ide-lsp/src/lib.rs
[12] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/crates/ide-dap/src/lib.rs — ide-dap/src/lib.rs
[13] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/docs/en/guide/debugging.mdx — Debugging guide
[14] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/docs/en/guide/code-intelligence.mdx — Code intelligence guide
[15] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/docs/en/plugins/architecture.mdx — Plugin architecture docs
[16] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/perf/budgets.json — Performance budgets
[17] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/perf/compare/README.md — External IDE comparison harness README
[18] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/NOTICE — NOTICE file
[19] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/THIRD_PARTY_LICENSES.md — Third-party licenses
[20] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/docs/en/changelog.mdx — Changelog
[21] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/crates/ide-vcs/src/lib.rs — ide-vcs lib.rs
[22] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/crates/ide-terminal/src/lib.rs — ide-terminal lib.rs
[23] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/crates/ide-db/src/lib.rs — ide-db lib.rs
[24] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/crates/ide-http/src/lib.rs — ide-http lib.rs
[25] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/crates/ide-plugin/Cargo.toml — ide-plugin Cargo.toml
[26] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/LICENSE — Apache-2.0 LICENSE
[27] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/languages/python.toml — Python language definition (LSP + debugger config)
[28] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/crates/ide-jsonrpc/src/lib.rs — ide-jsonrpc transport shared by LSP and DAP
[29] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/docs/en/introduction.mdx — Introduction: native app with a Rust core
[30] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/crates/ide-plugin/src/lib.rs — ide-plugin runtime lib.rs
[31] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/crates/ide-text/src/lib.rs — ide-text rope document model
[32] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/crates/ide-syntax/src/lib.rs — ide-syntax tree-sitter structure service
[33] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/crates/ide-spell/Cargo.toml — ide-spell Cargo.toml (MPL-2.0 dependency)
[34] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/src/editor/core/cm/setup.ts — CodeMirror setup in frontend
[35] https://api.github.com/search/issues?q=repo%3Al0ng-ai%2Fopenidea+is%3Aissue+is%3Aopen — GitHub search: exact open issue count
[36] https://github.com/l0ng-ai/openidea/commits/main — OpenIDEA main branch commit history
