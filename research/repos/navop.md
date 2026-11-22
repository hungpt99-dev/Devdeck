# Navop — Evidence-Based Repository Research Report

**Target:** `https://github.com/Alno0/navop` (mission "Repo C")
**Snapshot analyzed:** local clone at commit `78dba5c31309aa991dab8897c090189b939227e4`, dated 2026-09-12, branch `dev`.
**Method:** `git clone` + direct inspection of source, manifests, module trees, license files, git history, and GitHub REST/ Actions/issue APIs. README claims were treated as hypotheses and checked against code.

---

## 1. Executive summary

Navop is a large, actively maintained, GPUI-based Rust desktop application that genuinely implements most of the broad feature set advertised in its README: multi-database GUI, SSH/SFTP, local/SSH/Telnet/serial terminals, remote desktop, server monitoring, AI chat/agents, an MCP server, and a multi-mechanism extension system. The codebase is production-scale (1,906 Rust files across 59 workspace crates, 1,569 locked packages, ~7,543 test functions, only 13 `TODO`/`FIXME`/`todo!` occurrences by local count) [7][21]. It is not, however, a clean reference design to copy wholesale: it depends on self-maintained forks of GPUI and its component kit, embeds a WebView despite a "No WebView" marketing claim, and is governed by a custom supplementary license that explicitly forbids creating competing products — a direct concern for a Devdeck-style all-in-one developer desktop app [6][7][20].

**Verdict for Devdeck:** strong *architectural reference* (crate boundaries, wire protocols, permission model, typed value model, terminal observability); **not license-safe as a source to reuse**, and several "native-only" claims need qualification.

---

## 2. Repository identity and snapshot caveat

The requested URL is **not the canonical project**. `Alno0/navop` was created 2026-09-11, has 0 stars, 0 forks, 0 open issues, and was last pushed 2026-09-11 — it is a mirror [1][2]. The README, CI badges, issue links, and release links all point at `feigeCode/navop`, which has 2,204 stars, 219 forks, 64 open issues, was created 2026-07-10, and was pushed 2026-11-22 [3][5]. The canonical repo publishes frequent releases (v0.23.4 on 2026-11-18, preceded by v0.23.3/v0.23.2/v0.23.1/v0.23.0 and multiple betas) [4].

This report's code evidence comes from the `Alno0` mirror's `dev` snapshot at `78dba5c` (2026-09-12). Its CHANGELOG stops at v0.17.0 (2026-09-08) [36], so it is roughly two months and six minor releases behind upstream [4]. Upstream commits after the snapshot include removal of the built-in X11 component in favour of an external X server (`b2cb33d0`, 2026-11-22) [22]. Any statement below describes the **cloned snapshot**, not necessarily current upstream [22][36]. The `navop-extensions` submodule declared in `.gitmodules` was empty in the clone, so first-party driver/provider extensions could not be inspected directly.

---

## 3. Native GPUI implementation

The application entry point `main/src/main.rs` is a genuine GPUI app: `use gpui::*`, `gpui_component::{DialogStateChanged, Root}`, `gpui_component_assets::Assets`, and an `AppAssets` struct combining built-in assets, Navop icons, and a database-driver asset source; the root entity is `OnetCliApp` (a legacy name retained from the OnetCli migration) [8][25]. Window sizing uses GPUI `Size<Pixels>` with a 0.9 display ratio and 1800×1260 defaults [8]. Startup installs global window/hotkey systems through `gpui::App`/`Window` handles in `main/src/app_init.rs` — a locally verified file not separately registered — [8]. Roughly 571 Rust source files in the workspace reference `gpui::` directly (local grep count).

The GPU stack is **forked**, not vanilla: `[workspace.dependencies]` declares `gpui = gpui-pre 0.3.1` and `gpui-platform = gpui-pre-platform 0.3.1` with `font-kit`, `x11`, `wayland`, and `runtime_shaders` features, while `[patch.crates-io]` redirects ~22 `gpui-pre-*` crates to the authors' `feigeCode/gpui-pre` fork at tag `fork-0.3.99`; `gpui-component`, `gpui-base`, `gpui-shell`, `gpui-wry`, and `gpui-kit` are pinned to a single `feigeCode/gpui-component` git revision `464fa3bd…`; `rquickjs` is patched to the same revision to unify QuickJS types [7]. The fork is explicitly self-maintained via scripts (`script/publish-gpui-pre-fork.py`, `script/migrate-to-git-fork.py`) [7]. That gives Devdeck a usable pattern (own the UI toolkit fork, pin by revision) but also a supply-chain and long-term maintenance liability [7].

