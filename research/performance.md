# Devdeck — Performance Strategy and Benchmarking Methodology

**Status: research only. No product implementation is part of this document.**

## 0. Claim discipline

**No benchmark numbers are claimed in this report.** Every figure in Section 1 is a
*proposed target*, not a measurement. `docs/performance.md` already states that budgets
are enforced in CI from Phase 7 with criterion benchmarks and a >10% regression gate,
and that no number may be claimed before the command and its output are recorded there.
The same status is reiterated in `docs/progress.md`: performance numbers are not
measured and the budget document is intentionally empty of claims.

The only size figure anywhere in the repository is a **15 MB stripped, thin-LTO release
binary reported in `docs/progress.md`**. It is quoted here as an existing in-repo report,
not reproduced or verified by this research task; the reproduction command is in
Section 4.7.

Methodology statements below are grounded in the official documentation of the tools
named; the numbered citations point to those documents (see Sources). Design decisions,
targets and process placement are proposals of this report and are marked as such.

## 1. Proposed budgets

All statistics are computed over ≥30 samples unless stated otherwise. "p95/p99" means
the 95th/99th percentile of the sampled distribution, not a single best run.

| Metric | Proposed target | Statistic / condition | Status |
| --- | --- | --- | --- |
| Cold start, core only | ≤ 500 ms | wall time to first interactive frame, cold page cache, ≥30 runs | proposed |
| Cold start with 20 extensions | < 1 s | same, 20-extension fixture, ≤150 ms of it extension host startup | proposed (inherits `docs/performance.md`) |
| Idle RAM, core only | < 150 MB | RSS of UI process, 5 min idle, no workspace | proposed (inherits `docs/performance.md`) |
| Idle RAM, workspaces open | < 400 MB total | summed RSS of UI + all sidecars, 10 repos / 20 files each, 5 min idle | proposed (new) |
| Keystroke-to-screen | < 16 ms | p95, editor and terminal, 60 Hz; ≤8.3 ms p95 at 120 Hz | proposed (inherits `< 16 ms`) |
| 100k-row grid | smooth scroll | p95 frame ≤16.7 ms, p99 ≤33 ms, zero frames >50 ms over a 10 s scroll | proposed |
| 100k-row grid, first fill | ≤ 500 ms | from request to first painted window (windowing, not materialisation) | proposed |
| 50 MB file / response | no freeze | first window painted ≤2 s; no single UI span >50 ms; incremental parse | proposed |
| 50 MB file / response, peak RAM | ≤ 2× input size | peak RSS delta over baseline during open | proposed |
| Terminal throughput | ≥ 100 MB/s PTY drain | median, 200×50 grid, mixed corpus; p95 frame ≤16.7 ms during burst | proposed |
| Extension call, in-process WASM | p95 ≤ 1 ms | narrow host call (permission check + host fn), warmed, 20 extensions | proposed |
| Extension call, sidecar round trip | p95 ≤ 2 ms | extension host → UI request/response over local IPC | proposed |
| Binary size, release | ≤ 25 MB | stripped per-OS release artifact, median of 3 clean builds | proposed |
| GPU path frame rate | 60 fps p95 / 120 fps on 120 Hz | p95 frame ≤16.7 / ≤8.3 ms | proposed (inherits `docs/performance.md`) |
| Software fallback frame rate | ≥ 30 fps at 1200×800 | p95 frame ≤33 ms, no hang/crash when no adapter found | proposed (new) |

### 1.1 Cold start

Cold start must be measured with the page cache dropped before every run: hyperfine's
`--prepare` exists for exactly this purpose, and its documentation gives
`sync; echo 3 | sudo tee /proc/sys/vm/drop_caches` as the Linux cache-clearing
recipe [4]. Warm-cache runs use `--warmup` instead, which fills disk caches before
measurement [4][5]. Report both; never mix them into one number. A "cold launch"
versus "warm launch" spectrum and the need to test under several conditions are also
the position of Apple's launch-time guidance [19].

