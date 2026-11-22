# Devdeck Competitive and Market Analysis

**Scope:** research-only. No product code was changed. This report compares the market positioning, licensing, technology stack, maintenance signals and reuse potential of developer tools that overlap with an all-in-one native desktop developer environment. Repository facts are used only as maintenance signals; no source-level audit of any repository was performed (the delegation assigns deeper reviews of specific repositories to other agents).

**Snapshot date:** 2026-11-22. GitHub star counts, push dates and release tags are point-in-time readings from the GitHub REST API, not lifetime trends.

---

## 1. Executive summary

The market is not empty. A native Rust+GPUI desktop app called **Dodo** already packages an HTTP API client, container manager, database manager, diagrams, developer utilities and multilingual input into one desktop application, and it is MIT-licensed.[143][146] Its source repository has 7 GitHub stars, and the showcase manifest contains no release metadata, so it is a proof of concept rather than a credible mass-market competitor today.[144][146]

The strongest incumbents win on **breadth of ecosystem and workflow depth**, not on architecture. VS Code is MIT at the source layer and JetBrains sells deep language intelligence with a single suite licence.[4][7][25] Docker Desktop, DBeaver, Postman and GitKraken own individual workflow categories with mature feature sets and paid tiers.[8][12][13]

The defensible opening is not "native Rust" by itself. It is **one process and one permission/config model across the whole toolchain, with local-first, versioned configuration and genuinely native resource behaviour** — provided Devdeck can demonstrate depth parity in at least two high-frequency categories (terminal/Git and toolchain/environment management) before claiming all-in-one status.

**Top three gaps relative to an all-in-one native app:**

1. **Context switching across separately licensed, separately stateful tools.** Vendors themselves market consolidation: mise promises one `mise.toml` for tools, env vars and tasks; DevPod promises one devcontainer spec across backends and IDEs; Dodo explicitly promises fewer switches between APIs, containers and databases.[16][18][143]
2. **Environment drift and non-reproducible local setups.** mise pins tool versions with a lockfile and bootstraps a machine; DevPod uses `devcontainer.json` and providers to make environments reproducible locally or remotely.[16][18][56]
3. **Cost and licence fragmentation.** Docker Desktop requires a paid subscription above stated thresholds, and JetBrains commercial IDE pricing is per user per year.[9][25] Postman and DBeaver separate free and paid tiers, as does GitKraken.[12][155][13]

Scores (Section 5) put **mise (3.95/5)** and **Dodo (3.75)** at the top of the incumbent field on this weighting, with **VS Code (3.75)** close behind; note that the weighting rewards permissive licensing and native architecture, so it is a strategic lens rather than an objective measure of product quality.

---

## 2. Method and evidence standard

