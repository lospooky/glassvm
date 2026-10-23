# Run-length memory and evidence summary

All trials reached the requested frame count. Observation modes produced identical execution frame/cycle totals within each machine and run length.
Peak RSS is the per-process Linux `VmHWM`; three repetitions were used through 60,000 frames and one at 600,000 frames.
Evidence bytes are logical receipt totals. Encoded bytes are reported separately from finalized recorder receipts.

| Machine | Mode | Frames | Reps | Peak RSS median (MiB) | Runtime median (s) | Evidence logical (MiB) | Recorder encoded (MiB) |
|---|---|---:|---:|---:|---:|---:|---:|
| chip8 | disabled | 600 | 3 | 5.15 | 0.00 | 0.00 | — |
| chip8 | disabled | 6,000 | 3 | 5.11 | 0.01 | 0.00 | — |
| chip8 | disabled | 60,000 | 3 | 5.11 | 0.08 | 0.00 | — |
| chip8 | disabled | 600,000 | 1 | 5.10 | 0.75 | 0.00 | — |
| chip8 | events_null | 600 | 3 | 5.03 | 0.06 | 1.43 | — |
| chip8 | events_null | 6,000 | 3 | 5.11 | 0.56 | 14.46 | — |
| chip8 | events_null | 60,000 | 3 | 5.09 | 5.55 | 145.86 | — |
| chip8 | events_null | 600,000 | 1 | 5.07 | 56.74 | 1470.89 | — |
| chip8 | normalizer_null | 600 | 3 | 5.08 | 0.06 | 1.73 | — |
| chip8 | normalizer_null | 6,000 | 3 | 5.13 | 0.64 | 17.43 | — |
| chip8 | normalizer_null | 60,000 | 3 | 5.07 | 6.33 | 175.72 | — |
| chip8 | normalizer_null | 600,000 | 1 | 5.02 | 62.55 | 1771.74 | — |
| chip8 | recorder | 600 | 3 | 5.92 | 0.19 | 1.71 | 1.36 |
| chip8 | recorder | 6,000 | 3 | 6.04 | 1.70 | 17.22 | 13.70 |
| chip8 | recorder | 60,000 | 3 | 5.84 | 16.74 | 173.60 | 138.69 |
| chip8 | recorder | 600,000 | 1 | 6.05 | 167.69 | 1750.58 | 1397.58 |

## Figures

- `memory-vs-run-length.svg`
- `evidence-volume-vs-run-length.svg`

## Interpretation

chip8: peak RSS stayed within approximately 1.5× between the shortest and longest measured runs in every mode. disabled 5.15→5.10 MiB; events_null 5.03→5.07 MiB; normalizer_null 5.08→5.02 MiB; recorder 5.92→6.05 MiB.

The CHIP-8 cleanup removed retained per-frame hash/metric histories and whole-run native event accumulation. Its engine now keeps scalar event counters; selected frame-derived motifs are produced by the online bundle normalizer from requested frame artifacts. The measurement supports bounded observed RSS for these workloads and this host, not a general proof; allocator profiling was unavailable.

Recorder encoded bytes grow with run length and are reported separately from RSS. The 600,000-frame endpoint has one repetition; the smaller lengths have three. Results are specific to the listed fixtures/configurations and this WSL2 host, not a cross-host benchmark.