**Claim check — "No WebView" is not literally true.** The README markets "GPU-accelerated rendering · No WebView" [5], but `main/Cargo.toml` has `default = ["wasm-components", "embedded-webview", "windows-native-rdp"]`, and `crates/ai_chat_view/src/html_code_block.rs` uses `wry::WebViewBuilder` plus `gpui_wry::WebView` to render HTML code blocks inside `gpui_component` views [19][20]. The rest of the UI is native GPUI; the exception is scoped to AI-produced HTML [20].

---

## 4. Database architecture

Navop uses a **hybrid in-process + out-of-process driver architecture**. The `db` crate declares built-in modules for ClickHouse, MSSQL, MySQL, PostgreSQL, and SQLite, with DuckDB behind the `builtin-duckdb` feature, plus `ipc`, `plugin`, and `plugin_manifest` modules for external drivers [9]. Its manifest lists direct driver crates `rusqlite`, `mysql_async`/`mysql_common`, `tokio-postgres`/`tokio-postgres-rustls`, `tiberius`, `oracle`, `clickhouse`, and optional `duckdb` [33].

The out-of-process path is a formal protocol: `crates/extension-protocol/src/lib.rs` defines JSON-RPC 2.0 messages, 4-byte little-endian length-prefixed framing (MessagePack, or JSON for debug), and namespace methods (`conn/*`, `schema/*`, `query/*`, `sql/*`, `ddl/*`, `data/*`, `host/*`, `event/*`) with `WIRE_PROTOCOL_VERSION = "1.0"` [17]. `crates/db/src/ipc/protocol.rs` translates business config into wire JSON, injects `schema/*` calls as pseudo-SQL using the prefix `/*onetcli-ipc-wire*/`, and documents that the **host** establishes SSH/TCP tunnels before rewriting `conn/open` host/port to the local mapped address — the driver subprocess never handles tunneling itself [10]. This is a clean isolation boundary worth studying for Devdeck [10][17].

Values are normalized through a dedicated `db-value` crate: `DbValue` is a transport/UI/SDK-independent enum (Null, Bool, Integer/Unsigned/Float with width, Decimal, BitString, Text, LegacyText, Binary, Date/Time/DateTime, Json, Uuid, Duration, Unsupported-with-display), with `CellState::Decoded`/`Undecoded` and raw-payload provenance [31]. Recent commits in the snapshot are a typed-value-model refactor with real PostgreSQL/MySQL conformance tests [22]. The UI layer `db_view` adds table designer, SQL editor with inline completion/hover/signature, execution history, ER diagram, schema/data compare, import/export, and `AskAiButton` integration [32][9].

Broader claims need qualification: TDengine was moved to an external IPC driver as of v0.16, and Dameng DM, KingbaseES, GBase 8s, OceanBase, openGauss, IoTDB, and Oscar are advertised as installable extensions [5]. The corresponding code lives in the empty `navop-extensions` submodule, so those drivers are **not verifiable in this snapshot** [5]. The README also names MQTT as an extension (MQTT 3.1.1 over rustls, SSH tunneling, auto-resubscribe); likewise unverified here [5].

---

## 5. SSH and SFTP

`crates/ssh/src/lib.rs` exports a russh-based client (`RusshClient`, `RusshChannel`, `SshClient`, `SshAuth`), session manager/registry with leases, host-key policy and known-host types, jump-server and proxy connect configs, keyboard-interactive prompts/responders, local/remote/dynamic SOCKS port-forwarding tunnels, and X11 `ForwardRequest`/`X11Proxy` [11]. The crate manifest confirms `russh`, `tokio`, `hmac`, `sha1`, `tokio-socks`, and `x11_forwarding` as dependencies. Legacy SSH algorithm opt-in exists (`LegacyAlgorithmRequired`, `build_client_preferred_algorithms_with_legacy`) [11].