### 1.2 Idle RAM

RSS is read from `VmRSS` in `/proc/<pid>/status`, with `VmHWM` as the high-water mark;
both are documented as approximate because RSS accounting is asynchronous [26][27].
For a precise snapshot use `/proc/<pid>/smaps_rollup`, which is explicitly described as
slower but accurate [27]. Sum over the whole process tree (UI + every sidecar) rather
than the UI process alone, because isolation moves memory without deleting it.

### 1.3 Keystroke-to-screen and frame budget

A 60 Hz frame is ≈16 ms of wall time; RAIL allocates ~10 ms of that to application
work because the browser (here: the compositor) needs the rest [28]. Devdeck should
adopt the same split: ≤10 ms of work per frame and a hard ≤16 ms end-to-end budget.
At 120 Hz the per-frame budget halves to ~8.3 ms, so the 120 fps target is conditional
on the workload fitting that smaller budget.

### 1.4 Large grids and large files

Windowing math has been spiked but not measured (`docs/performance.md`). The target is
therefore expressed as frame-time quantiles during a scripted scroll, not as "smooth".
A full 100k-row or 50 MB materialisation is explicitly out of scope for the UI thread;
only the visible window plus a bounded overscan may exist as render state.

### 1.5 Terminal throughput

vtebench is the reference corpus driver: it generates PTY-read payloads and runs them
through the terminal under test [25]. Its own disclaimer is important: it stresses only
the speed at which the terminal reads from the PTY and does **not** cover frame rate or
latency [25]. Consequently the terminal budget pairs a vtebench median with a separate
frame-time constraint measured on the same burst.

### 1.6 Extension call overhead

Wasmtime offers two interruption mechanisms with deliberately different costs:
fuel-based interruption is fully deterministic but adds more per-instruction overhead,
while epoch-based interruption is measured by its own documentation at around a 10%
slowdown and is non-deterministic in where it traps [12][13]. This report proposes
epoch deadlines for wall-clock bound and fuel only where deterministic accounting is
required; the 10% figure is quoted from Wasmtime's docs, **not** measured on Devdeck.

### 1.7 Binary size and rendering fallback

Cargo's profile knobs for size are `opt-level = "s"`/`"z"`, `strip`, `lto` and
`codegen-units`; the Cargo Book also warns that `"z"` is not guaranteed to be smaller
than `"s"` and that `lto = "thin"` gives most of the benefit of fat LTO at lower link
time [29]. GPU versus software selection is a wgpu adapter decision:
`force_fallback_adapter: true` restricts the result to a fallback (generally software)
implementation, and `RequestAdapterOptions::default()` may fail for a surface-dependent
backend [23][24]. The fallback budget therefore includes "app still opens and paints"
as a binary acceptance condition.

## 2. Process isolation, memory ownership and async boundaries

### 2.1 Placement decision

| Component | Placement | Why |
| --- | --- | --- |
| UI, renderer, window shell, palette, i18n/theme | UI process, UI thread | Latency-critical; needs zero-copy access to render state; Rust type safety removes the usual memory-safety reason to isolate |
| Active editor buffer (rope), grid window model, terminal screen model | UI process, immutable snapshots | Render hot path; mutations applied on workers and published as snapshots |
| Config, keybindings, permissions, audit writer | UI process, worker threads | Small, typed, shared state; audit writer is append-only and bounded |
| File I/O, HTTP/MCP client, git status, indexing, debounced parse | UI process, Tokio worker threads | Cheap to spawn; no IPC for shared typed state; must never touch the UI thread |
| Large-file tokenise/parse, 100k-row sort/filter, big JSON | Dedicated worker thread or sidecar, chunked | Bounded chunk time; cancellable; results returned as indices/snapshots |
| Terminal PTY and shell | One OS process per terminal | Arbitrary native code cannot be sandboxed in-process; parser stays in UI process, fed by a reader task |
| Extensions (WASM) | Separate extension-host sidecar; one Wasmtime `Store` per extension | Crash isolation and resource limits; Wasmtime sandboxes memory between instances and zeroes instance memory on reuse [11] |
| Language servers, MCP servers, DB/Docker/toolchain drivers | External supervised sidecars over stdio/local sockets | Third-party native processes; restart independently of the UI |

