# Deliverable 3 — Product requirements document

## Problem statement

Developers switch between 6–10 applications with separate configs, credentials and state. Tool sprawl costs focus; environment drift blocks onboarding; licence costs fragment across Docker Desktop, JetBrains, Postman and GitKraken[142][140][146]. Devdeck must provide one native workspace where the daily loop — open project, edit, run, inspect Git, operate services, install tools — happens without context switches, with local-first data and explicit consent.

## Personas

- PER-1 Polyglot application developer: switches languages/repos; needs fast editor, terminal, Git, tool versions.
- PER-2 Backend/data developer: SQL, Redis/Postgres, API requests, migrations; needs safe write approvals.
- PER-3 Platform/DevOps engineer: containers, toolchains, kubectl/Terraform CLIs, environment reproducibility.
- PER-4 Tech lead/onboarding owner: reusable profiles, snapshots, documented setup for new machines.
- PER-5 Localized/assistive-technology developer: IME (Vietnamese Telex), RTL, keyboard-only, screen reader.

## Jobs and journeys

- J1 Daily loop: open workspace → edit file → run terminal command → review diff → commit. Must work offline.
- J2 Service loop: start container → inspect logs → query DB → send API request → inspect response → revert a bad change.
- J3 Onboarding loop: import profile → discover missing tools → approve an install plan → reproduce environment → verify.
- J4 Safety loop: an agent/tool asks for a destructive action → see diff/plan → approve or reject → audit entry.

## Functional requirements (mapping to the requested A–I)

| Area | P0 (MVP or first release) | P1 | P2 | Testable acceptance example |
| --- | --- | --- | --- | --- |
| A Editor/LSP/debug | Tabs, Tree-sitter, find/replace, large-file virtualization; single LSP slice and one DAP adapter after sidecar gate | Refactor, multi-cursor, full LSP set, test runner | Advanced debugging | Open 50 MB file; first window <2 s; completion <=100 ms after sidecar warm-up |
| B Terminal | PTY, tabs/splits, scrollback, workspace cwd, secret redaction | Shell integration, SSH sessions | Hyperlinks, search | 100 MB/s PTY drain without UI thread stall (target, unmeasured) |
| C Git | Status, diff, stage/unstage, commit, branch checkout, worktrees | Log graph, stash, blame, merge conflicts | Hosting integrations | All writes through public tool with audit entry |
| D Docker/Compose | (MVP: no) Container list/logs/exec + Compose up/down in P1 | Images/volumes/networks/resource usage | Compose visual editor | Destructive container ops require approval |
| E Databases | (MVP: no) PostgreSQL + Redis first; read-only default | MySQL/SQLite/MongoDB, schema editor | Backup/restore, ER diagrams | Write query requires approval; no secret in logs |
| F API | (MVP: no) REST + OpenAPI import | GraphQL/WebSocket/gRPC, collections, scripting | Mock server, CI runner | 50 MB response scrolls smoothly; cURL export |
| G Tool management | Read-only detection + ONE verified install/update/uninstall adapter (mise runtime slice) | More adapters, version switching, PATH conflict detection | GUI apps, offline cache | Every install records checksum/provenance + approval |
| H Configuration | Raw + validated YAML/JSON editor; files remain source of truth | TOML/XML/Dockerfile, schema registry, diff/history | Templates, import/export | No hidden state; config diff visible in Git |
| I Workspace/automation | Per-workspace settings, recents, atomic config/migrations | Profiles, named snapshots, task workflows | Cross-machine reproduce | 100 crash-injection sessions with zero lost edits |

## Non-functional requirements

- Performance: targets in Deliverable 6; none claimed as achieved.
- Security: deny-by-default capabilities, approval for dangerous operations, hash-chained audit, OS keychain for secrets, redaction in logs/context.
- Privacy: telemetry off unless opted in; local-first; no code/paths/queries in telemetry.
- Cross-platform: Windows 10/11, macOS current + previous two, Linux X11/Wayland; CI builds all three.
- i18n/a11y: en+vi at first release, runtime switch, typed keys, keyboard-only P0 journeys, AccessKit gap tracked.
- Reliability: atomic writes, autosave/recovery, forward migrations, graceful degradation.

## Acceptance criteria and success/kill criteria

Success: 90% of 10+ participants complete J1 offline in <5 min; cold start p95 <1 s; idle core RSS <150 MB; keystroke p95 <16 ms; zero lost edits across 100 crash sessions; 100% en/vi key parity; all P0 journeys keyboard-only; CI green on three OSes with zero open P0 security findings.

Kill: median J1 completion >10 min, >20% of participants cannot recover from a permission prompt, or any measured performance target misses by >2x on reference hardware after optimisation. Reference hardware is an explicit open question.

Full 96-requirement PRD with detailed IDs and user journeys: `research/prd.md`.

## Sources

[140] https://www.jetbrains.com/store
[142] https://docker.com/pricing
[146] https://www.postman.com/pricing
