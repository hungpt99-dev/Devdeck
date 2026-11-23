# Threat model v1 (2026-11-22)

Scope: host core, extensions, agents, MCP/API servers, registry/updater, secrets, imported files, config migrations. Legend: [I] implemented in Phase 0/1, [P] planned with mechanism named, [G] gap requiring human review.

| Surface | Threat | Mitigation | Status |
| --- | --- | --- | --- |
| WASM extension | Infinite loop / memory bomb | wasmtime fuel + memory limits, timeout, restart with backoff | [P] Phase 2 |
| WASM extension | Path/network/process abuse | Manifest capabilities, deny by default, per-workspace approval/revocation | [P] Phase 2; [I] permission types |
| WASM extension | Panic/malformed module crashes host | Component boundary, trap converted to localized error | [P] Phase 2 |
| Native sidecar | Malicious binary | Versioned JSON-RPC, same permission model, OS sandbox before untrusted use | [G] OS sandbox not designed yet |
| Agent tool call | Destructive command (`rm -rf`, force push, DROP/DELETE) | Policy engine forces approval; `devdeck_core::is_dangerous`; audit entry; `terminal.spawn` calls `PermissionPolicy::decide` before any PTY is created and returns the decision without executing | [I] policy + audit + terminal decision; [G] approval UI, founder human review |
| Agent context | Prompt injection via file/web/DB/tool output | Treat all tool output as untrusted; require approval when it triggers sensitive actions | [G] Phase 6.5 |
| Secrets | Token/.env/keychain value leaked to model, log or extension | Redaction in audit, keychain-only storage, explicit permission before exposure; terminal output is redacted before the grid and audit; PTY shells get no history file (`HISTFILE`, `HISTFILESIZE`, `HISTSIZE`) | [I] audit + terminal redaction; [P] keychain Phase 4; [G] founder human review of terminal redaction coverage |
| MCP/API server | Remote access, DNS rebinding, cross-origin request | Bind localhost only, auth token, Origin/Host validation | [G] Phase 3.5 |
| Registry/updater | Malicious or tampered extension/update | Signed archive, checksum, publisher verification, permission diff, block-list | [G] Phase 7.5 |
| Auto-update | Rollback to vulnerable version | Staged rollout, signature verification, one-click rollback | [G] Phase 7 |
| Imported files | Malicious Postman/OpenAPI/HAR/cURL payload | Fuzzed parsers, size limits, no script execution on import | [G] Phase 4.5 |
| Config | Corruption / version skew | Atomic writes, schema version, forward migration, backup before destructive migration | [I] v0→v1 migration and atomic save |
| Audit log | Tampering / secret leakage | SHA-256 hash chain, append-only JSONL, redaction before write | [I] |
| Crash reports | Paths/secrets uploaded | Local-first, opt-in upload, scrubbed bundles | [G] Phase 7 |

Human review required before 1.0: sandbox design for native sidecars, MCP authentication, registry signing/key management, and prompt-injection defenses. IND-213's PTY engine and approval boundary additionally require founder human review (Security Lead not yet approved); the engine exposes a permission decision only and is not marked done until that review and QA's verdict are recorded.

## IND-213 terminal engine — implementation note (2026-11-22)

Added `devdeck_terminal` (`new`/`spawn`/`resize`/`write`/`close`/`pump`) on top of `portable-pty 0.9.0` (MIT) and `vte 0.15.0` (Apache-2.0 OR MIT), both exact-pinned; justification recorded in `deny.toml`. Sessions spawn the workspace shell with `cwd` from `WorkspaceInfo`, stream PTY bytes through a bounded tokio channel into a vte grid capped at `rows + scrollback_lines`, and redact secrets before the grid or audit. `terminal.spawn` reuses `devdeck_core::PermissionPolicy::decide` with no policy fork: `rm -rf`, `git push --force`, `sudo` and `curl | sh` return `RequiresApproval` and never start a process. Typed `terminal.spawn/write/resize/close` tools carry schemars-generated JSON Schema; en/vi bundles carry tool and error text. The approval dialog, tabs/panes and task runner remain later children. Status: in review; founder human review pending.