**Decision:** keep latency-critical typed state in-process, move anything untrusted,
crash-prone or unbounded into supervised sidecars, and put all I/O on Tokio workers.
This is a proposal; it has not been implemented or benchmarked.

### 2.2 Memory ownership rules

1. One owner per large buffer (rope, byte buffer, columnar grid). UI holds `Arc` to
   immutable snapshots; workers mutate through copy-on-write and publish a new version.
2. No bulk copy across a WASM boundary. Host↔guest transfers use explicit pointer+length
   views validated on the host side; Wasmtime's guard regions and instance-memory
   zeroing are an additional defence, not a substitute for validation [11].
3. Bounded caches everywhere: grid window + overscan, parsed-row cache, terminal
   scrollback. Each has an explicit eviction policy and a hard byte cap.
4. Under memory pressure, shed caches and idle workers before shedding user data. The
   systemd pressure protocol is the model: on a pressure event, release allocation
   caches, terminate idle workers and defer background work [7].
5. Every sidecar has an RSS cap and a restart budget; exceeding either is an incident,
   not a slow degradation.
6. Audit log is append-only, hash-chained and rotated; no unbounded in-memory queue in
   front of it.

### 2.3 Async I/O boundaries

All file, network and PTY I/O lives on the Tokio runtime. Blocking syscalls that cannot
be made async go to `spawn_blocking`, never to the UI thread. The UI receives completion
as a bounded-channel message carrying an immutable snapshot and drains at most a fixed
time budget per frame. Async operations are lazy in Rust, so concurrency must be
introduced deliberately; Tokio's own tutorial warns that unbounded queues eventually
consume all memory and fail unpredictably [21]. Host calls from WASM that block are a
special case: epoch deadlines only interrupt *running Wasm code* and do not help a guest
blocked in a host call [13], so such calls belong in the sidecar with their own timeout.

### 2.4 Cancellation and backpressure

Every job longer than one frame carries a cancellation token, checked at chunk
boundaries; UI-side waits use `select!`/timeout and only cancel-safe futures. Cancel
safety is a known footgun in async Rust and deserves an explicit policy rather than
ad-hoc fixes [22]. All event channels are bounded with a documented capacity; overload
is exposed with `try_send` or coalesced to latest-value (`watch`) channels, which avoid
the memory blow-up of unbounded queues [21][22]. Input events are coalesced latest-wins
so a burst of keystrokes cannot queue a burst of renders.

### 2.5 Avoiding UI-thread blocking

Rules: no synchronous I/O, no parse of >1 MB, no blocking IPC and no lock waiting on
the UI thread; preallocate render scratch buffers; do incremental work in ≤10 ms chunks
(Section 1.3). Instrument every UI span with `tracing` and add a watchdog that logs any
UI span over 16 ms. If the renderer still misses frames, degrade deliberately: reduce
grid overscan, skip non-essential layers, or drop to the software path with a lower
frame target rather than freezing.

## 3. Measurement methodology

### 3.1 Microbenchmarks — criterion

Criterion runs four phases — warm-up, measurement, analysis and comparison — and its
comparison phase uses bootstrap resampling plus a T-test to decide whether a difference
is real [1][3]. Outliers are classified with a modified Tukey method using 1×/3× IQR
fences but are **not** dropped from the data [1]. Defaults are 100 samples, 3 s warm-up,
5 s measurement, 100,000 bootstrap resamples, 95% confidence and a 1% noise threshold
(noisy results inside ±threshold are ignored) [2][3]. Longer measurement time is the
documented remedy for transient external load [2][3]. Criterion's own guidance is to
benchmark on a quiet machine with equal work per iteration; it explicitly warns that
power source alone can move results [1][2].