SFTP is split into protocol operations, UI, and transfer engine [26][35]. `crates/sftp/src/lib.rs` re-exports `RusshSftpClient`, file operations, remote-file commands, and server-to-server copy with direct/relay strategies and explicit approval types; `FileEntry` carries permissions and owner resolution with numeric UID fallback [26]. `crates/sftp_transfer/src/lib.rs` implements a queued transfer engine with scheduler, executor, conflict resolver, progress, history, and cancellation, backed by `RusshSftpTransferProvider` [35]. The UI crate adds dual-pane lists, favorites, remote editing, drag-and-drop, and host-key prompts (verified by file tree, not separately cited). ZMODEM is a separate module in the terminal crate [12].

---

## 6. Remote infrastructure management

Beyond SSH tunneling, the workspace has dedicated `connection_tunnel`, `port_forwarding`, `port_forwarding_view`, `connection_tunnel`, and `x11_forwarding` crates (local/remote/dynamic forwarding and Xauthority/cookie/display handling by file tree) [7]. Connection management, encrypted credential storage, and cloud sync live in `one_core` [25].

Remote desktop is more nuanced than the README suggests [5]. `crates/remote_desktop/src/backends/rdp.rs` implements `RdpBackend` around a **`HelperProcessConfig`**: it spawns an external helper binary and exchanges framed requests over stdin/stdout (`crates/remote_desktop/src/backends/rdp/transport.rs`) [24]. There is **no `ironrdp` crate in `Cargo.lock`** in this snapshot; IronRDP appears only in test fixture strings and form labels, implying the actual RDP engine is shipped as an external helper/provider rather than linked into the app [24]. Providers are discovered from `extensions/remote_desktop_providers/` manifests via a global, refreshable registry [crates/remote_desktop/src/provider_registry.rs — locally inspected]. On Windows, `crates/windows_rdp_host` contains a C++ host (`native/host.cpp`, `event_sink.cpp`, `credential.cpp`, policy files) embedding the Microsoft MSTSC ActiveX control, with a marker `windows-native-rdp` feature enabled by default [19][24]. The companion plan document marked the native-RDP work "进行中" (in progress) at the time of the snapshot, so this subsystem should be considered platform-sensitive [37].

---

## 7. Terminal integration

`crates/terminal/src/lib.rs` reveals a full terminal stack: local shell/PTY backend, SSH backend, Telnet backend, serial backend, shell integration (OSC), session logging, recording and replay, WSL distribution discovery, ZMODEM, ingress queue, encoding, and selection text [12]. The `terminal_view` crate (local file tree) adds draggable split panes, tab drag/transfer, vi input, block selection, paste safety, searchable history, command bar, credential capture, background tasks, and recording playback controls [12]. Session logs with incremental loading and batch delete were added in v0.17.0 [36]. A recent commit adds WSL-distro auto-detection and one-click launch per distro [22].

Performance instrumentation is unusually explicit: `crates/terminal/src/performance_metrics.rs` provides payload-free, lock-free atomic counters (ingress bytes, pending bytes, parser chunks, lock wait/hold times, wakeup coalescing, render timing, SSH reconnect counts), gated by `NAVOP_TERMINAL_PERFORMANCE_METRICS`, with a documented warning that snapshots are best-effort and not transactionally consistent [28]. This is a directly transferable design for Devdeck's terminal [28].

---

## 8. Monitoring

Server monitoring is implemented in `crates/terminal_view/src/sidebar/server_monitor_panel.rs` [13]. It runs an embedded bash script over an existing SSH channel that reads `/proc/stat`, `/proc/cpuinfo`, `/proc/meminfo`, `/proc/uptime`, `/dev/net` and `ps`, emitting a small YAML-ish payload (OS, CPU, memory/swap, disks, network totals/rates, process counts, top-5 CPU/memory processes) [13]. The panel polls every 3 seconds (`REFRESH_INTERVAL_SECS = 3`), keeps 30 history samples, and renders `AreaChart`/`LineChart`/`PieChart` from `gpui_component` [13]. Crucially, the script self-reports `supported=false` with reason "Current host is not Linux or /proc is unavailable" when `uname != Linux` or `/proc/stat` is unreadable — so monitoring is **Linux-only**, not a general cross-platform server monitor [13]. The extension host also exposes a separate `monitor` host module for shell plugins (local file tree), and the app's own terminal performance counters are distinct from remote host monitoring [28].

