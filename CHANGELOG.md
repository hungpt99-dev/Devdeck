# Changelog

Format follows Keep a Changelog; versions follow Semantic Versioning.

## [Unreleased]

### Added
- Phase 0: workspace scaffolding (28 crates), CI gate definition, ADRs 0001-0004, threat model v1.
- Phase 1: themed native window (dark/light/high-contrast), sidebar/tab/status-bar shell, Ctrl/Cmd+K command palette with diacritic-insensitive search, runtime en/vi switching, first typed tool (`files.read`) with permission decision and hash-chained audit log, extension contribution-point types and draft WIT world.
- Visual smoke screenshot under Xvfb (docs/screenshots/phase1-shell.png).
- UI framework spikes for Iced 0.14 and GPUI 0.2.2 with measured results in ADR 0001.
- Phase 2 F5: workspace `devdeck.toml` task runner with typed `tasks.list`/`tasks.run` tools, F3 PTY execution, shared permission decision, localized errors and hash-chained audit entries (en/vi).