Benchmarks to add under `crates/devdeck-bench/benches/`: rope insert/delete at
1 KB/1 MB/50 MB, grid window slice at 1k/10k/100k rows, JSON chunk parse, WASM host
call, terminal parser `feed()` on the vtebench corpus, and snapshot construction.

### 3.2 Process-level cold start — hyperfine

hyperfine performs statistical analysis across runs, supports warmup runs and
cache-clearing preparation commands, detects outliers, and exports CSV/JSON/Markdown [4][5].
Use at least 30 measured runs, 3 warmup runs for the warm case, `--prepare` with the
drop-caches command for the cold case [4], and `--export-json` so CI can diff exact
distributions rather than retyped numbers.

### 3.3 RSS and PSI sampling

Sample the whole process tree: read `VmRSS`/`VmHWM` from `/proc/<pid>/status` for the
cheap high-frequency series [26], take `smaps_rollup` snapshots at the start and end of
each scenario for the accurate figure [27], and treat `VmHWM` as a peak that must be
reset between scenarios (the kernel documents `/proc/<pid>/clear_refs` for that) [26].
For contention, use Linux PSI: `/proc/pressure/{cpu,memory,io}` expose `some`/`full`
averages over 10/60/300 s plus a cumulative `total` in microseconds, and the same
format is available per cgroup as `<resource>.pressure` [6]. PSI files are pollable with
thresholds such as `some 150000 1000000`, i.e. 150 ms of stall in a 1 s window [6].

### 3.4 Per-OS tracing

- **Linux:** `perf stat` aggregates hardware/software event counts, and `perf record`
  writes a profile to `perf.data` for `perf report`; perf also covers tracepoints,
  kprobes and uprobes [8][9][10].
- **Cross-platform trace format:** emit Chrome Trace Event JSON (`traceEvents` with
  `ph` begin/end events) so the same instrumentation opens in Perfetto, a tracing
  suite that natively supports Chrome JSON [14][15][16].
- **Windows:** ETW is the underlying mechanism; WPR records ETW traces using built-in
  or custom profiles and WPA graphs and pivots them, with xperf retained for command-line
  capture [17][18].
- **macOS:** Xcode's App Launch template gathers a time profile and a thread-state trace
  during launch; Time Profiler is the instrument for hangs and CPU bottlenecks, and
  MetricKit provides histograms of field metrics [19][20].

### 3.5 Frame-time histograms

Instrument frame start/end as paired trace events (Section 3.4) and build a histogram
of frame durations with p50/p95/p99/max plus counts above 16.7 ms and 33 ms. Use the
RAIL 16 ms frame budget as the external rationale for the threshold [28]. Report GPU and
`force_fallback_adapter` runs as two separate histograms [23].

### 3.6 Test corpora generation

Generate corpora with a fixed seed and commit the generator, not the blobs:
100k rows with mixed numeric/unicode/long-string columns; text files at 50 MB and
100 MB; JSON at 50 MB pretty and minified; a 200×50 terminal corpus split into
ascii/unicode/CSI/scrolling suites using vtebench's benchmark-directory format [25];
a 20-extension fixture of small WASM modules; and workspace fixtures of 1/10/20 repos.
Record the exact generator invocation beside every result.

### 3.7 CI regression gates and noise handling

The repository's stated policy is a >10% regression gate enforced in CI from Phase 7
(`docs/performance.md`). Implementation rules: compare bootstrap confidence intervals,
not point estimates, as Criterion does [1]; keep a fast PR gate on a pinned quiet runner
and a full nightly gate on all three OSes; rerun any failed benchmark twice and require
the regression to reproduce before blocking; widen the noise threshold on shared runners
rather than accepting a flaky gate; store raw JSON artifacts so a failure can be
diagnosed from the distribution. Criterion's caution about uncontrolled environments
applies directly to CI [2].

