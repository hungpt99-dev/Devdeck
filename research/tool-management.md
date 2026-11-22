# Unified Developer Tool Manager — Adapter Survey and Orchestration Design

**Scope.** Research-only report for Devdeck. Nothing in the "Proposed design" sections is implemented; it is a specification for a future `crates/tools`/`crates/toolchain` feature. External-tool facts are marked **[Observed]** and sourced to official documentation. Devdeck design is marked **[Proposed]** and is derived from the project's own `VISION.md`/`AGENTS.md` constraints (app-owned `~/.devdeck` directory, no permanent PATH changes without consent, MIT/Apache-2.0-compatible dependencies, local-first config, atomic writes). No product code was written.

---

## 1. Executive summary

The differentiator is not a new package manager; it is a **manager-of-managers** that presents one declarative intent ("I need Node 22, PostgreSQL 16, and the Rust analyzer") and delegates to the native manager already present on the machine. That framing mirrors the existing precedent: UniGetUI describes itself as a graphical "package manager manager" over WinGet/Chocolatey/Scoop and more [35][36]. **[Observed]**

The hard parts are (a) provenance — knowing *which* manager owns a binary and *how* it was verified, (b) PATH/config conflicts when several managers shim the same executable, (c) offline reproducibility, and (d) refusing to act when a native manager would prompt or escalate. Native managers already solve download, checksum/signature verification, dependency resolution, local caching, and uninstall; reimplementing those would violate the "do not reinvent" requirement and the dependency-license policy.

**Key licensing constraint discovered:** several adapters are *not* MIT/Apache-compatible — cargo-binstall is GPLv3 [38], the npm CLI is Artistic License 2.0 [81], and pnpm has a PolyForm Shield source-available directory [72]. Under Devdeck's own dependency policy these can only be invoked as **external processes**, never vendored or linked. Chocolatey CLI [79], Homebrew [62] and Scoop [78] are Apache-2.0, BSD-2-Clause and Unlicense/MIT respectively. mise [65], asdf [66] and aqua [80] are MIT. proto [68] is MIT as well. Cargo [77], Maven [75] and Gradle [76] are MIT/Apache-2.0-compatible. uv [69], pip [74] and winget-cli [70] are dual-licensed or MIT-compatible. **[Observed]**

---

## 2. Adapter survey **[Observed]**

Legend for columns: **NI** = non-interactive/automation; **Provenance** = checksum/signature support; **Offline** = cache/offline install behavior; **CLI stability** = documented promise or practical contract.

### 2.1 Windows system managers

| Adapter | What it does / cannot do | License | CLI stability | Cross-platform | NI | Provenance | Offline |
|---|---|---|---|---|---|---|---|
| **winget** (Windows Package Manager) | Discover, install, upgrade, remove, configure apps on Windows 10/11/Server 2025; client to the Windows Package Manager service; ships with App Installer, requires Win10 1809+ [1]. Manifests describe installers; cannot force install of arbitrary unsigned artifacts outside manifest rules. | winget-cli MIT [70] | Documented CLI with `--exact`, `--silent`, `--disable-interactivity`, `--accept-package-agreements`, `--accept-source-agreements`; substring match by default [2][5] | Windows only [1] | Strong: explicit flags for scripted installs, agreements, silent mode [2] | Manifest fields `InstallerSha256`, optional `SignatureSha256` (MSIX); `DownloadCommandProhibited` blocks offline download [4] | `winget download` for offline install unless prohibited by manifest [4]; settings export/import via `winget settings` subcommands [3] |
| **Scoop** | Command-line installer that avoids UAC prompts, hides wizard GUIs, avoids PATH pollution and uninstall side effects [63]. Apps are JSON manifests with `version`, `url`, `extract_dir`, `bin`, optional `hash`, `autoupdate`, architecture-specific blocks [6]. Buckets are Git repos of manifests (main, extras, versions; custom buckets allowed) [7]. Cannot manage system-wide MSI-style installs in the same way winget/Chocolatey do. | Unlicense OR MIT [78] | Manifest schema + wiki; community bucket changes can break pins | Windows only [63] | Good — manifest-driven, shims generated automatically [6] | Manifest `hash`/`hash_type`, `autoupdate` with hash checking (manifest doc lists hash in architecture blocks) [6] | Downloads to Scoop cache; bucket + manifest are Git clones, so pinned buckets are locally reproducible [7] |
| **Chocolatey** | `choco` is "like yum or apt-get, but for Windows" [64]. Community repository by default, switchable to internal repos; client is open source, release binaries and PowerShell scripts are Authenticode-signed, VirusTotal-checked, no telemetry, works completely offline against internal repos [9]. `Get-ChecksumValid` compares a downloaded file against an expected checksum and throws on mismatch [8]. `choco download` and Package Internalizer (which recompiles packages to use internal resources for no-internet installs) are **Licensed Editions / C4B-MSP only** [10]. | Apache-2.0 [79] | Stable config model (`choco config set/unset`, features) [11] | Windows (source build possible elsewhere [64]) | Good; batch scripting supported | Checksums via `Get-ChecksumValid` [8]; Authenticode-signed client artifacts [9] | Offline by design with internal repositories [9]; internalizer is commercial [10] |

### 2.2 macOS / Linux system managers

