# ADR 0004 — Licensing

Status: Accepted (2026-11-22).

## Decision

Devdeck is dual-licensed **MIT OR Apache-2.0**. Third-party dependencies must be MIT, Apache-2.0 or otherwise demonstrably compatible permissive licenses (BSD-2/3-Clause, ISC, Unicode, Zlib). `cargo-deny check licenses` runs in CI; any exception needs a PR justification and an update to `deny.toml`. `Cargo.lock` is committed; dependencies are pinned and reviewed with `cargo audit`/`cargo deny advisories`.

## Rationale

Dual permissive licensing maximizes adoption by developers and enterprises, matches the Rust ecosystem, and avoids copyleft obligations for a tool developers install everywhere.

## Risks

- Transitive GPL/AGPL or unusually licensed crates must be rejected or isolated in a separate process with a documented legal review.
- License metadata of some crates is incomplete; `cargo deny` may need explicit per-crate allowances, each recorded in `deny.toml` with a reason.
