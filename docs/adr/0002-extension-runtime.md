# ADR 0002 — Extension runtime

Status: Accepted (2026-11-22).

## Decision

Extensions run as **WebAssembly components** in `devdeck-extension-host` using wasmtime 51.0.0 (component model, WIT worlds versioned per API major version). Permissions are capability-based and deny by default: the manifest declares fs paths, network hosts, process spawn, env vars, secrets, clipboard, database and agent access; the user approves on install and can revoke per workspace. Each extension has memory/fuel/time limits and is restarted with backoff on trap; failures surface in the Extensions panel and logs, never in the host process.

Native **out-of-process sidecars** speaking versioned JSON-RPC over stdio use the same permission model for LSP servers, DAP adapters, MCP servers and existing binaries. **Rhai** scripts handle small automations.

The `devdeck-api` WIT world is semver-versioned. Deprecations last at least two minor versions; breaking changes require a new ADR and a compatibility shim. Built-in modules use the same contribution-point types (`devdeck-extension-api`) as extensions — no private back doors.

## Rationale

The component model gives a stable typed ABI across languages, strong isolation without a webview, and per-extension capability enforcement. Wasmtime is the reference implementation and actively maintained. Sidecars avoid rewriting mature protocol servers. Rhai keeps small automations cheap and safe.

## Risks

- Component Model, WASI and MCP/ACP specs evolve quickly: pin versions, record them in the lockfile, and re-evaluate each release.
- Hot reload without restart is nontrivial for stateful extensions; Phase 2 ships restart-required updates if necessary.
- Fuel/time limits do not cover a malicious native sidecar; sidecars need OS-level sandboxing before untrusted use.
- Permission diffs on update must be shown and approved; silent escalation is forbidden.
