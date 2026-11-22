# ADR 0001 — UI framework

Status: Accepted (2026-11-22). Supersedes nothing; revisit if GPUI reaches 1.0 or the dependency blocker below is fixed.

## Context

Devdeck needs a GPU-rendered, native Rust UI with high-quality text, IME, complex scripts/RTL, AccessKit accessibility, terminal and editor widgets on Windows, macOS and Linux (X11 + Wayland), plus a software fallback. Candidate frameworks evaluated: GPUI (Zed), Iced, Floem.

Current data checked 2026-11-22 from crates.io: GPUI 0.2.2 (official zed-industries/zed repository, 109 direct dependencies, edition 2024, explicitly pre-1.0 with frequent breaking changes); Iced 0.14.0 (last crates.io release 2025-12, wgpu + tiny-skia renderers, X11/Wayland, ~14 direct dependencies); Floem 0.2.0 (last crates.io release 2024-11 — stale, rejected).

## Spike

Standalone spikes live under `spikes/` and are excluded from the workspace.

- Iced 0.14.0 spike (`spikes/iced-spike`): text input, 100,000-row virtual list with bounded visible window, 200x50 monospace terminal-like grid, unit tests for windowing and grid indexing. `cargo test`: 2/2 passed. `xvfb-run`/`Xvfb` smoke: window titled "Devdeck UI spike (Iced 0.14)" mapped at 1024x768 (verified with `xwininfo -root -tree`); process produced only benign `libEGL`/DRI3 warnings on the software path. IME support is first-class in `iced_core-0.14.0/src/input_method.rs` (`InputMethod::Enabled`, `Preedit`, `Purpose::Terminal`); text shaping uses cosmic-text 0.15 + harfrust.
- GPUI 0.2.2 spike (`spikes/gpui-spike`): official hello-world example compiled against the pinned crate. Compilation FAILED on Linux with current stable Rust: `xattr v0.2.3` (pulled in by `gpui_http_client` → `zed-async-tar`) references `libc::ENOATTR`, which does not exist for linux-gnu in libc 0.2.191. The four-part spike (IME, 100k list, terminal grid, window) could not be completed because of this transitive dependency break. Source inspection found an X11 XIM handler (`gpui-0.2.2/src/platform/linux/x11/xim_handler.rs`), Windows DirectWrite and macOS CoreText backends, and a marked-range text input example (`examples/input.rs`) — the framework is capable, but the published crate is not buildable here without patching a third-party dependency.

## Decision

Use **Iced 0.14.0**, pinned exactly, with the `tokio` and `canvas` features. GPUI is the preferred long-term option only if a pinned git revision or future 1.0 release builds cleanly on all three target OSes; Floem is rejected for staleness.

## Rationale

Iced builds and runs today on the current toolchain, supports all three OS families, integrates a tokio executor for the required async architecture, has documented IME support including a terminal purpose, and offers a tiny-skia software fallback. GPUI is a stronger base for an editor/terminal (Zed proves it) but its published crate is pre-1.0 and currently broken by a transitive dependency; adopting it would require carrying a fork from day one.

## Consequences and risks

- Upstream Iced 0.14 contains no AccessKit integration (no `accesskit` reference in iced 0.14 sources). Accessibility baseline is therefore PARTIAL: keyboard navigation and localized accessible labels exist, but screen-reader tree support needs an upstream contribution or a maintained fork before 1.0. Tracked in docs/progress.md.
- Virtualized lists and terminal/editor widgets must be built in Devdeck; the spike demonstrates the windowing math but not production widgets.
- Iced's declarative component vocabulary for extensions must map to native widgets only; no arbitrary drawing/webviews.
- Debug binaries are large (~421 MB for the spike, unstripped); release profile uses `strip` + thin LTO. Startup and RAM budgets are NOT yet measured.