---

## 9. AI agent integration

AI is implemented in layers rather than a single chat widget [14][16][15].

- **LLM transport:** `llm-connector` supports five protocols — OpenAI, Anthropic, Aliyun (DashScope), Zhipu, and Ollama — with a `LlmClient` builder, SSE streaming, and request/response type separation [crates/llm-connector/src/lib.rs — locally inspected].
- **Agent runtime:** `crates/agent_runtime/src/lib.rs` describes a codex-style model-driven loop: streamed sampling → no tool call means final answer → tool calls dispatched through `ToolRouter` / `update_plan` → observation appended to session history → follow-up sampling. It is deliberately render-free and decoupled from GPUI and resource crates, with planner, skills, sub-agent/delegate tasks, risk levels, and context compaction [14].
- **Tool permissions:** `crates/tool_runtime/src/permission.rs` defines four profiles (`Safe`, `Confirm`, `Auto`, `Unrestricted`) and per-operation policies (`Allow`/`Ask`/`Deny`) with read/write/high-risk splits and per-tool/per-resource overrides [16]. This is a strong, reusable security pattern [16].
- **External agents (ACP):** `main/src/ai_chat_acp.rs` loads ACP agent entries from an extension registry, supports HTTP and stdio transports, and merges user overrides from config/env [main/src/ai_chat_acp.rs — locally inspected]. The README names Codex, Claude Code, and OpenCode as supported ACP agents [5].
- **Public MCP:** `crates/public_mcp/src/server.rs` binds a `TcpListener` to `127.0.0.1:0` (ephemeral port), passes a per-client token, and spawns a task per connection using `rmcp`; discovery is a user-scoped `public-mcp.json` file with PID, port, token, and mode [15]. Tool groups include terminal read/write/exec, Redis, MongoDB, and remote ops (by file tree). The README documents Safe/Confirm/Auto profiles and tool-exposure selection [5].
- **Charts:** the README's "charts" capability is narrower than it sounds: AI output in `chart-json` code fences is parsed and rendered as lightweight bar/list/pie cards; interactive charts can be overridden by business modules but are not built in [27].

---

## 10. UI patterns

The UI uses `gpui_component::{Root, Assets}` with an asset bundle, global `App` state via `cx.set_global`, declarative dialogs (`window.open_dialog`), and window/hotkey initialization through `OnceLock` globals [8]. Themes are data-driven: a 32 KB `.theme-schema.json`, light/dark/system modes, importable themes, accent colors, and window opacity are advertised [5]. Internationalization uses `rust_i18n::i18n!` with `fallback = "en"`, with English, Simplified Chinese, and Traditional Chinese locales [5][9]. A declarative connection-form engine unifies database/SSH/middleware forms with composite keychain-auth fields [36]. The home page uses a unified grid, tree view, recent connections, and persisted workspace drag ordering [36]. Reusable `one_ui` and `one_assets` crates centralize icon buttons and SVG assets (file tree) [7].

---

## 11. Plugin and extension architecture

Navop implements **four distinct extension mechanisms**, which is the most architecturally interesting part of the repo for Devdeck:

1. **Framed JSON-RPC subprocess/WASM drivers** — `extension-protocol` (JSON-RPC 2.0 + 4-byte LE length prefix + namespaces) [17]; `extension-host` provides `process.rs`, `process_session.rs`, `transport.rs`, `negotiation.rs`, `manifest.rs`, and `host_api.rs` (file tree).
2. **WASM components** — `crates/extension-wasm/Cargo.toml` pins `wasmtime = "=36.0.14"` and `wasmtime-wasi`, with component runtimes for connection import, document render/export, and HTML preview transforms [18]. `crates/extension-runtime/src/lib.rs` wires catalogs, install flows, language bundles, database-driver install, remote-desktop providers, and a DB gateway [34].
3. **Shell plugins / universal plugins** — `crates/universal-plugins/src/shell_plugin_host/` exposes host modules for connection, resource, workbench, components, grant, policy, blob, monitor, job, and value to plugins (file tree).
4. **ACP agents** — external agent executables registered as extensions and driven over stdio/HTTP [main/src/ai_chat_acp.rs].

