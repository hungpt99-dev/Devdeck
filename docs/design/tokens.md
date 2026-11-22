# Design system

Tokens live in `crates/ui/src/lib.rs` (`Tokens`, `SPACE_*`, `RADIUS`, `BODY_SIZE`, `SIDEBAR_WIDTH`, `STATUS_HEIGHT`). Dark, light and high-contrast palettes are defined there; no widget hard-codes colors. Motion durations and reduced-motion handling are Phase 7 work. Components: sidebar, status bar, command palette; more will be added with each panel.

Accessibility: keyboard-first navigation and localized accessible names via Fluent. Upstream Iced 0.14 lacks AccessKit, so screen-reader support is an accepted gap (ADR 0001) to close before 1.0.
