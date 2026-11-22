# Deliverable 1 — Competitive research report

## Method and evidence standard

Three repositories were cloned and inspected file-by-file (source, `Cargo.toml`, frontend manifests, licenses, commits/issues); twelve product categories were scanned using official repos/docs/pricing pages. The full competitive report cites 79 distinct sources and includes a kill list and code-computed scoring(`research/competitive.md`). All external facts below carry a source; analytical judgements are labelled ANALYSIS.

## Market structure

| Segment | Representative | Stack / licence | Strength | Gap vs Devdeck vision |
| --- | --- | --- | --- | --- |
| Editors/IDEs | VS Code[138], JetBrains[140], Zed[135] | Electron MIT core; JVM proprietary; native Rust GPL core | Deep editing, ecosystem, LSP/DAP | Separate terminal/Git/DB/API/tool state; per-product config and licensing |
| Containers | Docker Desktop[142] | Proprietary with paid tiers | Best container UX | License thresholds; not an editor/workspace |
| Databases | DBeaver[163] | Java/Eclipse RCP, Apache-2.0 community | Broad driver support | Heavy UI; separate from code/Git/terminal |
| API | Postman[146] | Electron, tiered pricing | Collaboration, tooling depth | Separate state; account/cloud gravity |
| Git | GitKraken | Electron, paid tiers | Visual history | Separate from editor/terminal |
| Tool management | mise[150], UniGetUI[148] | Rust/MIT; C# GUI | Version lockfiles, GUI package management | Not a development workspace; no unified permission/audit |
| Environments | DevPod[152] | Go/MPL-2.0 (adapters) | Devcontainer providers | Weak recent maintenance signal; separate from daily editing |
| Native all-in-one | Dodo[37] | Native GPUI, MIT source | Closest analogue: API, Docker, DB, utilities | Young (~7 stars), GPL-linked dependency chain[40], no mass adoption |

ANALYSIS: the gap is not "no all-in-one app exists"; Dodo matches the framing[37]. The gap is **one governed local boundary** across all tools, plus reproducible environments via adapters.

## Scored comparison (from the competitive report)

Weights: native/resource efficiency 20%, all-in-one breadth 25%, ecosystem 15%, licence/cost 15%, maintenance 15%, reuse compatibility 10%; computed in code. mise 3.95, Dodo 3.75, VS Code 3.75, Zed 3.50, UniGetUI 3.10, JetBrains 2.95, DBeaver 2.95, DevCat 2.85, Docker Desktop 2.65, Postman 2.45, GitKraken 2.45, DevPod 2.10 (out of 5). Devdeck-as-planned scores 3.95 but is excluded: a plan is not a product.

## Genuine pain points (open)

1. Context switching across separately licensed, separately stateful tools.
2. Environment drift; only partly solved by mise[150] and DevPod[152], which should be integrated rather than cloned.
3. Licensing/cost fragmentation across Docker Desktop, JetBrains, Postman, DBeaver and GitKraken[142][140][146].

## Kill list — differentiators that are NOT defensible

- "Only native Rust all-in-one" — Dodo[37].
- "Native is automatically faster" — no independent benchmarks; GitKraken has shipped real Electron performance work.
- "Replace the whole toolchain" — depth parity with VS Code/JetBrains/DBeaver is not credible.
- "Unique AI/MCP hub" — VS Code, Zed, Postman and JetBrains all ship agent/MCP features.
- "Unique reproducible environments" — mise[150] and DevPod[152].
- "Unique multilingual/Telex input" — Dodo[37].
- "Permissive open source is unusual" — many competitors are MIT/Apache/MPL.

## Recommendation

Position as the **local-first, single-boundary workstation**: one versioned config and permission/audit model; integrate category leaders; measure performance before claiming it; track Dodo as a standing trigger. Full ranking, per-product evidence and open questions: `research/competitive.md`.

## Sources

[37] https://github.com/MrGru/dodo
[40] https://raw.githubusercontent.com/MrGru/dodo/b7b964d7916c059789cbceefb7d9941569dadd5c//THIRD-PARTY-NOTICES.md
[135] https://github.com/zed-industries/zed
[138] https://github.com/microsoft/vscode
[140] https://www.jetbrains.com/store
[142] https://docker.com/pricing
[146] https://www.postman.com/pricing
[148] https://github.com/Devolutions/UniGetUI
[150] https://mise.jdx.dev
[152] https://devpod.sh/docs/what-is-devpod
[163] https://dbeaver.com
