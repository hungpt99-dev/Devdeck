# Contributing

Read `VISION.md`, `AGENTS.md` and `docs/progress.md` first. Pick one task from the current phase; discuss large changes in an issue; breaking API changes require an ADR.

## Development

```bash
cargo build --workspace
cargo test --workspace
cargo run -p devdeck-app
cargo run -p devdeck-cli -- doctor
cargo run -p devdeck-xtask -- i18n-lint
```

Linux needs X11/Wayland, xkbcommon, fontconfig, freetype and ALSA development packages. The app falls back to software rendering when no GPU is available.

## Pull requests

- Conventional commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`).
- One coherent change per PR, with tests, docs and en/vi locale updates.
- Justify every new dependency (purpose, license, maintenance, why no existing crate works).
- Security-sensitive changes must update the threat model.
- CI must be green: fmt, clippy `-D warnings`, tests, cargo-deny, cargo-audit, i18n lint, 3-OS build.

## Adding a language

Copy `locales/en/*.ftl`, translate every value, keep keys identical, register the locale in `crates/i18n`, then run the i18n lint. Pseudo-localized and RTL stress tests run before a language ships.