## 4. Exact commands that would produce the numbers

Nothing in this section has been executed as part of this research task. Commands marked
"after instrumentation" require the Phase 7 harness and bench crate, which do not exist
yet. Commands marked "runnable now" can be run against the current workspace.

### 4.1 Microbenchmarks (after instrumentation)

```bash
cargo bench --workspace
cargo bench -p devdeck-bench -- grid_window --measurement-time 10 --warm-up-time 5 --sample-size 200
cargo bench -p devdeck-bench -- --save-baseline main
cargo bench -p devdeck-bench -- --baseline main          # CI comparison
```

### 4.2 Cold start, warm and cold (after `--bench-exit` hook)

```bash
hyperfine --export-json target/perf/warm-start.json --warmup 3 --runs 30 \
  'target/release/devdeck-app --bench-exit'
sudo -v
hyperfine --export-json target/perf/cold-start.json --runs 30 \
  --prepare 'sync; echo 3 | sudo tee /proc/sys/vm/drop_caches' \
  'target/release/devdeck-app --bench-exit'
```

### 4.3 RSS / PSI sampling (runnable now on Linux)

```bash
PID=$(pgrep -f target/release/devdeck-app | head -1)
grep -E '^(VmRSS|VmHWM|RssAnon|RssFile|RssShmem):' /proc/$PID/status
cat /proc/$PID/smaps_rollup
cat /proc/pressure/cpu /proc/pressure/memory /proc/pressure/io
for p in /proc/$PID/task/*/children; do cat "$p"; done   # enumerate sidecars to sum RSS
```

### 4.4 Tracing (after instrumentation)

```bash
# Linux: counters and sampled profile
perf stat -e cycles,instructions,cache-misses,page-faults,context-switches \
  -- target/release/devdeck-app --bench-scenario grid-100k
perf record -F 99 -g --call-graph dwarf -o target/perf/perf.data -- \
  target/release/devdeck-app --bench-scenario grid-100k
perf report -i target/perf/perf.data

# Chrome Trace Event JSON → Perfetto (open in https://ui.perfetto.dev)
target/release/devdeck-app --bench-scenario grid-100k --trace target/perf/frame.json

# Windows (PowerShell): ETW capture, analyse the .etl in WPA
wpr -start CPU -filemode
# ... run devdeck-app.exe scenario ...
wpr -stop target/perf/devdeck.etl

# macOS: App Launch / Time Profiler equivalent
xcrun xctrace record --template 'App Launch' --launch -- ./target/release/devdeck-app \
  --output target/perf/launch.trace
xcrun xctrace export --input target/perf/launch.trace --output target/perf/launch.xml
```

### 4.5 Frame-time histogram (after trace instrumentation)

```bash
python3 scripts/perf/frame_hist.py target/perf/frame.json \
  --out target/perf/frame-hist.json --thresholds 16.7,33
python3 scripts/perf/frame_hist.py target/perf/frame-software.json \
  --out target/perf/frame-hist-software.json --thresholds 16.7,33
```

### 4.6 Terminal throughput (runnable now once vtebench is vendored)

```bash
cd vendor/vtebench && cargo run --release -- --max-secs 3 --max-samples 30 \
  --dat results.dat
./gnuplot/summary.sh results.dat target/perf/terminal.svg
```

### 4.7 Binary size (runnable now)

```bash
cargo build --release --workspace
stat -c '%s bytes' target/release/devdeck-app     # Linux
```

### 4.8 CI gate (after baseline artifacts exist)

```bash
python3 scripts/perf/compare.py --baseline target/perf/baseline \
  --current target/perf/current --regression-threshold 0.10 target/perf/*.json
```

## 5. Risks

- No baseline exists, so every target in Section 1 is unfalsifiable until the Phase 7
  harness lands; CI runners are noisy and may produce flaky >10% gates.