- **Retrieval:** official repositories, official documentation, pricing/licensing pages, official engineering blogs and GitHub/crates.io REST APIs were fetched with `web_search` / `web_extract` / `terminal`-equivalent HTTP calls. Every externally sourced claim carries an inline numbered citation allocated at retrieval time by the grounded-citations ledger at `/root/Devdeck/research/.citations/competitive.json`. The generated Sources block at the end is mechanically rendered from that ledger.
- **Separation of fact and analysis:** Section 3 states sourced facts. Section 4 onward is explicitly labelled analysis, interpretation or recommendation. Where a claim could not be sourced, it is marked `[unverified]` rather than smoothed over.
- **No invented numbers:** no market-size estimates, adoption counts, download numbers or prices are inferred. Prices quoted are those visible on official pages on the snapshot date; static-page extraction sometimes omitted dynamically rendered prices (for example GitKraken's exact Pro figure), in which case the report says so.
- **Maintenance signals:** star count, last push and latest release tag from official APIs. These are signals of activity, not proof of product health, and small projects are penalised deliberately only in the maintenance dimension.
- **Known coverage limits:** the 2025 Stack Overflow Developer Survey technology page was retrieved, but its IDE chart values were not extractable as text, so no survey percentage is used here.[124] No paid analyst data, app-store telemetry or proprietary usage numbers were available.

---

## 3. Verified facts by product

### 3.1 Master fact table

| Product | Stack (verified) | Licence / business model (verified) | Core strength | Gap vs single all-in-one native app | Maintenance signal (2026-11-22) | Reuse potential |
|---|---|---|---|---|---|---|
| **VS Code** | Electron runtime pinned in `package.json`; Code-OSS repo is TypeScript-dominant.[24][58] | Repo MIT; Microsoft binary distribution under a separate product licence.[4][5][23] | Extension ecosystem; debugging, tasks, Git, dev containers; agent workflow.[23][166] | Editor-centric: no first-class DB/Docker/K8s/API-client/token-manager modules; multi-process Electron footprint.[24] | 195,732 stars; pushed 2026-11-22; release 1.147.0 on 2026-11-18.[58][106] | MIT source is reusable, but brand/binary and marketplace are Microsoft-controlled.[4][23] |
| **Zed** | Written from scratch in Rust for multicore and GPU; `gpui` is Apache-2.0.[21][2][153] | Most source GPL-3.0-or-later, Apache-2.0 for reuse-oriented components; distributed app governed by Zed terms; free tier plus Pro $10/mo and Business $30/seat/mo.[2][3][127] | Fast native editor, collaboration, bring-your-own-key AI.[21][127] | Editor-centric; no DB/container/API-client suite; GPL core is a problem for an MIT/Apache-only dependency policy.[1][2] | 95,503 stars; pushed 2026-11-22; v1.30.2-pre on 2026-11-20.[57][105] | GPUI and Apache-marked crates are reusable; GPL core is not.[2][153] |
| **JetBrains IDEs** | IntelliJ community repo is Java-dominant; Kotlin is a supported platform language; commercial IDEs are JVM-based (platform evidence).[59][133] | Proprietary subscription: IntelliJ IDEA $719/user/year, All Products Pack $979/user/year on the official buy page; several IDEs free for non-commercial use.[25][7][85] | Deep semantic refactoring, language-specific IDEs, DB tooling in DataGrip, suite licensing.[7][25] | Multiple separate IDE installs; JVM runtime; per-seat cost; no unified native toolbox across terminal/containers/API/DB.[59][25] | 20,749 stars on community repo; pushed 2026-11-22; release tag 2026-11-19.[59][114] | Closed-source commercial code: effectively no code reuse.[6][25] |
| **Docker Desktop** | Docker Engine (`moby`) is Go/Apache-2.0; Desktop runs a VM (WSL2 on Windows) with graphical resource controls.[60][103] | Desktop is free for personal use, non-commercial OSS and small businesses under stated thresholds (<250 employees AND <$10M revenue); otherwise paid. Personal/Pro/Team/Business listed at $0/$9/$15/$24 per user/month on annual plans.[9][8] | Container runtime, images, Kubernetes, extensions SDK, near-universal workflow adoption.[8][103] | Container-only; VM+daemon architecture; licence sensitivity in larger organisations.[9][27] | Engine repo 72,304 stars, pushed 2026-11-20; Desktop UI itself is not open source in the same repo.[60] | Engine/CLI reusable (Apache-2.0); Desktop UI and brand are not.[60][9] |
| **DBeaver** | Written mostly in Java with native OS components; Eclipse RCP UI; OpenJDK 25 included in distributions.[145] | Community edition Apache-2.0; paid PRO/Enterprise/Lite editions listed from $113/year on the official edition page.[10][11][155] | 100+ database drivers, JDBC/ODBC coverage, SQL/ER/data tooling, AI chat.[145] | Database-only; JVM/Eclipse footprint; driver/plugin complexity.[145] | 52,508 stars; pushed 2026-11-22; release 26.2.5 on 2026-11-15.[62][107] | Apache-2.0 community code is a high-value reuse source for DB UI/connectors.[11][145] |
| **Postman** | Desktop app runs on Electron.[91] | Proprietary freemium: Free, Solo $9/mo annual ($12 monthly), Team $19/user/mo annual ($23 monthly), Enterprise; plans changed in March 2026.[12][168] | API collections, environments, flows, mocks, multi-protocol client, MCP client, AI credits.[12] | API-only; cloud-account gravity; paid tiers and AI overage metering; Electron footprint.[12][168][91] | Public support repo exists but product is closed source; 6,029 stars on the support repo.[63] | No code reuse; format/concept interoperability (collections, environments) only.[12] |
| **GitKraken** | Desktop is Electron-based; migrated from NodeGit to core Git in v12.7, a five-year effort.[97][99] | Proprietary freemium: Basic free, paid Pro/Advanced/Business/Enterprise per seat; exact Pro price is rendered dynamically and was not captured statically.[13][165][167] | Best-in-class Git visualisation, workspaces, GitLens/CLI/MCP integration.[13][165] | Git-only; Electron; per-seat licensing; performance work needed even on mature hardware.[97][99] | Active 12.7 release with public engineering blog (2026-11).[99] | No code reuse; Git-visualisation patterns only.[99] |
| **UniGetUI** | C#/.NET; repository contains `UniGetUI.Avalonia` project and a self-contained NativeAOT build path; GitHub language is C#.[151][55][64] | MIT; free and open source, now maintained by Devolutions after 2026 stewardship announcement.[15][55][154] | One GUI over WinGet, Scoop, Chocolatey, Winget, pip, npm and other managers; installer/update workflows.[55] | Package management only; Windows-first despite cross-platform builds; not Rust.[55][64] | 27,199 stars; pushed 2026-11-21; release v2026.3.4 on 2026-11-13.[64][109] | MIT code and package-manager adapter patterns are directly reusable.[15][55] |
| **mise** | Rust CLI, MIT; `mise.toml` covers tools, env vars and tasks, with a lockfile and machine bootstrap.[16][54] | MIT, open source, free.[16][65] | Version manager for 1,000+ tools; directory-based switching; cross-shell/editor/CI consistency; bootstrap.[16] | CLI-first, no GUI or broader toolbox; toolchain management only.[16] | 36,385 stars; pushed 2026-11-22; release v2026.11.12 on 2026-11-21.[65][108] | Excellent: MIT Rust code and the config/lockfile model can be embedded or mirrored.[16][65] |
| **DevPod** | Go CLI/desktop client; each environment is a container defined by `devcontainer.json`; provider model for local/cloud backends.[18][56][66] | MPL-2.0; client-only, no hosted backend required.[17][18][66] | Client-side reproducible dev environments; cross-IDE (VS Code, JetBrains, SSH); prebuilds and credential sync.[18][56] | Environment provisioning only; not a desktop IDE/toolbox; MPL is outside Devdeck's MIT/Apache-only dependency rule.[18][17] | 15,338 stars but last push 2025-11-14 and latest release v0.7.0-alpha.34 from 2025-06-23 — the weakest momentum signal in this set.[66][110] | Concept reuse (devcontainer spec, provider abstraction); MPL code needs care.[17][18] |
| **Dodo (GPUI Kit showcase)** | Native desktop toolbox built in Rust with GPUI Kit; source repo `MrGru/dodo`. It is unrelated to the Dodo systems language and to Dodo Payments.[143][144][39][52] | MIT (source repo).[146] | HTTP API explorer, containers, PostgreSQL/MySQL/MariaDB/Redis/SQLite, diagrams, developer utilities, Vietnamese Telex input.[143][47] | 7 stars, one showcase entry, no release metadata in the manifest; no Git editor, terminal, toolchain manager, extension system or evidence of production use.[144][146] | Pushed 2026-11-16, but 7 stars as of 2026-10-04 in showcase manifest.[146][144] | High technical reuse potential (MIT Rust/GPUI) but almost no proven product surface.[143][146] |
| **DevCat** | Rust CLI for local snapshots, diffing, auto-watch and context export; MIT.[19][53][67] | MIT, free, open source.[67] | Fast local experiment state without polluting Git history.[19] | Single narrow utility; 1 star; release v0.1.1 from 2025-12-25.[67][111] | Pushed 2026-10-27, 1 star.[67] | Small MIT Rust crate; snapshot/context ideas could be integrated.[19] |

### 3.2 Native Rust adjacencies discovered during research

These are not all-in-one apps, but they narrow the claim that "native Rust desktop developer tooling" is empty space. All figures are from the GitHub REST API on the snapshot date.

| Tool | Category | Stack / licence | Stars / last push | Relevance |
|---|---|---|---|---|
| Helix | Modal text editor | Rust, MPL-2.0.[68] | 46,960 / 2026-11-20.[68] | Native editor alternative; MPL is not MIT/Apache-compatible as a copied dependency. |
| Lapce | Code editor | Rust, Apache-2.0.[69] | 39,098 / 2026-11-18.[69] | Direct native-editor precedent; permissive reuse candidate. |
| GPUI Kit | Rust desktop UI framework | Rust, Apache-2.0, 75+ components, 120 FPS vendor claim.[115][147] | 21,179 / 2026-11-22; v0.10.0 on 2026-11-16.[75][113] | Makes native Rust UI far more achievable; also the foundation of Dodo. |
| Zellij | Terminal multiplexer | Rust, MIT.[70] | 36,183 / 2026-11-20.[70] | Terminal layer precedent. |
| GitUI | Terminal Git UI | Rust, MIT.[71] | 22,679 / 2026-10-06.[71] | Terminal Git layer precedent. |
| Atuin | Shell history sync | Rust, MIT.[72] | 32,559 / 2026-11-21.[72] | Local-first sync and shell integration precedent. |
| Starship | Prompt | Rust, ISC.[73] | 60,843 / 2026-11-22.[73] | Large-scale Rust developer utility precedent. |
| Nushell | Shell | Rust, MIT.[74] | 41,061 / 2026-11-22.[74] | Shell/pipeline precedent. |

---

## 4. Analysis: developer pain points and what is already solved

Framing evidence comes from vendors' own positioning. Dodo says it reduces switching between APIs, containers, databases and utilities, while mise says one file handles tools, env vars and tasks.[143][16] DevPod says one devcontainer spec works across backends, and JetBrains pitches one subscription instead of separate licences.[18][7]

| Pain point | Evidence that it is real (or at least market-recognised) | Already solved? | Remaining opening for Devdeck |
|---|---|---|---|
| Tool sprawl / context switching | Vendors explicitly sell consolidation: Dodo, UniGetUI, mise, JetBrains All Products.[143][55][16][7] | Partly, inside categories (JetBrains for IDEs; Dodo for a demo toolbox). No incumbent has solved the whole P0 list in one native process.[7][143] | One tab/command palette/permission model across terminal, Git, API, DB, Docker, toolchains, secrets. |
| Environment drift | mise pins versions with `mise.lock` and bootstraps machines; DevPod standardises containers across IDEs/backends.[16][18] | Largely solved by mise + DevPod + devcontainers for teams that adopt them.[16][18] | Integrate, do not reinvent: wrap these tools as first-class providers under a single UI. |
| Licensing and cost | Docker Desktop thresholds and paid tiers; JetBrains per-seat pricing; Postman/DBeaver/GitKraken tiering.[9][25][12][155][13] | Partly: free tiers exist, but multi-vendor procurement remains fragmented.[8][85][13] | Predictable single licence (or free/open core) with no per-tool seat multiplication — but this is a business model claim, not a proven demand metric. |
| Resource use | Zed markets Rust/GPU from scratch; GitKraken spent five years on internals and two releases on Electron performance; Dodo markets "native".[21][99][97][143] | Native editors (Zed, Helix, Lapce) address editor cost; no independent benchmark in this report proves an all-in-one app is lighter.[21][68][69] | Only defensible with published, reproducible benchmarks against real workflows (see Kill list). |
| AI-agent fragmentation | VS Code open-sourced Copilot Chat and is moving components into core; Zed supports external agents and hosted models; Postman ships an MCP client; GitKraken ships an MCP server; JetBrains supports multiple agents/models.[23][127][12][13][7] | Heavily contested; "agent hub" is not white space.[23][127][12] | differentiation must be local permissioning/audit/checkpointing, not "we have an agent panel". |
| Localisation / multilingual input | Dodo already advertises language switching and Vietnamese Telex input; JetBrains and UniGetUI ship multi-language UIs.[143][55] | Not a differentiator by itself.[143] | Only if Devdeck's locale pipeline is demonstrably better (runtime switch, RTL/IME, full coverage). |

**Interpretation (analysis, not sourced fact):** the incumbents' genuine moat is depth and installed workflows, not licensing or language choice. A newcomer that merely re-hosts existing capabilities in one window inherits the integration cost without inheriting the incumbents' feature completeness.

---

## 5. Scored comparison (computed, not mental arithmetic)

Weights sum to 1.00 in code: native/resource efficiency 20%, all-in-one breadth 25%, ecosystem 15%, licence/cost accessibility 15%, maintenance momentum 15%, reuse compatibility with an MIT/Apache-only Rust project 10%. Scores are 1–5; the weighted total is computed with Python from these weights. Scoring is an explicit analytical model, not a sourced fact.

| Rank | Product | Native/eff (20%) | Breadth (25%) | Eco (15%) | Cost (15%) | Maint (15%) | Reuse (10%) | Weighted /5 |
|---|---|---|---|---|---|---|---|---|
| 1 | mise | 5 | 2 | 3 | 5 | 5 | 5 | 3.95 |
| 2 | Dodo (GPUI) | 5 | 4 | 1 | 5 | 3 | 4 | 3.75 |
| 3 | VS Code | 1 | 4 | 5 | 5 | 5 | 3 | 3.75 |
| 4 | Zed | 5 | 2 | 3 | 4 | 5 | 2 | 3.50 |
| 5 | UniGetUI | 2 | 2 | 3 | 5 | 4 | 4 | 3.10 |
| 6 | JetBrains IDEs | 1 | 4 | 4 | 2 | 5 | 1 | 2.95 |
| 7 | DBeaver | 1 | 2 | 4 | 4 | 5 | 3 | 2.95 |
| 8 | DevCat | 5 | 1 | 1 | 5 | 2 | 4 | 2.85 |
| 9 | Docker Desktop | 2 | 2 | 4 | 2 | 5 | 1 | 2.65 |
| 10 | Postman | 1 | 2 | 4 | 2 | 5 | 1 | 2.45 |
| 11 | GitKraken | 1 | 2 | 3 | 3 | 5 | 1 | 2.45 |
| 12 | DevPod | 1 | 2 | 3 | 4 | 1 | 2 | 2.10 |

For reference only, the Devdeck target described in the local vision document scores 3.95 on the same model (native 5, breadth 5, ecosystem 2, cost 5, maintenance 1, reuse 5) — it is excluded from the ranking because none of it is shipping yet. The score is identical to mise's, which is the point: a plan is not a product, and mise already delivers a large part of the environment-management promise.[16]

**How to read this:** the ranking rewards breadth heavily but cannot reward unproven execution. Dodo's high score reflects feature-list breadth and licensing, not adoption (7 stars).[144][146] VS Code's low "native" score is not a quality judgement; its Electron runtime is documented in the repository build config.[24] DevPod's 1 in maintenance is driven by a last push in 2025 and an alpha release in mid-2025.[66][110]

---

## 6. Kill list: differentiators that are NOT defensible

Each entry is a claim to drop from marketing/positioning unless new evidence appears.

1. **"The only native Rust all-in-one developer app."** Dodo is a native Rust+GPUI desktop toolbox with API, container, database, diagram and utility modules, MIT-licensed.[143][146] Zed, Helix, Lapce, Zellij, GitUI and Nushell are all mature native Rust developer tools.[21][68][69][70][71][74] Narrow the claim to a specific verified combination, not the category.
2. **"Native means dramatically faster than Electron, by construction."** No independent benchmark in this research compares the candidate apps. GitKraken has shipped measurable Electron performance improvements and a core-Git migration, and Zed's speed claims are vendor-authored.[97][99][21] Without reproducible numbers, this is marketing, not differentiation.
3. **"We replace the whole toolchain."** VS Code alone covers editing, debugging, tasks, Git and dev containers, and JetBrains sells ten IDEs plus profilers in one pack.[23][7] DBeaver supports 100+ database drivers, while Postman and Docker own their categories.[145][12][8] Claim integration with best-of-breed tools first, replacement second.
4. **"Unique AI agent hub / MCP support."** VS Code is open-sourcing Copilot Chat and Zed supports external agents and hosted models.[23][127] Postman ships an MCP client, GitKraken an MCP server, and JetBrains lets users bring their own agents and models.[12][13][7] The differentiator would have to be local policy, audit and checkpointing — not presence of an agent panel.
5. **"Solves tool sprawl / context switching for everyone."** Vendors already sell consolidation inside categories and the problem is real, but no evidence here shows users will abandon VS Code or JetBrains for a less mature environment.[143][16][7] Position as reduction of *state* switching (one config, one permission boundary), not as the death of all other apps.
6. **"Unique reproducible environments."** mise and DevPod already do this with lockfiles and devcontainers.[16][18] Devdeck should integrate them rather than claim novelty.
7. **"Unique multilingual / Vietnamese input support."** Dodo explicitly advertises language switching and Vietnamese Telex input; UniGetUI and JetBrains localise their UIs.[143][55] Drop this as a headline differentiator.
8. **"Permissive open source is unusual."** mise, UniGetUI, Dodo, Lapce and Zellij are all permissively licensed.[16][15][146][69][70] GitUI and Atuin are MIT, and VS Code's Code-OSS core is MIT.[71][72][4] MIT/Apache is a development constraint, not a market differentiator.
9. **"Low price / free as a moat."** Docker Personal, JetBrains non-commercial licences and GitKraken Basic already cover the free end.[9][85][13] Postman Free and DBeaver Community do the same in their categories.[12][10] Price parity is table stakes.
10. **"We will beat Docker Desktop because it is licensed."** Docker's licence thresholds are real, but the engine is Apache-2.0 and the workflow is entrenched; a licence grievance is not a migration reason on its own.[9][60][27]

---

## 7. Differentiation recommendation

**Recommendation: position Devdeck as the local-first, single-boundary developer workstation — not as a faster editor or a cheaper IDE suite.** Build the wedge where the incumbents are structurally split:

1. **One config, one permission boundary, one audit trail.** Ship a versioned, Git-friendly `~/.devdeck` config that declares tools, secrets, env vars, MCP agents and extensions, with deny-by-default capability scopes. mise already proves the one-file model for versions/tasks, and DevPod proves the provider model; Devdeck's new contribution is unifying those with secrets/agent policy in a GUI and a CLI.[16][18]
2. **Integrate, do not replace, category leaders.** First-class adapters for `mise` (toolchains), devcontainers/DevPod (environments) and Git core (following GitKraken's own migration) come first.[16][18][99] Docker Engine/CLI is Apache-2.0, DBeaver community code is a useful DB reference, and Postman demonstrates MCP client patterns.[60][145][12]
3. **Prove the resource claim or remove it.** Publish a benchmark harness measuring cold start, idle RSS, keystroke latency and a standard "open repo → run tests → query DB → inspect container" workflow against VS Code, Zed and a stock terminal+CLI stack. The native architecture is an enabler; measured numbers are the differentiator.[21][97]
4. **Watch Dodo specifically.** It is the closest architectural and feature analogue and is MIT-licensed; if it gains adoption, the "native all-in-one" framing collapses. Track its stars, releases and feature additions as a standing competitive trigger.[143][146][144]
5. **Do not compete on depth first.** Terminal, Git and toolchain management are enough for a credible P0.[16][18] API client, DB manager and Docker can start as adapters, referencing existing engines and drivers.[60][145]

**Open questions for follow-up research:** exact current prices for GitKraken Desktop tiers are dynamically rendered, and independent RAM/startup benchmarks for the named apps are missing.[167] It is unknown whether Dodo has published releases or users beyond the 7-star showcase entry, and whether UniGetUI's Avalonia/NativeAOT migration is complete across all platforms.[144][151] User-sourced evidence of willingness to consolidate tools is also missing; the 2025 Stack Overflow IDE chart was not machine-readable in this pass.[124]

---

## Sources

[1] https://github.com/zed-industries/zed — Zed GitHub repository
[2] https://zed.dev/software-overview — Zed software overview
[3] https://github.com/zed-industries/zed/blob/main/LICENSE-GPL — Zed GPL license
[4] https://github.com/microsoft/vscode — VS Code GitHub repository
[5] https://github.com/microsoft/vscode/blob/main/LICENSE.txt — VS Code license
[6] https://www.jetbrains.com/store — JetBrains store pricing
[7] https://www.jetbrains.com/all — JetBrains All Products Pack
[8] https://docker.com/pricing — Docker pricing
[9] https://docs.docker.com/subscription/desktop-license — Docker Desktop license
[10] https://dbeaver.io/about — DBeaver about
[11] https://github.com/dbeaver/dbeaver/blob/HEAD/LICENSE.md — DBeaver license
[12] https://www.postman.com/pricing — Postman pricing
[13] https://dev.gitkraken.com/pricing — GitKraken pricing
[15] https://github.com/Devolutions/UniGetUI/blob/HEAD/LICENSE — UniGetUI license
[16] https://mise.jdx.dev — mise official site
[17] https://github.com/loft-sh/devpod/blob/main/LICENSE — DevPod license
[18] https://devpod.sh/docs/what-is-devpod — DevPod docs what is
[19] https://github.com/gni/devcat — DevCat GitHub
[21] https://zed.dev — Zed official homepage
[23] https://code.visualstudio.com/docs/supporting/faq — VS Code official FAQ
[24] https://raw.githubusercontent.com/microsoft/vscode/main/package.json — VS Code package.json
[25] https://www.jetbrains.com/idea/buy — IntelliJ IDEA buy page
[27] https://www.docker.com/pricing/faq — Docker pricing FAQ
[39] https://github.com/MrGru/dodo — MrGru/dodo
[47] https://gpui-kit.com/apps/dodo — Dodo — GPUI Kit
[52] https://github.com/Jotrorox/dodo
[53] https://raw.githubusercontent.com/gni/devcat/main/README.md
[54] https://raw.githubusercontent.com/jdx/mise/main/README.md
[55] https://raw.githubusercontent.com/Devolutions/UniGetUI/main/README.md
[56] https://raw.githubusercontent.com/loft-sh/devpod/main/README.md
[57] https://api.github.com/repos/zed-industries/zed — GitHub API metadata zed-industries/zed
[58] https://api.github.com/repos/microsoft/vscode — GitHub API metadata microsoft/vscode
[59] https://api.github.com/repos/JetBrains/intellij-community — GitHub API metadata JetBrains/intellij-community
[60] https://api.github.com/repos/docker/docker — GitHub API metadata docker/docker
[62] https://api.github.com/repos/dbeaver/dbeaver — GitHub API metadata dbeaver/dbeaver
[63] https://api.github.com/repos/postmanlabs/postman-app-support — GitHub API metadata postmanlabs/postman-app-support
[64] https://api.github.com/repos/Devolutions/UniGetUI — GitHub API metadata Devolutions/UniGetUI
[65] https://api.github.com/repos/jdx/mise — GitHub API metadata jdx/mise
[66] https://api.github.com/repos/loft-sh/devpod — GitHub API metadata loft-sh/devpod
[67] https://api.github.com/repos/gni/devcat — GitHub API metadata gni/devcat
[68] https://api.github.com/repos/helix-editor/helix — GitHub API metadata helix-editor/helix
[69] https://api.github.com/repos/lapce/lapce — GitHub API metadata lapce/lapce
[70] https://api.github.com/repos/zellij-org/zellij — GitHub API metadata zellij-org/zellij
[71] https://api.github.com/repos/extrawurst/gitui — GitHub API metadata extrawurst/gitui
[72] https://api.github.com/repos/atuinsh/atuin — GitHub API metadata atuinsh/atuin
[73] https://api.github.com/repos/starship/starship — GitHub API metadata starship/starship
[74] https://api.github.com/repos/nushell/nushell — GitHub API metadata nushell/nushell
[75] https://api.github.com/repos/longbridge/gpui-kit — GitHub API metadata longbridge/gpui-kit
[85] https://blog.jetbrains.com/blog/2024/10/24/webstorm-and-rider-are-now-free-for-non-commercial-use — WebStorm and Rider Are Now Free for Non-Commercial Use - The JetBrains Blog
[91] https://blog.postman.com/going-native — Going native
[97] https://www.gitkraken.com/blog/gitkraken-client-v8-3 — GitKraken Client v8.3 Release - Now 2x Faster for Apple Silicon Users
[99] https://www.gitkraken.com/blog/farewell-nodegit-from-gitkraken-desktop — Farewell, NodeGit: GitKraken Desktop Now Runs on Core Git
[103] https://www.docker.com/blog/the-magic-behind-the-scenes-of-docker-desktop — The Magic Behind the Scenes of Docker Desktop | Docker
[105] https://api.github.com/repos/zed-industries/zed/releases?per_page=1 — Latest GitHub release zed-industries/zed
[106] https://api.github.com/repos/microsoft/vscode/releases?per_page=1 — Latest GitHub release microsoft/vscode
[107] https://api.github.com/repos/dbeaver/dbeaver/releases?per_page=1 — Latest GitHub release dbeaver/dbeaver
[108] https://api.github.com/repos/jdx/mise/releases?per_page=1 — Latest GitHub release jdx/mise
[109] https://api.github.com/repos/Devolutions/UniGetUI/releases?per_page=1 — Latest GitHub release Devolutions/UniGetUI
[110] https://api.github.com/repos/loft-sh/devpod/releases?per_page=1 — Latest GitHub release loft-sh/devpod
[111] https://api.github.com/repos/gni/devcat/releases?per_page=1 — Latest GitHub release gni/devcat
[113] https://api.github.com/repos/longbridge/gpui-kit/releases?per_page=1 — Latest GitHub release longbridge/gpui-kit
[114] https://api.github.com/repos/JetBrains/intellij-community/releases?per_page=1 — Latest GitHub release JetBrains/intellij-community
[115] https://raw.githubusercontent.com/longbridge/gpui-kit/main/README.md
[124] https://survey.stackoverflow.co/2025/technology — Technology | 2025 Stack Overflow Developer Survey
[127] https://zed.dev/pricing — Pricing
[133] https://plugins.jetbrains.com/docs/intellij/using-kotlin.html — Configuring Kotlin Support | IntelliJ Platform Plugin SDK
[143] https://raw.githubusercontent.com/longbridge/gpui-kit-showcases/main/apps/dodo/README.md
[144] https://raw.githubusercontent.com/longbridge/gpui-kit-showcases/main/apps/dodo/manifest.json
[145] https://raw.githubusercontent.com/dbeaver/dbeaver/devel/README.md
[146] https://api.github.com/repos/MrGru/dodo — API https://api.github.com/repos/MrGru/dodo
[147] https://crates.io/api/v1/crates/gpui-kit — API https://crates.io/api/v1/crates/gpui-kit
[151] https://raw.githubusercontent.com/Devolutions/UniGetUI/main/src/UniGetUI.Avalonia/UniGetUI.Avalonia.csproj
[153] https://raw.githubusercontent.com/zed-industries/zed/main/crates/gpui/Cargo.toml
[154] https://devolutions.net/blog/2026/03/unigetui-enters-its-next-chapter-with-devolutions
[155] https://dbeaver.com/edition — Compare DBeaver Products & Pricing - DBeaver Enterprise, Lite
[165] https://help.gitkraken.com/gk-dev/gk-dev-subscription
[166] https://code.visualstudio.com
[167] https://www.gitkraken.com/pricing-c
[168] https://learning.postman.com/docs/billing/about-plans — Postman plans documentation
