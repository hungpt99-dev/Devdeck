# Deliverable 6 — Performance strategy

## Claim discipline

No performance numbers are claimed. Existing local evidence: Iced 0.14 spike tests 2/2, Xvfb window, release app binary 15 MB stripped; cold start, RSS and frame latency are unmeasured. All figures below are **proposed targets**, following the research report's explicit claim discipline.

| Metric | Proposed target |
| --- | --- |
| Cold start core / with 20 extensions | <=500 ms to first interactive frame / <1 s, extension host <=150 ms |
| Idle core RAM / 10 workspaces open | <150 MB RSS / <400 MB summed UI+sidecars |
| Keystroke-to-screen | p95 <16 ms at 60 Hz; <=8.3 ms at 120 Hz |
| 100k-row grid | p95 frame <=16.7 ms, p99 <=33 ms, no frame >50 ms in 10 s |
| 50 MB file / response | first window <=2 s, peak RSS delta <=2x input, no UI span >50 ms |
| Terminal throughput | >=100 MB/s PTY drain median at 200x50 |
| WASM call / sidecar IPC | p95 <=1 ms / <=2 ms warmed |
| Binary size | <=25 MB stripped release |
| GPU / software fallback | 60/120 fps; >=30 fps at 1200x800 software |

## Process isolation and memory ownership

Latency-critical typed state (UI, active rope buffer, terminal screen model, grid window, palette, i18n, permission cache) stays in the UI process as immutable `Arc` snapshots. All file/network/PTY I/O, parsing and indexing run on tokio workers, never the UI thread. Untrusted or crash-prone work goes to supervised sidecars: one PTY child per terminal, a wasmtime host with per-extension Store and epoch/fuel deadlines, and external LSP/DAP/MCP/DB/Docker processes[345]. Bounded channels with coalescing provide backpressure; long jobs are cancellable at chunk boundaries.

## Measurement methodology

```bash
cargo bench --workspace
cargo bench -p devdeck-bench -- grid_window --measurement-time 10 --warm-up-time 5 --sample-size 200
hyperfine --export-json target/perf/cold-start.json --runs 30 'target/release/devdeck-app --bench-exit'
# Linux cold runs additionally use `sync; echo 3 | sudo tee /proc/sys/vm/drop_caches`
grep -E '^(VmRSS|VmHWM|RssAnon):' /proc/$PID/status && cat /proc/$PID/smaps_rollup
cat /proc/pressure/cpu /proc/pressure/memory /proc/pressure/io
perf stat -e cycles,instructions,cache-misses,page-faults -- target/release/devdeck-app --bench-scenario grid-100k
xcrun xctrace record --template 'App Launch' --launch -- ./target/release/devdeck-app   # macOS
wpr -start CPU -filemode; wpr -stop target/perf/devdeck.etl                            # Windows
```

`criterion` supplies statistically analysed microbenchmarks[609]; `hyperfine` handles command-level warm/cold starts[612]; PSI and `perf` cover kernel-level pressure and CPU[614][616]. Frame-time histograms must accompany terminal/grid tests: PTY throughput alone does not prove smooth rendering.

## CI gates

Store baselines per OS and reference hardware; fail on >10% regression of p95 metrics with reruns and noise bands; track binary size and RSS of UI+sidecars separately. Pin reference hardware and OS versions before treating any number as a commitment. Wasmtime epoch interruption has a cost, so extension-call budgets must be re-measured after isolation is enabled.

Full methodology and commands: `research/performance.md`.

## Sources

[345] https://docs.wasmtime.dev/api/wasmtime/component
[609] https://criterion-rs.github.io/book/user_guide/analysis.html
[612] https://github.com/sharkdp/hyperfine
[614] https://kernel.org/doc/Documentation/accounting/psi.txt
[616] https://perfwiki.github.io/main