| Adapter | What it does / cannot do | License | CLI stability | Cross-platform | NI | Provenance | Offline |
|---|---|---|---|---|---|---|---|
| **Homebrew** | Installs UNIX tools on macOS and Linux without sudo for formulae; default prefixes `/opt/homebrew` (Apple Silicon) and `/home/linuxbrew/.linuxbrew` (Linux); some casks/services still need elevation [13]. `brew bundle` + `Brewfile` give a declarative "desired state" interface and can also start services via `brew services` [12]. Bottles are binary packages [13]. Running `brew` with sudo is explicitly discouraged because it trusts the prefix owner and can execute arbitrary code as root [14]. | BSD-2-Clause [62] | Documented manpage and declarative Brewfile [12][14] | macOS + Linux [13] | Strong: `brew bundle` is designed for automation [12] | Bottle checksums; formulae/taps are Git-backed; env vars such as `HOMEBREW_NO_AUTO_UPDATE` are defined in `env_config.rb` [15] | Local Cellar/cache + bottles; Brewfile pins desired state [12] |
| **APT / dpkg** | `apt-get`/`apt` are the advanced interface to Debian packaging; `apt` offers equivalents of common `apt-get`/`apt-cache` commands [17]. Package authenticity: signed `Release`/`Packages` indexes with SHA256 hashes chain trust to package contents [16]. | OS component (Debian) | Very stable | Debian/Ubuntu family only | `apt-get` non-interactive with front-end flags; `DEBIAN_FRONTEND` | Repository signature + index SHA256 chain [16] | `apt-offline` downloads packages/dependencies on a connected host and installs offline, with `--strict-deb-check` delegating `.deb` checksum validation to apt [18] |
| **DNF / RPM** | "Next major version of YUM", CLI-compatible with YUM, strict API for extensions/plugins [19]. `offline-upgrade` is provided by `dnf-plugins-core` [19]. Repository/main configuration governs verification and caching behavior [20]. Fedora signs RPM packages with GPG keys distributed out-of-band, including via the `fedora-gpg-keys` package [61]. | OS component (GPL family) | Documented command and configuration references [19][20] | RPM-family Linux only | Good: batch flags, `dnf download`/`offline-upgrade` [19] | GPG-signed RPMs [61] | `dnf download` + local cache; `offline-upgrade` [19] |
| **pacman** | Package manager utility tracking installed packages with dependency support, groups, install/uninstall scripts and remote sync; packages are zipped tarballs built on libalpm [21]. `--noconfirm` bypasses prompts (documented as intended for scripts) [21]. `--cachedir` defaults to `/var/cache/pacman/pkg` and supports multiple ordered caches [21]. | OS component (GPL-2.0-or-later) | Manpage contract [21] | Arch family only | `--noconfirm` for scripting [21] | OpenPGP web-of-trust; `SigLevel` in `pacman.conf` decides required trust, default `Required DatabaseOptional`, all official packages signed [22] | Package cache directory (`/var/cache/pacman/pkg`) supports offline reinstall [21] |

### 2.3 Language package managers