Marketplace installation is functional: entries resolve per-platform artifacts with `sha256` metadata, and non-language extensions are rejected if no checksum is present [30]. Extension kinds include database driver, remote desktop provider, connection importer, document renderer/exporter, language bundle, ACP agent, and shell tool [34][17]. A `navop-extensions` git submodule holds first-party extensions but was empty in this clone [1].

---

## 12. Performance and maintainability

Positive signals: extremely high test density (~7,543 `#[test]`/`#[tokio::test]` occurrences by local count), workspace-wide lint configuration, a `cargo fmt --check` / `clippy --workspace --all-targets` / `cargo test --all` workflow documented in the README, and recent CI runs mostly green (success at 09:29 and 09:12 UTC on 2026-11-22, with one failure and two cancellations during an active release cycle) [5][21]. Only 13 `TODO`/`FIXME`/`todo!` markers were found locally [7]. Crate boundaries are fine-grained (59 members) and several crates are explicitly UI-free for testability (`agent_runtime`, `db-value`, `ssh`, `sftp_transfer`) [14][31][11].

Costs: 1,906 Rust files and 1,569 locked packages make the build and audit surface very large [7]. `AGENTS.md` (85 KB) and `CHANGELOG.md` (81 KB) indicate heavy process overhead. Self-maintained git forks of GPUI, gpui-component, and QuickJS mean upstream fixes must be manually rebased [7]. Historical `onetcli` naming persists in persisted IDs, crate names (`onetcli_runtime`), and env vars (`ONETCLI_REMOTE_DESKTOP_PROVIDER_DIR`), which the repo's own guide flags as compatibility-sensitive [25].

---

## 13. License and third-party dependency risk

**This is the largest blocker for Devdeck.** Navop's source is Apache-2.0 *plus* a custom "Navop Supplementary License" [5][6]. The supplementary license permits personal and commercial *use* and free redistribution only through free channels, but Section 4 imposes: (a) no sale/resale/fee-based distribution, (b) no redistribution as a component of a commercial product/suite/platform, (c) no hosting on paid distribution platforms, and — most importantly — **(d) "You may not create competing products or services based on the Software's code, in whole or in part"** [6]. It is revocable, non-transferable, non-sublicensable, governed by PRC law, and disputes go to Haidian District People's Court, Beijing [6]. GitHub classifies the repo license as `NOASSERTION`/Other [2][3]. Since Devdeck is explicitly an all-in-one native Rust developer desktop app covering overlapping categories, any copying of Navop source could be asserted as a competing product [6]. **Recommendation: study architecture and protocols conceptually, do not copy code, and keep a clean-room design record** [6].

Third-party risk is also material: ~22 `gpui-pre-*` crates and `rquickjs` are patched to author-controlled forks; `gpui-component` is pinned to one git revision; `reqwest`, `async-process`, and `async-task` come from zed-industries git repos; `psm` tracks a branch of `rust-lang/stacker`; `wasmtime` is hard-pinned to `=36.0.14` [7][18]. The vendored `ferrum-flow` (ER diagram rendering) is Apache-2.0 [vendor/ferrum-flow/LICENSE — locally inspected]. There is a Pro licensing subsystem (`CloudSync`, `TeamManagement` gated features) whose offline public key is an empty string in source, meaning offline license verification is effectively disabled in this snapshot [23].

---

## 14. Maturity

The canonical project is mature by activity metrics: 2,204 stars, 219 forks, 64 open issues, continuous releases through v0.23.4 (2026-11-18), and commits on 2026-11-22 [3][4][22]. Open issues include real bugs (SFTP keyboard-interactive with 2FA, AI duplicate `Content-Type` headers, history command blanks) and feature requests (SSH key generation, LLM OpenAI-compatible provider issues), indicating an active user base rather than a dormant repo [29]. The CHANGELOG shows detailed bilingual release notes and hundreds of fixes across UI, terminal, SQL editor, and cross-platform builds [36]. Against that, the requested `Alno0` mirror is essentially empty metadata and two months stale [2][36].

---

## 15. Implemented vs planned — explicit split

