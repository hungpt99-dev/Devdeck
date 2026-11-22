# Native UI foundation evaluation: GPUI vs Iced vs Slint

Date: 2026-11-22. Toolchain: `rustc`/`cargo` 1.100.0 (stable, Linux x86_64). Scope: research only; the product tree (`app/`, `crates/`, CI, ADRs, locales) was not modified. The only new files are the throwaway spike and this report/ledger under `research/`.

## Decision

**Keep Iced 0.14.0.** Weighted score: Iced **82.0**, GPUI **75.4**, Slint **74.2** (100-point scale; weights below). The result is driven by three project-specific hard points. First, a terminal/editor must be built as a custom widget, which Iced's public `Widget`/`Renderer` traits and terminal-purpose IME API support directly[24][20]. Second, Devdeck's licensing ADR permits only permissive licenses and CI runs `cargo-deny` allowlists, while Slint is triple-licensed GPLv3 / royalty-free / commercial rather than permissive[5][15]. Third, GPUI's published 0.2.2 crate does not build on this toolchain because of a transitive `xattr` 0.2.3 / `libc` ENOATTR break recorded in ADR 0001 and still present in the spike lockfile. Iced's main weakness — no shipped AccessKit integration — is a gap to close, not a framework switch: accessibility issue #552 has been open since 2020 and the AccessKit PR is still an open draft[41][25].

If legal approves Slint's royalty-free license and accepts its attribution obligation, Slint becomes the closest alternative (its release is fresh, it has built-in virtualization, AccessKit-backed accessibility, and a real software renderer)[1][31][10]; it still loses on custom terminal/editor work.

## Current facts (retrieved 2026-11-22)

| Framework | Latest crate | Published | Metadata license | Notes |
|---|---|---|---|---|
| GPUI | 0.2.2 | 2025-10-22 | Apache-2.0 | 7 published versions; pre-1.0 with frequent breaking changes[3][21] |
| Iced | 0.14.0 | 2025-12-07 | MIT | 23 published versions; default features include `wgpu`, `tiny-skia`, `x11`, `wayland`[34][2] |
| Slint | 1.19.0 | 2026-11-16 | GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0 | 53 published versions; active release cadence[1][15] |

## Verification of spikes (local evidence)

- ADR `docs/adr/0001-ui-framework.md` was read; both spike sources under `spikes/` were inspected.
- Iced 0.14.0 (`spikes/iced-spike`): re-ran `cargo test` — exit 0, `2 passed; 0 failed` (`tests::grid_indexing_is_dense`, `tests::virtual_list_window_is_bounded`); log `research/spikes/slint-spike/verify-iced-test.log`. Re-ran `cargo build` — exit 0; debug binary 411,292 KB. Xvfb smoke mapped `Devdeck UI spike (Iced 0.14)` at 1024x768 (`xwininfo`), log `research/spikes/slint-spike/verify-iced-window.sh` output. No new source changes were made.
- GPUI 0.2.2 (`spikes/gpui-spike`): source and `Cargo.lock` inspected; lock still pins `xattr 0.2.3` and `libc 0.2.191`, the exact break recorded in ADR 0001. Per instruction, the full build was **not** repeated; the ADR's failed-build result stands.
- New Slint 1.19.0 spike (`research/spikes/slint-spike/`): `Cargo.toml`, `build.rs`, `src/main.rs`, `ui/app.slint` (window + `LineEdit` + 100,000-row `ListView`), `smoke.sh`. Exact sequence:
  1. `cargo build` → **exit 101** (two `.slint` source errors: `Text` wrongly imported from `std-widgets.slint`, and an invalid `\(i)` escape). Log: `build-debug.log`.
  2. Fixed `.slint` markup, then `cargo build` → **exit 0**, 18.41 s incremental, 348 MB unstripped debug binary. Log: `build-debug2.log`.
  3. `cargo build --release` → **exit 0**, 474.72 s cold build (556 crates locked), **30 MB** release binary. Log: `release-build.log`.
  4. `bash smoke.sh` with `DISPLAY=:98`, `SLINT_BACKEND=winit-software` → exit 0; `xwininfo` showed `Devdeck UI spike (Slint 1.19)` mapped at 800x600; app log empty. Log: `smoke.log`.
- Iced release-build comparison: `cargo build --release` → exit 0, 327.12 s cold build, 23 MB unstripped release binary. Log: `research/spikes/slint-spike/verify-iced-release.log`.

## Criterion-by-criterion findings

### Text rendering, shaping, bidi/RTL

Iced 0.14 shapes text with cosmic-text 0.15[37], which depends on harfrust 0.3.1 and unicode-bidi[36]. Slint unified its four renderers behind Fontique and Parley in 1.14[23]; the 1.19 sources still contain a rustybuzz `TextShaper` implementation[18] and Parley's `complex-scripts` feature[19]. Slint's text input aligns to "start"/"end" edges depending on text direction[13], which is the correct abstraction for RTL. GPUI's Linux feature set pulls in cosmic-text and font-kit[3], with platform-native text backends described in its README[21]. Verdict: all three have credible modern shaping; Slint and Iced are strongest on documented bidi plumbing, GPUI has the deepest editor-specific line layout but the least portable text stack (placeholder glyphs on macOS without font-kit)[21].