- vtebench measures PTY drain only, not frame rate or latency [25]; a passing number can
  hide a janky terminal without the paired frame histogram.
- `VmRSS` is explicitly approximate [26], and `VmHWM` is a peak; using either alone can
  misstate the idle-RAM budgets.
- Wasmtime's ~10% epoch overhead [12] plus host-call marshalling may push extension
  calls past the proposed budgets; fuel is deterministic but costlier [13].
- wgpu headless/DRI3 warnings are already observed in the repo without real Windows or
  macOS GPU verification; the fallback adapter path is untested for the 30 fps target.
- Cargo size knobs are not monotonic — `"z"` is not guaranteed smaller than `"s"` [29] —
  so the binary budget may need several build profiles.
- Isolation moves memory into sidecars; summing only UI RSS would under-report the
  "workspaces open" budget.
- The GPUI spike failed upstream (`docs/progress.md`); Iced 0.14 is pre-1.0-adjacent, so
  renderer changes could invalidate measured frame budgets.

## 6. Open questions

- What exact reference hardware (CPU, RAM, GPU, OS versions) pins every budget?
- Is the 150 MB core-idle target achievable with the current Iced + wgpu + ICU4X +
  Fluent baseline, or must it be revised after first measurement?
- Do first-party extensions run in the shared extension host or each get their own
  process, and what is the memory cost of the latter?
- Should the software-fallback target be 30 fps, or feature-reduced rendering with an
  explicit "degraded" indicator?
- Which percentile (p50 or p95) and how many reruns define the >10% CI gate?
- Is an in-process WASM tier needed for trusted extensions to cut per-call overhead?
- Is the 100k-row grid columnar or rope-backed? This determines the ownership rule in
  Section 2.2.
- Does "cold start" include first-run extraction and extension compilation, or only
  process launch to first frame?

## Sources

[1] https://criterion-rs.github.io/book/user_guide/analysis.html
[2] https://criterion-rs.github.io/book/user_guide/command_line_output.html
[3] https://docs.rs/criterion/latest/criterion/struct.Criterion.html
[4] https://github.com/sharkdp/hyperfine
[5] https://man.archlinux.org/man/extra/hyperfine/hyperfine.1.en
[6] https://kernel.org/doc/Documentation/accounting/psi.txt
[7] https://systemd.io/PRESSURE
[8] https://perfwiki.github.io/main
[9] https://man7.org/linux/man-pages/man1/perf-stat.1.html
[10] https://man7.org/linux/man-pages/man1/perf-record.1.html
[11] https://docs.wasmtime.dev/security.html
[12] https://docs.wasmtime.dev/examples-interrupting-wasm.html
[13] https://docs.wasmtime.dev/api/wasmtime/struct.Config.html
[14] https://perfetto.dev/docs
[15] https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview
[16] https://perfetto.dev/docs/instrumentation/track-events
[17] https://learn.microsoft.com/en-us/windows-hardware/test/wpt
[18] https://learn.microsoft.com/en-us/windows/win32/etw/event-tracing-tools
[19] https://developer.apple.com/documentation/xcode/reducing-your-app-s-launch-time
[20] https://developer.apple.com/documentation/xcode/improving-your-app-s-performance
[21] https://tokio.rs/tokio/tutorial/channels
[22] https://rfd.shared.oxide.computer/rfd/0400
[23] https://docs.rs/wgpu/latest/wgpu/type.RequestAdapterOptions.html
[24] https://docs.rs/wgpu/latest/wgpu/struct.Instance.html
[25] https://github.com/alacritty/vtebench
[26] https://man7.org/linux/man-pages/man5/proc_pid_status.5.html
[27] https://www.man7.org/linux/man-pages/man5/proc_pid_statm.5.html
[28] https://web.dev/articles/rail
[29] https://doc.rust-lang.org/cargo/reference/profiles.html