| Adapter | What it does / cannot do | License | CLI stability | Cross-platform | NI | Provenance | Offline |
|---|---|---|---|---|---|---|---|
| **npm** | `npm ci` is the clean, automation-oriented install: requires an existing `package-lock.json`/`npm-shrinkwrap.json`, errors if lock and manifest disagree, installs whole projects only [44]. `npm cache` add/list/clean/verify [45]. Config comes from CLI flags, env, files with documented priority [46]. | npm CLI: Artistic License 2.0 [81] | Versioned CLI docs | Windows/macOS/Linux | Strong (`npm ci`) [44] | Lockfile integrity hashes | Local content-addressable cache; `npm cache` [45] |
| **pnpm** | `pnpm install --offline` uses only store contents and fails on a miss; `--frozen-lockfile` forbids lockfile updates (CI default when a lockfile exists); `--dry-run` previews [48]. `pnpm store status` verifies installed packages are unmodified and reports `ERR_PNPM_MODIFIED_DEPENDENCY` [47]. | MIT, except `pnpr/` directory under PolyForm Shield [72] | Versioned CLI docs | Cross-platform | Strong [48] | Lockfile + store status verification [47] | Content-addressed store, `--offline` [48] |
| **Yarn (Berry)** | `yarn install --immutable` for CI; `--immutable-cache`; `--check-cache` refetches and ensures checksums match the lockfile and cached files — recommended for Zero-Installs with third-party PRs [49]. | BSD-2-Clause [73] | Versioned CLI docs | Cross-platform | Strong [49] | Lockfile checksums + `--check-cache` [49] | Zero-Installs / checked-in cache [49] |
| **Cargo / crates.io** | `Cargo.toml` describes broad dependencies; `Cargo.lock` contains exact information, is maintained by Cargo and should be committed for applications [52]. Registry index defines `config.json` with `dl`/`api` endpoints and an append-only index format [53]. | Cargo: MIT OR Apache-2.0 [77] | Very stable (lockfile contract) | Cross-platform | `cargo --locked/--offline` | `Cargo.lock` checksums + index `cksum` [52][53] | Local registry cache + `cargo vendor` [52][53] |
| **cargo-binstall** | Installs Rust binaries from prebuilt CI artifacts: reads crate info from crates.io, searches the linked repository for matching releases, falls back to quickinstall [37]. Configured via `[package.metadata.binstall]` and `--manifest-path` [40]. Signature support requires explicit Cargo.toml metadata — **no autodiscovery** [39]. Telemetry is on by default for some strategies, disabled with `--disable-telemetry` [38]. | **GPLv3** — external process only [38] | Documented CLI; artifact layouts vary by crate [37][40] | Cross-platform targets [37] | Good (CI-friendly) | Optional signature metadata only [39] | Uses prebuilt binaries; no general offline store documented |
| **pip** | By default pip does **no** tamper checking and runs arbitrary code from distributions; secure mode requires `--require-hashes` plus `--only-binary :all:` [50]. `pip download`/`pip wheel` stage artifacts for offline use [51]. | MIT [74] | Stable CLI | Cross-platform | Good | Explicit hash-checking mode [50] | Wheel cache; `pip download` [51] |
| **uv** | Extremely fast Python package manager with `uv lock`, `uv sync`, `uv tool`, `uv python`, `uv audit` [42]. `--offline` disables network entirely and uses only local cache/files (`UV_OFFLINE`) [42]. Aggressive cache respects HTTP headers for registry/direct-URL deps, Git commit hashes for Git deps, mtimes for local deps [41]. Private indexes via `[[tool.uv.index]]`/`--index` [43]. | Apache-2.0 OR MIT [69] | Fast-moving but documented CLI [42] | Cross-platform | Excellent (`--offline`, lock) [42] | Lockfile + resolved Git commit hashes [41] | Strong, first-class offline mode [41][42] |
| **Maven** | Local repository is a directory on the machine that caches remote downloads; remote repositories include `file://` and `https://` [55]. Resolver checksums sit next to artifacts (SHA-1/MD5 by default) but **only provide integrity verification — they do not establish trust or protect against MITM/supply-chain attacks**; "trusted checksums" are a separate mechanism [54]. CLI: `-o/--offline`, `-B/--batch-mode`, `-C/--strict-checksums`, `-c/--lax-checksums` [56]. | Apache-2.0 [75] | Very stable | Cross-platform (JVM) | Strong [56] | Repository checksums are integrity-only; use trusted checksums [54] | Local repo + `-o` [55][56] |
| **Gradle** | Local cache (e.g. `~/.gradle/caches/modules-2`); `--offline` uses only cached artifacts; `--refresh-dependencies` bypasses cache and uses hashes to avoid redundant downloads [57]. Dependency verification/locking are separate documented features; wrapper is strongly recommended for reproducibility [58]. | Apache-2.0 [76] | Stable; wrapper pins the version | Cross-platform (JVM) | Strong [57][58] | Dependency verification / checksum-based storage [57] | `--offline` + portable cache [57] |

### 2.4 Version managers and orchestrators

| Adapter | What it does / cannot do | License | CLI stability | Cross-platform | NI | Provenance | Offline |
|---|---|---|---|---|---|---|---|
| **mise** | Installed via script to `~/.local/bin/mise`; project-local `mise.toml`; `mise install` in a project; has a **trust** step for project config [23]. Version requests (`exact`, prefix, `latest`, alias) resolve to concrete versions and can be recorded in a lockfile for reproducibility [25]. | MIT [65] | Fast-moving; documented version-request grammar [25] | macOS/Linux (and Windows support in the project) | Good | Lockfile pins resolved builds [25] | Installed versions under user data dir; lockfile for reproducibility [25] |
| **asdf** | Extendable version manager installed via package manager or precompiled binary; primarily needs `git` [26]. `.tool-versions` is shareable; `.asdfrc` and env vars are user-specific; version forms include exact, `ref:` (build from VCS) and `path:` (custom build) [27]. `asdf install/list` commands [28]. Plugins are the extension unit, so provenance varies by plugin. | MIT [66] | Mature CLI; plugin ecosystem is the unstable surface [27] | macOS/Linux/WSL | Good (`asdf install <name> <version>`) [28] | Depends on plugin (binary vs `ref:` source build) [27] | `always_keep_download` controls keeping downloaded source/binaries [27] |
| **aqua** | Declarative CLI version manager: `aqua.yaml` with pinned `registries` ref, `packages` and imports [29]. Security design: does not execute external commands except `go install`/`go build`, centrally managed registry, checksum verification, policy-as-code, standard registry only by default [30]. | MIT [80] | Config schema documented; v2 changed defaults [29] | Cross-platform | Excellent (declarative) | Checksum verification + policy as code [30] | Pinned registry ref + checksums make installs reproducible [29][30] |
| **proto** | "Pluggable version manager, unified toolchain": Rust + WASM, multi-language, tools can be languages, language package managers, or custom plugins [31]. `.prototools` TOML at local/global/user locations pins versions, enables plugins and sets options [32]. | MIT [68] | Documented config locations/version spec [31][32] | Cross-platform [31] | Excellent | Pinned versions via `.prototools` [32] | Managed tool installs under user dir; version pins [32] |
| **UniGetUI** | GUI that unifies WinGet, Scoop, Chocolatey, Snap and more; GitHub topics also list npm/pip; explicitly a "package manager manager" [35][36]. Its comparison guide discusses source models, scope, permissions and package coverage across the three Windows managers [34]. It is a **frontend/orchestration precedent, not a backend library** for Devdeck. | MIT [35] | GUI releases; wraps underlying CLIs [35] | Product page advertises Windows, macOS and Linux [36] | N/A (GUI) | Delegates to underlying managers [34] | Delegates to underlying managers [34] |
| **DevPod-compatible tooling** | A DevPod workspace is a containerized dev environment holding source and dependencies, created/managed through a **provider** (local, cloud, Kubernetes); it reuses the `devcontainer.json` spec also used by VS Code dev containers and GitHub Codespaces [59]. Providers include Docker, Kubernetes, SSH, AWS, GCloud, Azure, installed via `devpod provider add <name>` [60]. It orchestrates *environments*, not host package installs — an optional adapter for "give me a ready toolchain", not a replacement for the host manager layer. | Open source (loft-sh/devpod) | CLI + devcontainer.json standard [59][60] | Cross-platform via providers [60] | Good (CLI) | devcontainer spec + provider images [59] | Cached provider images; devcontainer definitions [59] |