**Implemented and verified in the cloned source:**
- Native GPUI app shell with asset system, theming, i18n, dialogs, hotkeys [8][5].
- Built-in MySQL/PostgreSQL/SQLite/MSSQL/Oracle/ClickHouse (+ optional DuckDB) drivers, typed value model, SQL editor, ER diagram, schema/data compare [9][31][32].
- JSON-RPC framed IPC protocol for external drivers, with host-side SSH tunneling [17][10].
- russh SSH with host-key policy, jump/proxy, keyboard-interactive, local/remote/dynamic forwarding, X11 [11].
- SFTP with server-to-server copy and a queued transfer engine [26][35].
- Local/SSH/Telnet/serial terminals with recording, session logs, WSL, ZMODEM, performance counters [12][28].
- Linux-only SSH server monitoring with charts [13].
- Agent runtime, tool permission profiles, ACP agent registry, loopback MCP server with token discovery [14][16][15].
- WASM components (wasmtime 36), subprocess plugins, shell plugins, marketplace with SHA-256 checks [18][30][34].

**Advertised but not verifiable in this snapshot / qualified:**
- TDengine, Dameng DM, KingbaseES, GBase 8s, OceanBase, openGauss, IoTDB, Oscar, and MQTT as installable extensions — implementation lives in the empty `navop-extensions` submodule [5].
- "Pure-Rust IronRDP canvas backend" — no `ironrdp` crate in `Cargo.lock`; in-repo RDP code spawns an external helper [24].
- "No WebView" — default feature `embedded-webview` embeds Wry for AI HTML blocks [19][20].
- Native Windows MSTSC — real C++ host code exists, but its plan document was still marked in progress at snapshot time [37][19].
- "Charts" in AI — implemented only as lightweight `chart-json` cards [27].
- Upstream features after 2026-09-12 (e.g., external-X11-only policy) are outside this snapshot [22].

---

## 16. Reusable components for Devdeck (design patterns, not code)

- **Framed JSON-RPC extension protocol with typed namespaces and version constant** — copy the *pattern*, not the code, to isolate Devdeck drivers/plugins [17][10].
- **Transport-independent typed DB value model** (`DbValue` + `CellState` + raw provenance) to keep UI decoupled from SDK specifics [31].
- **Four-tier tool permission model** (Safe/Confirm/Auto/Unrestricted × read/write/high-risk × per-tool overrides) [16].
- **Loopback-only MCP with ephemeral port, per-client token, and user-scoped discovery file** [15].
- **Lock-free terminal observability counters** with explicit best-effort semantics [28].
- **Crate-boundary discipline**: agent runtime, value model, SSH, and transfer engine are UI-free and independently testable [14][31][11].
- **Marketplace checksum enforcement** (reject artifacts without SHA-256) [30].
- **Anti-pattern to avoid**: self-maintained GUI toolkit forks and a custom "no competing products" license [7][6].

---

## 17. Open questions

1. How does current upstream (v0.23.4/v0.24.0) differ from this 2026-09-12 snapshot, especially after the X11 component removal? [4][22]
2. What is actually inside `feigeCode/navop-extensions` — do the claimed MQTT/TDengine/IronRDP helper implementations exist and under which licenses? [5]
3. Does Navop's supplementary license's "competing products" clause apply to architectural ideas/protocol shapes, or only copied code? Legal review needed [6].
4. Are the self-maintained `gpui-pre`/`gpui-component` forks licensed permissively, and can they be consumed independently? [7]
5. How effective is the Pro license gate in practice given the empty offline public key [23]?
6. What is the failure mode of the Linux-only monitor on macOS/Windows hosts in release builds [13]?

## Sources

