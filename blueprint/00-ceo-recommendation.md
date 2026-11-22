# CEO recommendation — Devdeck blueprint

Date: 2026-11-22. Evidence base: 10 independent research reports in `research/` (3,527 lines, 635 unique sourced URLs consolidated into `research/.citations/master.json`). This is a research and design decision document; no product code was changed.

## Verdict

Build Devdeck as a **local-first, single-boundary developer workstation**, not as "a faster editor" or "a free IDE suite". The defensible wedge is one versioned config, one permission/approval boundary, one audit trail, and one workspace model across terminal, Git, editor, containers, databases, APIs, tools and agents — with best-of-breed engines integrated through adapters rather than reimplemented.

Native-Rust-only is feasible, but it is not white space: Zed, Helix, Lapce, GitUI, Nushell, mise and Dodo already ship native Rust tooling[135][150][37]. "Only native all-in-one" was killed by Dodo[37]. "Native means faster" was killed by the absence of independent benchmarks, so performance must be measured, not asserted.

## The five decisions

| Question | Decision | Why |
| --- | --- | --- |
| What to reuse | OpenIDEA's Apache-2.0 Rust backend crates as a selective reference; Dodo's UI/state patterns (not GPUI-linked code); Navop's protocol shapes as clean-room study only; standard crates for protocols/engines | OpenIDEA's shell is Tauri+React, not native Rust[1][3][8]; Dodo's MIT source has an unresolved GPL-3.0-or-later linked chain[40][41]; Navop's supplementary license forbids competing products[103] |
| What to build | Typed tool schema, permission policy, audit, config/migrations, workspace model, extension host, adapter orchestration, and the domain UIs that make each tool genuinely usable | These are the single boundary incumbents do not share[138][140][142] |
| What not to build | Webview/JS UI, package managers, container engines, DB servers, LSP/DAP servers, custom text shaper, deep clones of VS Code/JetBrains/Postman | Reuse mature engines; "replace everything" is not achievable in v1 and the kill list shows depth parity is not a credible claim |
| Which architecture | Iced 0.14.0 pinned + tokio; one trusted UI/core process, supervised extension-host process, one process per LSP/DAP/MCP sidecar and per terminal PTY; WASM components primary plugin runtime, JSON-RPC sidecars secondary | Iced scored 82.0 vs GPUI 75.4 and Slint 74.2; GPUI 0.2.2 is blocked by a transitive build break, Slint's license conflicts with MIT/Apache-only policy[302][304][345][346] |
| What is the MVP | Devdeck Foundation: shell + projects + terminal + Git essentials + minimal editor + YAML/JSON config editor + settings + read-only tool detection + ONE consent-gated install slice via mise; LSP, DAP, Docker, DB, API, agents deferred behind named gates | Smallest loop that proves one native workspace is better than three apps, while proving the tool-manager differentiator with one verified adapter instead of a full registry |

## Why this can be meaningfully better

1. **One boundary instead of N state silos.** VS Code, JetBrains, Docker Desktop, Postman, DBeaver and GitKraken each own their own settings, credentials and update cycles[138][140][142][146][163]. Devdeck keeps one versioned, Git-friendly workspace config and one consent trail.
2. **Environment drift handled by delegation, not reinvention.** mise and DevPod already solve large parts of versioning/devcontainers[150][152]; Devdeck orchestrates them behind one UI and one provenance ledger.
3. **Measured native performance.** The architecture enables low overhead; the benchmark harness in Deliverable 6 turns it into a claim only after p95 numbers exist on all three OSes[609][612].
4. **Safety by construction.** Deny-by-default capabilities, approval prompts, hash-chained audit, secret redaction, and sandboxed WASM/sidecar boundaries are designed in before the powerful tools arrive.

## Standing competitive trigger

Track Dodo continuously[37]. It is MIT-licensed, native GPUI, and already covers containers, DB, API explorer and developer utilities. If it ships stable releases and gains users, the "native all-in-one" framing must be re-evaluated against its actual depth and GPL linkage risk[40].

## Documents

- [01 — Competitive research](01-competitive-research.md)
- [02 — Technical architecture](02-technical-architecture.md)
- [03 — Product requirements](03-prd.md)
- [04 — Repository reuse assessment](04-reuse-assessment.md)
- [05 — UI/UX specification](05-ui-ux-spec.md)
- [06 — Performance strategy](06-performance-strategy.md)
- [07 — Implementation roadmap](07-implementation-roadmap.md)
- [08 — MVP definition](08-mvp-definition.md)

Full agent evidence: `research/competitive.md`, `research/repos/*.md`, `research/architecture.md`, `research/ui-frameworks.md`, `research/tool-management.md`, `research/ux-spec.md`, `research/prd.md`, `research/performance.md`.

## Sources

[1] https://github.com/l0ng-ai/openidea
[3] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/package.json
[8] https://github.com/l0ng-ai/openidea/blob/c209e189e082fe4b59e00ee9d7b13a19d0ae6458/src-tauri/tauri.conf.json
[37] https://github.com/MrGru/dodo
[40] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//THIRD-PARTY-NOTICES.md
[41] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//Cargo.toml
[103] https://github.com/Alno0/navop/blob/78dba5c/NAVOP_LICENSE
[135] https://github.com/zed-industries/zed
[138] https://github.com/microsoft/vscode
[140] https://www.jetbrains.com/store
[142] https://docker.com/pricing
[146] https://www.postman.com/pricing
[150] https://mise.jdx.dev
[152] https://devpod.sh/docs/what-is-devpod
[163] https://dbeaver.com
[302] https://crates.io/api/v1/crates/iced
[304] https://github.com/slint-ui/slint/blob/master/LICENSE.md
[345] https://docs.wasmtime.dev/api/wasmtime/component
[346] https://component-model.bytecodealliance.org/design/wit.html
[609] https://criterion-rs.github.io/book/user_guide/analysis.html
[612] https://github.com/sharkdp/hyperfine