---

## 3. Proposed orchestration layer **[Proposed — not implemented]**

### 3.1 Design principles

1. **Delegate, do not reimplement.** Every install/update/uninstall/verify runs the native manager as an external process with captured stdout/stderr. The orchestrator owns *planning, policy, state and recovery*, not archive extraction or dependency resolution.
2. **Own only a namespace.** All Devdeck-managed artifacts live under `~/.devdeck/...` (per `VISION.md`). A single generated shim directory `~/.devdeck/shims` is the only PATH entry Devdeck asks for, and only after explicit consent.
3. **Dry-run first.** Every mutating operation produces a typed `Plan` that can be rendered to the user and stored for approval/rollback before execution.
4. **Provenance is mandatory.** No record is marked "installed" without a source, a verification result and an approval trace.
5. **Adapt at runtime.** Adapter availability is detected, not assumed; the selection algorithm chooses among available managers per operation.

### 3.2 Adapter trait (conceptual Rust interface)

```rust
trait ManagerAdapter {
    fn id(&self) -> &'static str;                         // "winget", "brew", "uv", ...
    fn capabilities(&self) -> Capabilities;              // bitflags
    fn detect(&self) -> Result<Vec<InstalledArtifact>>;   // what exists now
    fn discover_versions(&self, tool: &ToolId) -> Result<Vec<Version>>;
    fn plan(&self, op: Operation, spec: &Spec) -> Result<Plan>;
    fn apply(&self, plan: &Plan, ctx: &ExecContext) -> Result<Outcome>;
    fn verify(&self, record: &InstallRecord) -> Result<Verification>;
    fn warm_cache(&self, spec: &Spec) -> Result<CacheResult>;
    fn uninstall(&self, record: &InstallRecord) -> Result<Outcome>;
    fn rollback_hint(&self, record: &InstallRecord) -> RollbackHint;
}

struct Capabilities {
    version_pin: bool, checksum: bool, signature: bool,
    offline: bool, uninstall: bool, downgrade: bool,
    system_scope: bool, service_actions: bool, config_export: bool,
}
```

Capability flags are populated from the surveyed facts. `signature=true` for winget manifests [4], pacman [22] and cargo-binstall-with-metadata [39]. `offline=true` for uv [42], Gradle [57] and pnpm [48]. apt-offline also qualifies [18]. `config_export=true` for Homebrew [12], mise [23] and asdf [27]. aqua [29] and proto [32] also export config.

### 3.3 Detection

Three independent probes are merged and reconciled:

1. **Filesystem probe** — scan app-owned `~/.devdeck` state plus manager-native locations. Debian and Arch expose `/var/lib/dpkg/status` and `/var/lib/pacman/local` [17][21]. Scoop uses `~/scoop/apps` [6][7]. Homebrew Cellar [13] and `~/.asdf/installs` [26][27] are probed next. mise install dirs [23] and `~/.proto/tools` [31][32] complete the set. Cargo `~/.cargo/bin` [52] is probed as well.
2. **Manager probe** — invoke each detected adapter's `detect()`/`list`.
3. **PATH probe** — resolve each executable name on `PATH`, hash the target, and compare against known artifact paths to identify *shadowing* (a different manager's binary winning).

Each `InstalledArtifact` records: binary name, absolute path, resolved version, owning adapter (or `Unknown`), SHA-256 of the target, and scope (user/system). Unknown binaries are still recorded — they are the most important conflict source.

### 3.4 Version discovery and selection

`discover_versions()` queries adapter sources and caches results with a TTL. Version requests follow the mise grammar (exact, prefix, `latest`, alias) and resolve to concrete versions recorded in a lockfile [25]. Selection score, highest wins, with deterministic tie-break by adapter id:

| Criterion | Weight | Rationale |
|---|---|---|
| Requested version available | hard requirement | — |
| Platform supported | hard requirement | e.g. winget Windows-only [1] |
| Provenance strength (signature > checksum > none) | high | Maven's warning that repository checksums are integrity-only [54] |
| Offline cache satisfiable | high when offline requested | uv `--offline` [42], pnpm `--offline` [48] |
| No admin/elevation required | high | Homebrew warns sudo breaks its trust model [14]; Scoop avoids UAC [63] |
| Smallest conflict footprint | medium | Scoop explicitly avoids PATH pollution [63] |
| User preference order | medium | user-configurable |
| License-policy compatible for in-process use | hard rule | GPLv3 cargo-binstall [38], Artistic-2.0 npm [81], PolyForm pnpm subdir [72] are **external-process only** |

The chosen plan is returned for approval; alternatives with their scores are displayed so the user can override.

### 3.5 Install / update / uninstall execution

- Compose argv from a fixed template per adapter; no shell interpolation; arguments allowlisted; secrets redacted from logs.
- Always pass the adapter's non-interactive switches. winget supports `--disable-interactivity`, `--accept-package-agreements` and `--silent` [2][5]. pacman supports `--noconfirm` [21]; Maven supports `-B` [56]. pnpm supports `--frozen-lockfile` [48] and Yarn `--immutable` [49]. pip supports `--require-hashes` when hashes are available [50]; uv `--offline` in offline mode [42]; Gradle `--offline` [57].
- Never auto-elevate. If a plan requires elevation (system-scope winget, some Homebrew casks [13]), stop and ask.
- After `apply`, run `verify()` and compare detected state against the plan; a mismatch downgrades the record to `Partial` and triggers rollback evaluation.
- Uninstall uses the native manager; if none is available, quarantine the app-owned directory rather than deleting it.

### 3.6 PATH conflict detection algorithm

```
build_shim_map(records, path_env):
    names = {}
    for record in records:
        for bin in record.declared_binaries:              # e.g. Scoop "bin" [6]
            target = resolve_shim(record, bin)            # ~/.devdeck/shims/<bin>
            names[bin].append((record, target, path_index(target, path_env)))

    conflicts = []
    for bin, entries in names.items():
        if len(entries) > 1:
            winners = sorted(entries, key=path_index)
            if winners[0].record != intended_owner(bin):
                conflicts.push(Shadowed(bin, winners, intended_owner))
        for entry in entries:
            external = resolve_on_path(bin, exclude=devdeck_paths)
            if external and external.path not in record.owned_paths:
                conflicts.push(SystemShadow(bin, external, entry))
        if case_insensitive_fs and has_case_variant(bin, names):
            conflicts.push(CaseCollision(bin))

    if path_env.count(devdeck_shim_dir) > 1: conflicts.push(DuplicatePathEntry)
    if devdeck_shim_dir not first: conflicts.push(PrecedenceRisk)
    return conflicts
```

Severity: `SystemShadow` and `Shadowed` are **high** (require approval); `CaseCollision` (Windows/macOS default filesystems) and `DuplicatePathEntry` are **medium**; `PrecedenceRisk` is **low** with a suggested fix. Remediation is always "one Devdeck shim directory, reordered" rather than each manager prepending its own directory — this directly addresses the PATH-pollution problem Scoop was designed to avoid [63].

### 3.7 Config and environment variable management

Every mutation is an `EnvChange { scope, key, op, value, reversible }` where scope is `Process | Session | User | System`. Prefer session-scoped injection into Devdeck's own terminal/tool processes; only write user-level config after approval. Persist declarative manifests that managers already understand: `Brewfile` [12], `mise.toml` [23] and `.tool-versions` [27]. `aqua.yaml` [29] and `.prototools` [32] are next. Lockfiles [25][48][52] are stored too. Store them under `~/.devdeck/manifests/`, versioned and Git-friendly per `VISION.md`.

### 3.8 Installation record data model

```rust
struct InstallRecord {
    id: Ulid,
    tool: ToolId,                       // logical tool, e.g. "node"
    component: String,                  // "runtime", "lsp", "service"
    requested_spec: String,             // "22", "16.2", "latest"
    resolved_version: String,
    adapter_id: String,                 // "winget" | "brew" | "uv" | ...
    adapter_version: String,
    manager_invocation: RedactedArgv,   // argv, secrets removed
    source: SourceRef {                 // repo/index/manifest and exact ref
        kind: SourceKind, url: String, ref_or_rev: Option<String>
    },
    artifact: ArtifactRef {
        url: String, algorithm: HashAlg, digest: String,
        signature: Option<SignatureRef>, signer: Option<String>
    },
    verification: Verification {
        checksum_verified: bool, signature_verified: bool, verified_at: DateTime<Utc>
    },
    owned_paths: Vec<PathBuf>,          // all under ~/.devdeck
    shims: Vec<Shim>,
    env_changes: Vec<EnvChange>,
    service_refs: Vec<ServiceRef>,
    depends_on: Vec<Ulid>,
    install_scope: Scope,               // User | System
    approval: ApprovalRef,              // who/when/which plan hash
    transaction_id: Ulid,
    previous_record: Option<Ulid>,
    status: InstallStatus,              // Installed | Partial | Superseded | Removed | Failed
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

Verification fields map directly to manager evidence. winget `InstallerSha256`/`SignatureSha256` [4] and Chocolatey `Get-ChecksumValid` [8] provide artifact hashes. Yarn `--check-cache` [49] and pnpm `store status` [47] verify installed content. aqua adds checksum verification and policy-as-code [30]; cargo-binstall signature metadata [39] is optional. pip `--require-hashes` [50] and GPG/RPM plus pacman OpenPGP trust [22][61] complete the picture. For Maven-family artifacts the record must flag `checksum_trust = IntegrityOnly` because repository checksums do not establish trust [54].

### 3.9 Offline operation

- **Warm phase (online):** for each planned artifact, call `warm_cache()`. Use winget `download` unless `DownloadCommandProhibited` [4], and `pip download`/`pip wheel` [51]. `apt-offline get` supports `--strict-deb-check` [18]; DNF offers `download`/`offline-upgrade` [19]. pacman's package cache [21] is next. npm/pnpm/Yarn caches [45][47][49] follow. uv cache and `--offline` [41][42] come next. Maven local repo + `-o` [55][56] and Gradle cache + `--offline` [57] follow. Cargo registry cache/index [52][53] completes the list. Chocolatey's general offline story relies on internal repositories and its commercial internalizer [9][10] — record this as a capability gap for the free tier.
- **Apply phase (offline):** set a global `offline` flag that forces adapter flags (`--offline`, `-o`, `--frozen-lockfile`, `--immutable-cache`) and **forbids network fallback**. A cache miss is a hard failure with a precise "stage this artifact first" message, never a silent online retry.
- **Verification offline:** checksums can be re-verified locally; online-only signature revocation checks are recorded as `deferred` and surfaced when connectivity returns.

### 3.10 Services (without reinventing them)

Devdeck stores a `ServiceRef { id, definition_source, desired_state, manager, healthcheck, log_path }` and delegates lifecycle actions:

| Service class | Delegated manager | Evidence |
|---|---|---|
| Homebrew-installed daemons | `brew services` (already part of `brew bundle`) [12] | Brewfile can start services [12] |
| Linux daemons (Docker Engine, PostgreSQL, Redis) | systemd units via `systemctl`, with adapter detecting unit names | OS facility, not reimplemented |
| macOS daemons | launchd plists | OS facility |
| Windows services | SCM / `sc.exe` | OS facility |
| Language servers | LSP adapters launched over stdio by the editor/debugger crate, version pinned by mise/asdf/aqua/proto or by Cargo/npm | Version-manager pins [23][27][29][32] |
| Container engines / dev environments | Docker/containerd socket abstraction; optional DevPod provider using `devcontainer.json` | DevPod workspaces and providers [59][60] |

Devdeck owns *desired state and health checks*, not init systems, socket protocols or devcontainer semantics. A service action is always high-risk and requires approval.

### 3.11 Safety and approval rules

- **Risk tiers.** Low: query/dry-run/cache-warm (no approval). Medium: user-scope install under `~/.devdeck` with no PATH change (notify). High: system scope, any PATH/env change, service start/stop, uninstall of a pre-existing tool, elevation, or override of a conflict (explicit approval modal showing the plan diff and rollback hint).
- **Plan hash binding.** The approval record stores a hash of the exact `Plan`; any drift invalidates approval and re-prompts.
- **No hidden prompts.** If an adapter cannot be put in non-interactive mode for the operation, the plan is rejected rather than launched (winget and pacman both provide explicit flags [2][21]; Maven provides `-B` [56]).
- **License policy.** Non-compatible adapters are external-process only (GPLv3 [38], Artistic-2.0 [81], PolyForm [72]). Telemetry is disabled by default where an adapter offers it (cargo-binstall `--disable-telemetry` [38]).
- **Audit.** All plans, approvals, invocations and verification results go to the append-only, redacted audit log described in `VISION.md`.

### 3.12 Windows / macOS / Linux differences

| Area | Windows | macOS | Linux |
|---|---|---|---|
| Primary adapters | winget [1], Scoop [6][7], Chocolatey [64] | Homebrew [13] | APT [17], DNF [19], pacman [21] |
| Filesystem case sensitivity | case-insensitive by default → case-collision check | usually case-insensitive → same check | usually case-sensitive |
| PATH | User vs Machine scope; long-standing pollution problem [63] | shell profile injection; `/usr/bin` is SIP-protected | shell profile / `/etc/profile.d` |
| Elevation | UAC; Scoop deliberately avoids it [63] | `brew` sudo discouraged; casks may need it [13][14] | `sudo` for system managers |
| Trust model | Authenticode [9], MSIX `SignatureSha256` [4] | signed bottles; Gatekeeper/notarization | GPG web-of-trust [22][61], SHA256 Release indexes [16] |
| Offline | winget `download` [4], Chocolatey internal repos [9] | Homebrew cache/bottles [13] | apt-offline [18], dnf [19], pacman cache [21] |
| Services | SCM | launchd | systemd |

### 3.13 Rollback strategy

1. **Snapshot before apply.** Write `~/.devdeck/transactions/<tx>.json` containing prior `InstallRecord`s, PATH/env values, config file hashes, service desired states and owned-path inventory; atomic write per `AGENTS.md`.
2. **Prefer native rollback.** Manager uninstall/update-to-prior-version where supported; otherwise quarantine the app-owned directory to `~/.devdeck/.trash/<tx>/` instead of deleting.
3. **Restore side effects.** Revert each recorded `EnvChange` in reverse order; restore config files from atomic backups; restart services to their prior desired state.
4. **Re-detect and verify.** Re-run detection; compare to the snapshot. Report `RolledBack`, `PartialRollback` or `RollbackFailed` explicitly — never claim success on a partial result.
5. **Honest limits.** System package managers may not support downgrade, OS installers can leave registry/service residue, and a manager that changed shared state outside the snapshot cannot be fully reverted. These are documented as known limits, not hidden.

---

## 4. Risks

- **License incompatibility:** cargo-binstall GPLv3 [38], npm CLI Artistic-2.0 [81], pnpm PolyForm directory [72] cannot be vendored; relying on them as external CLIs creates version-drift and availability risk.
- **Manager churn:** mise [23] and uv [42] have fast-moving CLIs/config defaults. aqua [29] and proto [31] do too; adapter templates need version gating and tests.
- **Commercial feature gap:** Chocolatey's offline internalizer is licensed-only [10]; the free tier cannot promise the same offline story.
- **Checksum ≠ trust:** Maven repository checksums are integrity-only [54]; a naive orchestrator could report "verified" while remaining vulnerable to supply-chain replacement.
- **Signature gaps:** cargo-binstall signatures require explicit crate metadata (no autodiscovery) [39]; pip is insecure by default without `--require-hashes` [50]; asdf provenance depends on the plugin [27].
- **PATH/config drift:** multiple managers editing shell profiles can defeat the single-shim-directory design; Windows case-insensitivity adds collisions.
- **Privilege escalation:** any auto-`sudo`/UAC path breaks Homebrew's explicit trust model [14] and violates Devdeck's consent constraints.
- **DevPod scope creep:** environment orchestration via providers/devcontainers [59][60] is a much larger surface than host tool management and should stay a separate, optional adapter.

## 5. Open questions

1. Should Devdeck ship its own shim-directory implementation, or reuse Scoop-style shims on Windows [6][63]?
2. Which version-manager is the *primary* backend for language runtimes? Candidates include mise [23][25]. aqua [29][30] and proto [31][32] are alternatives. The survey shows overlapping but non-identical trust models (aqua's policy-as-code vs mise's lockfile).
3. How should `Unknown` pre-existing binaries be handled: adopt them into the provenance ledger, or leave them untouched and merely flag conflicts?
4. What is the offline artifact format? Reuse each manager's cache — uv [41], npm [45] and Maven [55] — or add a Devdeck-level content-addressed store with its own checksums? Gradle's portable cache [57] is one precedent.
5. Is system-scope installation ever worth the approval/rollback cost, or should Devdeck be strictly user-scope?
6. Should the npm/Artistic-2.0 and cargo-binstall/GPLv3 adapters be invoked at all, given the project's dependency policy, or exposed only via optional plugin installs?
7. How do we model service health checks portably across systemd/launchd/SCM without pulling in heavy dependencies?
8. How should revocation/expiry of signatures be represented when operating offline?

---

## Sources

[1] https://learn.microsoft.com/en-us/windows/package-manager/winget — Use WinGet to install and manage applications | Microsoft Learn
[2] https://learn.microsoft.com/en-us/windows/package-manager/winget/install — install command (winget) | Microsoft Learn
[3] https://learn.microsoft.com/en-us/windows/package-manager/winget/settings — settings command (winget) | Microsoft Learn
[4] https://github.com/microsoft/winget-pkgs/blob/master/doc/manifest/schema/1.10.0/singleton.md — winget-pkgs manifest schema 1.10.0 singleton | Microsoft
[5] https://github.com/microsoft/winget-cli/blob/master/doc/windows/package-manager/winget/install.md — winget-cli install command documentation | Microsoft
[6] https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests — App Manifests | Scoop Wiki
[7] https://github.com/ScoopInstaller/Scoop/wiki/Buckets — Buckets | Scoop Wiki
[8] https://docs.chocolatey.org/en-us/create/functions/get-checksumvalid — Get-CheckSumValid | Chocolatey Docs
[9] https://docs.chocolatey.org/en-us/information/security — Security | Chocolatey Docs
[10] https://docs.chocolatey.org/en-us/choco/commands/download — Download command | Chocolatey Docs
[11] https://docs.chocolatey.org/en-us/configuration — Configuration | Chocolatey Docs
[12] https://docs.brew.sh/Brew-Bundle-and-Brewfile — Homebrew Bundle and Brewfile | Homebrew Docs
[13] https://docs.brew.sh/Installation — Installation | Homebrew Docs
[14] https://docs.brew.sh/Manpage — brew(1) Manpage | Homebrew Docs
[15] https://github.com/Homebrew/brew/blob/main/Library/Homebrew/env_config.rb — Homebrew env_config.rb | Homebrew/brew
[16] https://www.debian.org/doc/manuals/debian-handbook/sect.package-authentication.en.html — Checking Package Authenticity | Debian Handbook
[17] https://www.debian.org/doc/manuals/debian-faq/pkgtools.en.html — Debian package management tools | Debian FAQ
[18] https://manpages.debian.org/testing/apt-offline/apt-offline.8.en.html — apt-offline(8) | Debian Manpages
[19] https://dnf.readthedocs.io/en/latest/command_ref.html — DNF Command Reference
[20] https://dnf.readthedocs.io/en/latest/conf_ref.html — DNF Configuration Reference
[21] https://man.archlinux.org/man/pacman.8.en.html — pacman(8) | Arch Manual Pages
[22] https://wiki.archlinux.org/title/Pacman/Package_signing — Pacman/Package signing | ArchWiki
[23] https://mise.jdx.dev/getting-started — Getting started | mise-en-place
[25] https://mise.jdx.dev/dev-tools/versions — Version requests and version files | mise-en-place
[26] https://asdf-vm.com/guide/getting-started.html — Getting Started | asdf
[27] https://asdf-vm.com/manage/configuration.html — Configuration | asdf
[28] https://asdf-vm.com/manage/versions.html — Versions | asdf
[29] https://aquaproj.github.io/docs/reference/config — Configuration | aqua
[30] https://aquaproj.github.io/docs/reference/security — Security | aqua
[31] https://moonrepo.dev/docs/proto — What is proto? | moonrepo
[32] https://moonrepo.dev/docs/proto/config — Configuration | proto | moonrepo
[34] https://unigetuisoftware.com/winget-vs-chocolatey-vs-scoop — WinGet vs Chocolatey vs Scoop | UniGetUI
[35] https://github.com/devolutions/unigetui — UniGetUI | GitHub
[36] https://devolutions.net/unigetui — UniGetUI for Windows, macOS, and Linux | Devolutions
[37] https://github.com/cargo-bins/cargo-binstall — cargo-binstall | GitHub
[38] https://github.com/cargo-bins/cargo-binstall/blob/main/HELP.md — cargo-binstall HELP.md
[39] https://github.com/cargo-bins/cargo-binstall/blob/main/SIGNING.md — cargo-binstall SIGNING.md
[40] https://github.com/cargo-bins/cargo-binstall/blob/main/SUPPORT.md — cargo-binstall SUPPORT.md
[41] https://docs.astral.sh/uv/concepts/cache — Caching | uv
[42] https://docs.astral.sh/uv/reference/cli — Commands | uv CLI reference
[43] https://docs.astral.sh/uv/concepts/indexes — Package indexes | uv
[44] https://docs.npmjs.com/cli/v11/commands/npm-ci — npm-ci | npm Docs
[45] https://docs.npmjs.com/cli/v12/commands/npm-cache — npm-cache | npm Docs
[46] https://docs.npmjs.com/cli/v11/using-npm/config — Config | npm Docs
[47] https://pnpm.io/cli/store — store command | pnpm
[48] https://pnpm.io/cli/install — install command | pnpm
[49] https://yarnpkg.com/cli/install — yarn install | Yarn
[50] https://pip.pypa.io/en/stable/topics/secure-installs — Secure installs | pip
[51] https://pip.pypa.io/en/latest/cli/pip_download — pip download | pip
[52] https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html — Cargo.toml vs Cargo.lock | Cargo Book
[53] https://doc.rust-lang.org/cargo/reference/registry-index.html — Registry Index | Cargo Book
[54] https://maven.apache.org/resolver/about-checksums.html — Checksums | Artifact Resolver | Maven
[55] https://maven.apache.org/guides/introduction/introduction-to-repositories.html — Introduction to Repositories | Maven
[56] https://maven.apache.org/ref/current/maven-embedder/cli.html — Maven CLI Options Reference
[57] https://docs.gradle.org/current/userguide/dependency_caching.html — Dependency caching | Gradle
[58] https://docs.gradle.org/current/userguide/command_line_interface.html — Command Line Interface | Gradle
[59] https://devpod.sh/docs/developing-in-workspaces/what-are-workspaces — What are Workspaces? | DevPod docs
[60] https://devpod.sh/docs/managing-providers/add-provider — Add a Provider | DevPod docs
[61] https://docs.fedoraproject.org/en-US/security/cryptography/signatures — Cryptographic Signatures in Fedora | Fedora Docs
[62] https://github.com/Homebrew/brew — Homebrew/brew | GitHub
[63] https://github.com/ScoopInstaller/Scoop — ScoopInstaller/Scoop | GitHub
[64] https://github.com/chocolatey/choco — chocolatey/choco | GitHub
[65] https://github.com/jdx/mise — jdx/mise | GitHub
[66] https://github.com/asdf-vm/asdf — asdf-vm/asdf | GitHub
[68] https://github.com/moonrepo/proto — moonrepo/proto | GitHub
[69] https://github.com/astral-sh/uv — astral-sh/uv | GitHub
[70] https://github.com/microsoft/winget-cli — microsoft/winget-cli | GitHub
[72] https://github.com/pnpm/pnpm — pnpm/pnpm | GitHub
[73] https://github.com/yarnpkg/berry — yarnpkg/berry | GitHub
[74] https://github.com/pypa/pip — pypa/pip | GitHub
[75] https://github.com/apache/maven — apache/maven | GitHub
[76] https://github.com/gradle/gradle — gradle/gradle | GitHub
[77] https://github.com/rust-lang/cargo — rust-lang/cargo | GitHub
[78] https://github.com/ScoopInstaller/Scoop/blob/master/LICENSE — Scoop LICENSE
[79] https://github.com/chocolatey/choco/blob/develop/LICENSE — Chocolatey choco LICENSE
[80] https://github.com/aquaproj/aqua/blob/main/LICENSE — aqua LICENSE
[81] https://github.com/npm/cli/blob/latest/LICENSE — npm/cli LICENSE