[1] https://github.com/Alno0/navop — Navop GitHub repository
[2] https://api.github.com/repos/Alno0/navop — GitHub API metadata for Alno0/navop mirror
[3] https://api.github.com/repos/feigeCode/navop — GitHub API metadata for feigeCode/navop upstream
[4] https://api.github.com/repos/feigeCode/navop/releases — GitHub releases for feigeCode/navop
[5] https://github.com/feigeCode/navop/blob/dev/README.md — Navop README (upstream)
[6] https://github.com/Alno0/navop/blob/78dba5c/NAVOP_LICENSE — Navop Supplementary License at cloned commit
[7] https://github.com/Alno0/navop/blob/78dba5c/Cargo.toml — Navop workspace Cargo.toml at cloned commit
[8] https://github.com/Alno0/navop/blob/78dba5c/main/src/main.rs — Navop GPUI app entry point (main.rs)
[9] https://github.com/Alno0/navop/blob/78dba5c/crates/db/src/lib.rs — db crate module tree
[10] https://github.com/Alno0/navop/blob/78dba5c/crates/db/src/ipc/protocol.rs — Database IPC/wire protocol helpers
[11] https://github.com/Alno0/navop/blob/78dba5c/crates/ssh/src/lib.rs — SSH crate exports and capabilities
[12] https://github.com/Alno0/navop/blob/78dba5c/crates/terminal/src/lib.rs — Terminal crate module tree
[13] https://github.com/Alno0/navop/blob/78dba5c/crates/terminal_view/src/sidebar/server_monitor_panel.rs — Server monitoring panel implementation
[14] https://github.com/Alno0/navop/blob/78dba5c/crates/agent_runtime/src/lib.rs — Agent runtime architecture
[15] https://github.com/Alno0/navop/blob/78dba5c/crates/public_mcp/src/server.rs — Public MCP loopback server
[16] https://github.com/Alno0/navop/blob/78dba5c/crates/tool_runtime/src/permission.rs — Tool permission profiles
[17] https://github.com/Alno0/navop/blob/78dba5c/crates/extension-protocol/src/lib.rs — Extension wire protocol
[18] https://github.com/Alno0/navop/blob/78dba5c/crates/extension-wasm/Cargo.toml — WASM extension runtime dependency
[19] https://github.com/Alno0/navop/blob/78dba5c/main/Cargo.toml — Main crate Cargo features
[20] https://github.com/Alno0/navop/blob/78dba5c/crates/ai_chat_view/src/html_code_block.rs — Embedded WebView for AI HTML blocks
[21] https://api.github.com/repos/feigeCode/navop/actions/runs — Recent CI workflow runs
[22] https://api.github.com/repos/feigeCode/navop/commits — Recent upstream commits
[23] https://github.com/Alno0/navop/blob/78dba5c/crates/core/src/license/models.rs — License-gated feature enum
[24] https://github.com/Alno0/navop/blob/78dba5c/crates/remote_desktop/src/backends/rdp/transport.rs — RDP helper process transport
[25] https://github.com/Alno0/navop/blob/78dba5c/CLAUDE.md — Repository architecture guide
[26] https://github.com/Alno0/navop/blob/78dba5c/crates/sftp/src/lib.rs — SFTP crate API
[27] https://github.com/Alno0/navop/blob/78dba5c/crates/ai_chat_view/src/cards.rs — AI chart-json card rendering
[28] https://github.com/Alno0/navop/blob/78dba5c/crates/terminal/src/performance_metrics.rs — Terminal performance counters
[29] https://api.github.com/search/issues?q=repo%3AfeigeCode%2Fnavop%2Btype%3Aissue%2Bstate%3Aopen — Open issues search
[30] https://github.com/Alno0/navop/blob/78dba5c/crates/extension-runtime/src/extension_downloader/transfer.rs — Extension download checksum verification
[31] https://github.com/Alno0/navop/blob/78dba5c/crates/db-value/src/lib.rs — Typed database value model
[32] https://github.com/Alno0/navop/blob/78dba5c/crates/db_view/src/lib.rs — Database view module tree
[33] https://github.com/Alno0/navop/blob/78dba5c/crates/db/Cargo.toml — Database crate dependencies
[34] https://github.com/Alno0/navop/blob/78dba5c/crates/extension-runtime/src/lib.rs — Extension runtime module tree
[35] https://github.com/Alno0/navop/blob/78dba5c/crates/sftp_transfer/src/lib.rs — SFTP transfer engine
[36] https://github.com/Alno0/navop/blob/78dba5c/CHANGELOG.md — Navop user-facing changelog
[37] https://github.com/Alno0/navop/blob/78dba5c/docs/superpowers/plans/2026-08-07-windows-native-rdp.md — Windows native RDP implementation plan
