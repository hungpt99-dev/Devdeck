# IND-212 — Phase 2 shell screenshot inspection note

Date: 2026-11-22. Environment: Xvfb (`xvfb-run -a -s "-screen 0 1400x1000x24"`),
`devdeck-app` debug build, window title `Devdeck`, mapped at 1200x800 with
absolute origin (0,0) confirmed by `xwininfo -id <window>`.

## Real screenshots

| File | Theme/locale | Verified size |
| --- | --- | --- |
| `phase2-shell-dark.png` | Dark, en | 1200x800 PNG, 57,666 B |
| `phase2-shell-light.png` | Light, en | 1200x800 PNG, 57,512 B |
| `phase2-shell-contrast.png` | High contrast, en | 1200x800 PNG, 55,800 B |
| `phase2-shell-vi.png` | Dark, vi | 1200x800 PNG, 65,772 B |

All four were captured with `import -window <window id>` from the mapped Xvfb
window; none is a mockup or a rendered design file.

## Region inspection (pixel probes, dark theme)

Sample coordinates are absolute window pixels. Colors are token values from
`crates/ui/src/lib.rs`, not ad-hoc literals.

| Probe | Result | Reading |
| --- | --- | --- |
| (20,47) header bottom edge | `srgb(53,64,80)` | header separated from body |
| (24,47) rail top edge | `srgb(53,64,80)` | activity rail starts at y=48 |
| (47,200) / (48,200) | `srgb(53,64,80)` / `srgb(27,33,43)` | 48 px rail border |
| (247,200) / (248,200) | `srgb(53,64,80)` / `srgb(91,168,255)` | 200 px sidebar ends; focused center starts |
| (959,200) / (960,200) | `srgb(91,168,255)` / `srgb(53,64,80)` | center ends; 240 px context bar starts |
| (400,528) | `srgb(53,64,80)` | splitter strip above the bottom panel |
| (400,772) | `srgb(53,64,80)` | 28 px status bar boundary |

The center region is keyboard-focused by default: its border is 2 px accent
(`srgb(91,168,255)`) while unfocused regions use 1 px border
(`srgb(53,64,80)`). Focus is therefore visible by width as well as color, so
nothing is signalled by color alone.

High contrast: region borders probe as `srgb(255,255,0)` and the focused
center border as `srgb(0,255,255)` (accent cyan), with panel
surfaces at `srgb(10,10,10)` and white text. All six regions remain visibly
delimited.

## Theme and locale differences (ImageMagick AE, 0 = identical)

- dark vs light: 960,000 px differ.
- dark vs high contrast: 959,960 px differ.
- dark vs vi: 20,244 px differ (localized text changed, same token palette).

## Interaction verification (XTEST key events, pixel diff against prior state)

| Action | Pixels changed |
| --- | --- |
| Ctrl+K opens palette | 192,532 |
| Escape closes palette | 192,532 |
| Ctrl+J hides bottom panel | 16,255 |
| Ctrl+J shows bottom panel again | 16,255 |
| Mouse drag of splitter (400,530) → (400,450) | 31,048 |
| Ctrl+` grows bottom panel | 26,637 |
| Ctrl+Shift+B hides context bar | 6,358 |
| F6 moves focus to next region | 3,310 |
| Shift+F6 returns focus | 3,310 (final image identical to baseline: 0 px) |

The shell is keyboard-first and mouse-resizable; the right context bar can be
hidden. Unit tests in `crates/ui/src/shell.rs` pin the geometry and
collapsed-state math that these interactions drive.

No design-review or mock-approval gate was created for this child. QA is the
assigned reviewer; Product/UX Lead approval remains pending as recorded in the
IND-210 plan.
