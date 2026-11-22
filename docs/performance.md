# Performance budget

| Metric | Target | Measured now |
| --- | --- | --- |
| Cold start with 20 extensions | < 1 s | NOT MEASURED (no extensions yet) |
| Idle core RAM | < 150 MB | NOT MEASURED |
| Keystroke-to-screen (editor/terminal) | < 16 ms | NOT MEASURED |
| Editor 50 MB file | no freeze | NOT IMPLEMENTED |
| 100k-row grid / 50 MB response | smooth scroll | spike windowing math tested; not measured |
| 60/120 fps | target | NOT MEASURED |

Budgets are enforced in CI from Phase 7 with criterion benchmarks and regression gates (>10% regression fails the build). Do not claim any number before the benchmark command and output are recorded here.
