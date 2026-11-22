# Instructions for coding agents

1. Read `VISION.md`, `AGENTS.md` and `docs/progress.md` before starting work. Work on ONE phase/task at a time; do not jump ahead or add scope.
2. Write a short plan before coding (goal, files/crates touched, tests, risks). Significant decisions require an ADR in `docs/adr/`.
3. Small, reviewable commits, conventional commit messages. Every PR includes tests, docs and localization updates (en + vi).
4. Run the full CI gate locally before declaring anything done: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check`, `cargo run -p devdeck-xtask -- i18n-lint`. Show the actual output.
5. Never claim something works without running it. Never disable tests, lints or checks to make them pass. Never fabricate benchmarks, test results or screenshots.
6. Zero hard-coded user-facing strings: all labels, errors, notifications and onboarding text go through `devdeck_i18n`; add keys to both `locales/en` and `locales/vi`. Canonical tool names/descriptions stay English and stable.
7. Every feature registers through public contribution points in `devdeck-extension-api`. Built-ins get no private back doors. Every capability is a typed tool in `devdeck-tools` with permission/risk, audit entry, CLI and MCP exposure.
8. Core never depends on UI. UI crates depend on core. All blocking work runs on tokio, never on the UI thread.
9. Security-sensitive code (permissions, sandbox, secrets, extension host, updater, MCP server) gets extra tests, a threat-model note and human review. Secrets are redacted from logs and audit entries.
10. Keep `main` buildable on Windows, macOS and Linux. Use atomic writes and config migrations; never lose user data.