### IME and preedit

Iced has first-class `InputMethod::Enabled` with a `Preedit` overlay and `Purpose::Terminal` for terminal input[20]. Slint 1.19 core already carries composition events with `preedit_text` and `preedit_selection`[40], and `TextEdit` exposes platform input-method hints[12]; however, letting a *custom* editor surface act as an IME client is still an open PR as of 2026-11-16[27]. GPUI's 0.2.2 X11 handler receives XIM preedit/commit callbacks[42] and its input example tracks a marked range[43]; coverage on Wayland/Windows is less explicit than Iced's widget API. Verdict: Iced is the best fit for a terminal that must own its preedit rendering.

### Accessibility

Slint enables `accessibility` by default and exposes the element tree to OS accessibility APIs[10]; its build pulled in AccessKit/AT-SPI crates. GPUI documents AccessKit integration and an accessibility guide[22]. Iced 0.14 has no AccessKit slot: issue #552 remains open[41] and the integration PR remains an open draft limited to a counter example[25]. Verdict: this is Iced's largest functional deficit and the main reason to revisit the decision if the draft lands.

### Terminal / editor widget feasibility and extensibility

GPUI is the strongest base: Zed ships a built-in terminal emulator with shell, scrollback and integration[45], and GPUI's `Render` trait is the public extension point[33]. Iced exposes a full public `Widget<Message, Theme, Renderer>` trait plus a `canvas` feature, so a terminal grid and code editor can be implemented in-product[24][34]; the IME `Purpose::Terminal` hint is directly relevant[20]. Slint ships `TextEdit` and virtualized `ListView` widgets[12][31]. Custom drawing is primarily expressed in `.slint` markup; arbitrary Rust-rendered widgets require a custom platform/renderer integration[10], and custom-editor IME is not merged[27]. Verdict: Iced and GPUI are practical; Slint would force a second rendering layer for a terminal.

### Cross-platform and software fallback

Iced's default features include both `x11` and `wayland` plus the `tiny-skia` software renderer[34]. Slint's winit backend explicitly supports Windows, macOS, X11 and Wayland[11], and offers `renderer-software` selected by `SLINT_BACKEND=winit-software`[10] — confirmed by our Xvfb smoke. GPUI's README lists Win32/DirectWrite, Metal/CoreText and Linux Wayland/X11 with feature flags[21], but its software story is weak: community reports describe hard failure without a GPU adapter on Windows[29] and a few-FPS experience on emulated GPUs[28]. Verdict: Iced and Slint are safe; GPUI needs a GPU or WARP-class fallback work.

### Stability, maintenance, build cost

GPUI is explicitly pre-1.0 with frequent breaking changes[21] and has had no crate release since 2025-10[3]. Iced's last release was 2025-12[34] and its design philosophy is openly opinionated and maintainer-led[38]; slower cadence is a risk. Slint is the most actively released of the three[1]. On measured build cost, Slint's clean release build took 474.72 s and produced a 30 MB binary, versus Iced's 327.12 s and 23 MB; debug binaries are 348 MB (Slint) and 411 MB (Iced). GPUI could not be built at all. These are unstripped local measurements, not optimized shipping sizes.

### Licensing (decisive for Slint)

Slint's official license file states the framework is available under any of three choices: royalty-free (proprietary desktop/mobile/web at no cost, embedded excluded), GPLv3, or paid commercial[5]. The FAQ adds that the royalty-free option requires showing the `AboutSlint` attribution[6][7]. These are **not** MIT/Apache-2.0; Devdeck's ADR 0004 permits only permissive licenses and `deny.toml` has no ID for any Slint license, so adoption needs a legal decision and a CI policy exception, not just a code change. Iced's crate metadata is MIT[34] and GPUI's is Apache-2.0[3], both already compatible.

## Decision matrix

Score 1 (poor) to 5 (excellent). Weights sum to 100 and reflect this project's stated requirements (editor/terminal first, accessibility and licensing as hard constraints). Weighted total = Σ(score × weight) / 5.

| Criterion | Weight | Iced | GPUI | Slint |
|---|---:|---:|---:|---:|
| Buildability & stability on current toolchain | 12 | 5 | 1 | 5 |
| Text rendering & bidi | 9 | 4 | 5 | 5 |
| IME / preedit | 8 | 5 | 4 | 3 |
| Accessibility (AccessKit) | 12 | 1 | 5 | 5 |
| Terminal / editor feasibility | 13 | 4 | 5 | 2 |
| Widget extensibility | 8 | 5 | 5 | 3 |
| Cross-platform (Win/mac/X11/Wayland) | 9 | 5 | 4 | 5 |
| Software rendering fallback | 7 | 5 | 1 | 5 |
| License compatibility | 12 | 5 | 5 | 1 |
| Maintenance / cadence | 5 | 3 | 3 | 5 |
| Build cost / binary size | 5 | 3 | 1 | 3 |
| **Weighted total (of 100)** | **100** | **82.0** | **75.4** | **74.2** |

