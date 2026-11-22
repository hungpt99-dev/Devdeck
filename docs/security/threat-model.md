# Threat model v1 (2026-11-22)

Scope: host core, extensions, agents, MCP/API servers, registry/updater, secrets, imported files, config migrations. Legend: [I] implemented in Phase 0/1, [P] planned with mechanism named, [G] gap requiring human review.

| Surface | Threat | Mitigation | Status |
| --- | --- | --- | --- |
| WASM extension | Infinite loop / memory bomb | wasmtime fuel + memory limits, timeout, restart with backoff | [P] Phase 2 |
| WASM extension | Path/network/process abuse | Manifest capabilities, deny by default, per-workspace approval/revocation | [P] Phase 2; [I] permission types |
| WASM extension | Panic/malformed module crashes host | Component boundary, trap converted to localized error | [P] Phase 2 |
| Native sidecar | Malicious binary | Versioned JSON-RPC, same permission model, OS sandbox before untrusted use | [G] OS sandbox not designed yet |
| Agent tool call | Destructive command (`rm -rf`, force push, DROP/DELETE) | Policy engine forces approval; `devdeck_core::is_dangerous`; audit entry | [I] policy + audit; [G] approval UI |
| Agent context | Prompt injection via file/web/DB/tool output | Treat all tool output as untrusted; require approval when it triggers sensitive actions | [G] Phase 6.5 |
| Secrets | Token/.env/keychain value leaked to model, log or extension | Redaction in audit, keychain-only storage, explicit permission before exposure | [I] audit redaction; [P] keychain Phase 4 |
| MCP/API server | Remote access, DNS rebinding, cross-origin request | Bind localhost only, auth token, Origin/Host validation | [G] Phase 3.5 |
| Registry/updater | Malicious or tampered extension/update | Signed archive, checksum, publisher verification, permission diff, block-list | [G] Phase 7.5 |
| Auto-update | Rollback to vulnerable version | Staged rollout, signature verification, one-click rollback | [G] Phase 7 |
| Imported files | Malicious Postman/OpenAPI/HAR/cURL payload | Fuzzed parsers, size limits, no script execution on import | [G] Phase 4.5 |
| Config | Corruption / version skew | Atomic writes, schema version, forward migration, backup before destructive migration | [I] v0→v1 migration and atomic save |
| Audit log | Tampering / secret leakage | SHA-256 hash chain, append-only JSONL, redaction before write | [I] |
| Crash reports | Paths/secrets uploaded | Local-first, opt-in upload, scrubbed bundles | [G] Phase 7 |

Human review required before 1.0: sandbox design for native sidecars, MCP authentication, registry signing/key management, and prompt-injection defenses.
