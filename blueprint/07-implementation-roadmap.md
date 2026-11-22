# Deliverable 7 — Implementation roadmap (proposal, not started)

Status: Phase 0 and the Phase 1 slice are already shipped on `main` (commit 7929181; hosted CI green). Everything below is proposed and requires review before coding.

| Phase | Milestone and scope | Depends on | Workstreams | Exit criteria |
| --- | --- | --- | --- | --- |
| 0 (done) | Spikes, ADRs 0001–0004, threat model, workspace skeleton, CI | — | architecture, quality | CI gates green |
| 1 (done) | Themed window, palette, en/vi, first typed tool, audit, contribution types, WIT draft | 0 | UI, core | 21 tests, Xvfb window, 3-OS build |
| 2 Foundation | Six-region shell (activity rail, tabbed splits, bottom panel), real PTY terminal + task runner, project model, read-only Tool Center, config migrations | 1 | UI, terminal, platform | J1 offline on 3 OSes; perf/a11y gates; no data loss in crash tests |
| 3 Editor + Git | Minimal editor with Tree-sitter, find, large-file virtualization; Git essentials incl. worktrees; YAML/JSON validated config editor | 2 | editor, Git, config | J1 with editor+commit; diff review and audit for all writes |
| 4 Trusted sidecars | Extension-host MVP (WASM command/tool with permission enforcement, crash isolation), LSP slice with one server, tool-install slice via mise adapter | 3 + signed native-sidecar sandbox decision | plugins, languages, tool center | Permission prompts enforced; malicious extension cannot crash host; provenance recorded |
| 5 Services | Docker/Podman container list/logs/exec + Compose controls; PostgreSQL + Redis query workspaces; REST client + OpenAPI import | 4 | containers, DB, API | Read-only defaults; destructive action approvals; secret redaction tests |
| 6 Depth | GraphQL/WebSocket/gRPC, MySQL/SQLite/MongoDB, more adapters, task workflows, worktree-based parallel agents | 5 | API, DB, agents | Category journeys pass; MCP/CLI parity test |
| 7 Production hardening | Benchmarks and gates, AccessKit solution, security review/fuzzing, installers/signing/auto-update, CJK+RTL stress, public beta | 6 | quality, security, release | Budgets met, audits passed, 3+ languages |
| 8 Platform | Extension registry/signing, Extensions panel, agent marketplace, P1 tools as bundled extensions | 7 | platform | 10+ third-party extensions through public API only |
| 9 1.0 | P2 toolbox, enterprise policies/private registry, docs site | 8 | all | VISION 1.0 criteria |

## Parallel workstreams

- UI/shell: Iced widgets, layouts, a11y, i18n surface.
- Core/platform: config, permissions, audit, events, workspace, storage.
- Protocol sidecars: LSP/DAP/MCP clients, sandbox decision, lifecycle supervision.
- Services: containers, databases, API client adapters.
- Supply chain: tool adapters, provenance ledger, extension signing.
- Quality: benchmarks, fuzzing, 3-OS CI, security/i18n gates.

## Top risks and mitigations

| Risk | Trigger | Mitigation |
| --- | --- | --- |
| Dodo gains adoption | release + user growth signal[37] | Re-evaluate positioning; lean harder on governance/adapters |
| Iced AccessKit gap blocks 1.0 | a11y audit | Upstream patch or maintained fork before Phase 7 exit[340] |
| GPL-linked runtime reappears | cargo-deny/advisory finding | External-process-only policy; legal review[40] |
| Approval fatigue | users disable prompts | Risk-tier prompts, quiet allow-lists, visible audit |
| Performance miss | benchmark p95 > target | Profile before adding features; cut scope, not gates |
| Config divergence | multiple writers | Single-writer store + atomic migrations |

Each phase exits only with tests, docs, localization updates, and the full CI gate. No phase starts before the previous exit criteria are met and reviewed.

## Sources

[37] https://github.com/MrGru/dodo
[40] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//THIRD-PARTY-NOTICES.md
[340] https://github.com/iced-rs/iced/issues/552