Sensitivity: removing the license weight entirely makes Slint ~81.6 and Iced ~79.5 — so the recommendation is genuinely contingent on ADR 0004's permissive-license constraint. If that constraint is relaxed, re-run this matrix with terminal/editor effort re-weighted before switching.

## Migration cost (engineering estimates, not measured)

- **Keep Iced (recommended): ~0 framework migration.** Budget separately for an AccessKit contribution or maintained fork, and for production terminal/editor widgets; the existing spike already proves windowing math and grid indexing.
- **Switch to Slint: high.** Rewrite the widget tree in `.slint` markup plus Rust callbacks, rebuild terminal/editor drawing (likely a custom platform/renderer), obtain legal approval for a non-permissive license, and add a `cargo-deny` exception. Estimate 8–16 engineer-weeks before feature parity.
- **Switch to GPUI: high and blocked.** First repair or vendor the `xattr`/`libc` break, then port UI code to `Render`/elements; accept pre-1.0 API churn and the missing software fallback. Estimate 6–12 engineer-weeks after the dependency is fixed.

## Top risks

1. Iced accessibility: no shipped screen-reader tree; the AccessKit work is an open draft[25][41].
2. Iced release cadence and maintainer concentration[34][38].
3. Slint license conflict with ADR 0004 and attribution duty under the royalty-free option[5][6].
4. Slint custom-editor IME remains unimplemented upstream[27]; terminal widgets are not built in.
5. GPUI's transitive build break, pre-1.0 churn[21], and absent native software renderer[28][29].
6. Shipping size/startup/RAM are still unmeasured for release builds; debug binaries are large.

## Open questions

- Will legal approve Slint's royalty-free license (with `AboutSlint` attribution) despite ADR 0004's permissive-only rule?
- Can AccessKit be upstreamed to Iced on a supported release, or must Devdeck carry a fork?
- Does Iced's cosmic-text stack render Arabic/Hebrew and mixed-direction terminal text correctly end-to-end? (Not covered by the spike.)
- Does Slint 1.19 handle desktop IME preedit correctly on Windows, macOS and Wayland, not just Xvfb/X11?
- When will GPUI's `xattr`/`libc` break be fixed upstream, and when is 1.0 planned?
- What are release-binary startup time and RSS for Iced vs Slint on all three OS families?

## Sources

[1] https://crates.io/api/v1/crates/slint
[2] https://crates.io/api/v1/crates/iced
[3] https://crates.io/api/v1/crates/gpui
[5] https://github.com/slint-ui/slint/blob/master/LICENSE.md
[6] https://github.com/slint-ui/slint/blob/master/FAQ.md
[7] https://slint.dev/faqs
[10] https://docs.slint.dev/latest/docs/rust/slint/docs/cargo_features
[11] https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backend_winit
[12] https://docs.slint.dev/latest/docs/slint/reference/std-widgets/views/textedit
[13] https://docs.slint.dev/latest/docs/slint/reference/keyboard-input/textinput
[15] https://crates.io/api/v1/crates/slint/1.19.0
[18] https://github.com/slint-ui/slint/blob/v1.19.0/internal/core/textlayout/shaping.rs
[19] https://github.com/slint-ui/slint/blob/v1.19.0/internal/core/Cargo.toml
[20] https://github.com/iced-rs/iced/blob/0.14.0/core/src/input_method.rs
[21] https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md
[22] https://github.com/zed-industries/zed/blob/main/crates/gpui/src/_accessibility.rs
[23] https://slint.dev/blog/slint-1.14-released
[24] https://docs.rs/iced/0.14.0/iced/advanced/widget/trait.Widget.html
[25] https://github.com/iced-rs/iced/pull/3111
[27] https://github.com/slint-ui/slint/pull/14292
[28] https://github.com/zed-industries/zed/discussions/57066
[29] https://github.com/zed-industries/zed/discussions/50972
[31] https://docs.slint.dev/latest/docs/slint/reference/std-widgets/views/listview
[33] https://docs.rs/gpui/0.2.2/gpui/trait.Render.html
[34] https://crates.io/api/v1/crates/iced/0.14.0
[36] https://github.com/pop-os/cosmic-text/blob/0.15.0/Cargo.toml
[37] https://github.com/iced-rs/iced/blob/0.14.0/graphics/Cargo.toml
[38] https://book.iced.rs/philosophy.html
[40] https://github.com/slint-ui/slint/blob/v1.19.0/internal/core/input.rs
[41] https://github.com/iced-rs/iced/issues/552
[42] https://docs.rs/crate/gpui/0.2.2/source/src/platform/linux/x11/xim_handler.rs
[43] https://docs.rs/crate/gpui/0.2.2/source/examples/input.rs
[45] https://zed.dev/docs/terminal
