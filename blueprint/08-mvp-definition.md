# Deliverable 8 — MVP definition

## Name and thesis

**Devdeck Foundation** — one fast, local-first workspace that proves the daily loop and the tool-manager differentiator without attempting every subsystem.

The research PRD proposed cutting tool installation entirely and deferring LSP, Docker and databases(`research/prd.md` sections 9–12). CEO resolution: keep the PRD's minimal scope, but add a **narrow Tool Center slice** because a major product differentiator must be demonstrated in the MVP. We prove it with one adapter and one category, not a registry.

## In scope

| # | Capability | Why it is in |
| --- | --- | --- |
| 1 | Native shell: resizable tabs/splits, command palette, themes, settings, runtime en/vi | Already built in Phase 1; proves native UX |
| 2 | Project/workspace management, recents, atomic config + migrations | Required for every other feature |
| 3 | Real PTY terminal with tabs/panes, workspace cwd, redaction | Highest-value daily tool; feasible now |
| 4 | Git essentials: status, diff, hunk staging, commit, branch checkout, worktrees | Completes the J1 loop |
| 5 | Minimal editor: tabs, Tree-sitter, find/replace, large-file virtualization | Needed to call it a developer environment; **no LSP/DAP in MVP** |
| 6 | YAML/JSON editor with schema validation, files as source of truth | Proves Configuration Studio without a visual-editor rewrite |
| 7 | Tool Center v0: read-only detection for all tools + ONE consent-gated install/update/uninstall path for language runtimes via mise[150] | Demonstrates the differentiator with a MIT adapter and checksum/provenance ledger |
| 8 | Permissions, hash-chained audit, telemetry off, offline operation, keyboard-only journeys, en/vi parity | Non-negotiable quality bar |

## Explicitly out of scope for MVP

LSP/DAP (first sidecar phase), task runner beyond terminal commands, Docker/Compose, all database connectors, API client, registry/extension marketplace, agents/MCP, SSH/K8s, profiles/snapshots, full Configuration Studio, GUI application installation.

## User journeys supported

- J1: open project → edit file → run command → review diff → commit, fully offline.
- J3-lite: open Tool Center → see detected tools → approve one mise install plan → verify version → see provenance record.
- J4: tool asks for a risky action → approval prompt with plan/diff → allow/deny → audit entry.

## Gates inside the MVP

1. Gate A: shell + projects + terminal + Git pass J1 offline on three OSes.
2. Gate B: editor + config editor pass crash-recovery and file-source-of-truth tests.
3. Gate C (new): Tool Center install slice passes provenance/rollback/consent tests; only then may more adapters be added.
4. Sidecar/LSP, containers, DB and API each get their own later gate after permission and sandbox decisions.

## Success criteria

90% of 10+ participants complete J1 in <5 min offline; cold start p95 <1 s; idle core RSS <150 MB; keystroke p95 <16 ms; zero lost edits in 100 crash sessions; 100% en/vi key parity; all P0 journeys keyboard-only; CI green on three OSes with zero open P0 security findings; Tool Center slice records verified provenance for 100% of installs.

## Kill criteria

Median J1 >10 min, >20% of participants cannot complete an approval prompt, performance misses targets by >2x after optimisation, or the mise adapter cannot provide verifiable provenance on any target OS — in which case reduce Tool Center to read-only detection and revisit in Phase 6.

## Why this is meaningfully better

Not because it has more checkboxes than VS Code or Dodo[138][37], but because every capability lives behind one typed tool API, one permission decision, one audit trail and one versioned local config. Incumbents own separate config/credential silos and paid tiers[140][142][146]; mise and DevPod solve parts of environment drift but not the daily workspace[150][152]. Devdeck Foundation proves that boundary with a working daily loop and one verifiable install flow, then scales the same plumbing to the rest of the vision.

Full MVP analysis and the rejected 10-item scope evaluation: `research/prd.md` sections 9–12.

## Sources

[37] https://github.com/MrGru/dodo
[138] https://github.com/microsoft/vscode
[140] https://www.jetbrains.com/store
[142] https://docker.com/pricing
[146] https://www.postman.com/pricing
[150] https://mise.jdx.dev
[152] https://devpod.sh/docs/what-is-devpod
