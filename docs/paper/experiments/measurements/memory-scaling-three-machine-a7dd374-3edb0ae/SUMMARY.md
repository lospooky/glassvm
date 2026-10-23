# Run-length memory and evidence summary

All trials reached the requested frame count. Observation modes produced identical frame/cycle totals, boot outcome, and termination within each machine and run length.
Peak RSS is the per-process Linux `VmHWM`; three repetitions were used through 60,000 frames and one at 600,000 frames.
Evidence bytes are logical receipt totals. Encoded bytes are reported separately from finalized recorder receipts.

| Machine | Mode | Frames | Reps | Peak RSS median (MiB) | Runtime median (s) | Evidence logical (MiB) | Recorder encoded (MiB) |
|---|---|---:|---:|---:|---:|---:|---:|
| chip8 | disabled | 600 | 3 | 5.00 | 0.00 | 0.00 | — |
| chip8 | disabled | 6,000 | 3 | 5.01 | 0.01 | 0.00 | — |
| chip8 | disabled | 60,000 | 3 | 5.08 | 0.07 | 0.00 | — |
| chip8 | disabled | 600,000 | 1 | 5.02 | 0.72 | 0.00 | — |
| chip8 | events_null | 600 | 3 | 5.05 | 0.06 | 1.40 | — |
| chip8 | events_null | 6,000 | 3 | 5.07 | 0.57 | 14.12 | — |
| chip8 | events_null | 60,000 | 3 | 5.11 | 5.43 | 142.43 | — |
| chip8 | events_null | 600,000 | 1 | 5.11 | 54.30 | 1436.59 | — |
| chip8 | normalizer_null | 600 | 3 | 5.00 | 0.07 | 1.67 | — |
| chip8 | normalizer_null | 6,000 | 3 | 5.01 | 0.62 | 16.88 | — |
| chip8 | normalizer_null | 60,000 | 3 | 5.07 | 6.18 | 170.28 | — |
| chip8 | normalizer_null | 600,000 | 1 | 5.02 | 62.16 | 1717.34 | — |
| chip8 | recorder | 600 | 3 | 5.81 | 0.19 | 1.67 | 1.33 |
| chip8 | recorder | 6,000 | 3 | 6.35 | 1.66 | 16.88 | 13.37 |
| chip8 | recorder | 60,000 | 3 | 6.03 | 16.36 | 170.28 | 135.36 |
| chip8 | recorder | 600,000 | 1 | 6.04 | 160.42 | 1717.34 | 1364.33 |
| pico8 | disabled | 600 | 3 | 5.27 | 0.27 | 0.00 | — |
| pico8 | disabled | 6,000 | 3 | 5.37 | 3.17 | 0.00 | — |
| pico8 | disabled | 60,000 | 3 | 5.34 | 32.54 | 0.00 | — |
| pico8 | disabled | 600,000 | 1 | 5.41 | 323.49 | 0.00 | — |
| pico8 | events_null | 600 | 3 | 5.28 | 0.28 | 0.20 | — |
| pico8 | events_null | 6,000 | 3 | 5.55 | 3.26 | 2.04 | — |
| pico8 | events_null | 60,000 | 3 | 5.58 | 33.42 | 20.73 | — |
| pico8 | events_null | 600,000 | 1 | 5.54 | 335.57 | 210.15 | — |
| pico8 | normalizer_null | 600 | 3 | 5.27 | 0.28 | 0.48 | — |
| pico8 | normalizer_null | 6,000 | 3 | 5.54 | 3.33 | 4.88 | — |
| pico8 | normalizer_null | 60,000 | 3 | 5.55 | 34.23 | 49.29 | — |
| pico8 | normalizer_null | 600,000 | 1 | 5.49 | 342.53 | 498.09 | — |
| pico8 | recorder | 600 | 3 | 6.04 | 0.34 | 0.48 | 0.38 |
| pico8 | recorder | 6,000 | 3 | 6.60 | 3.75 | 4.88 | 3.88 |
| pico8 | recorder | 60,000 | 3 | 6.59 | 38.33 | 49.29 | 39.07 |
| pico8 | recorder | 600,000 | 1 | 6.54 | 376.88 | 498.09 | 398.07 |
| tic80 | disabled | 600 | 3 | 5.12 | 0.10 | 0.00 | — |
| tic80 | disabled | 6,000 | 3 | 5.00 | 1.07 | 0.00 | — |
| tic80 | disabled | 60,000 | 3 | 5.12 | 10.48 | 0.00 | — |
| tic80 | disabled | 600,000 | 1 | 5.12 | 105.14 | 0.00 | — |
| tic80 | events_null | 600 | 3 | 5.12 | 0.11 | 0.45 | — |
| tic80 | events_null | 6,000 | 3 | 5.32 | 1.13 | 4.60 | — |
| tic80 | events_null | 60,000 | 3 | 5.21 | 11.11 | 46.52 | — |
| tic80 | events_null | 600,000 | 1 | 5.27 | 110.78 | 470.95 | — |
| tic80 | normalizer_null | 600 | 3 | 5.13 | 0.15 | 0.73 | — |
| tic80 | normalizer_null | 6,000 | 3 | 5.25 | 1.49 | 7.34 | — |
| tic80 | normalizer_null | 60,000 | 3 | 5.22 | 14.92 | 74.15 | — |
| tic80 | normalizer_null | 600,000 | 1 | 5.24 | 148.95 | 749.54 | — |
| tic80 | recorder | 600 | 3 | 6.16 | 0.23 | 0.73 | 0.58 |
| tic80 | recorder | 6,000 | 3 | 6.12 | 2.02 | 7.34 | 5.82 |
| tic80 | recorder | 60,000 | 3 | 6.19 | 20.12 | 74.15 | 58.58 |
| tic80 | recorder | 600,000 | 1 | 6.08 | 200.66 | 749.54 | 596.83 |

## Figures

- `memory-vs-run-length.svg`
- `evidence-volume-vs-run-length.svg`

## Interpretation

chip8: peak RSS stayed within approximately 1.5× between the shortest and longest measured runs in every mode. disabled 5.00→5.02 MiB; events_null 5.05→5.11 MiB; normalizer_null 5.00→5.02 MiB; recorder 5.81→6.04 MiB.

pico8: peak RSS stayed within approximately 1.5× between the shortest and longest measured runs in every mode. disabled 5.27→5.41 MiB; events_null 5.28→5.54 MiB; normalizer_null 5.27→5.49 MiB; recorder 6.04→6.54 MiB.

tic80: peak RSS stayed within approximately 1.5× between the shortest and longest measured runs in every mode. disabled 5.12→5.12 MiB; events_null 5.12→5.27 MiB; normalizer_null 5.13→5.24 MiB; recorder 6.16→6.08 MiB.

The CHIP-8 cleanup removed retained per-frame hash/metric histories and whole-run native event accumulation. Its engine now keeps scalar event counters; selected frame-derived motifs are produced by the online bundle normalizer from requested frame artifacts. The measurement supports bounded observed RSS for these workloads and this host, not a general proof; allocator profiling was unavailable.

Recorder encoded bytes grow with run length and are reported separately from RSS. The 600,000-frame endpoint has one repetition; the smaller lengths have three. Results are specific to the listed fixtures/configurations and this WSL2 host, not a cross-host benchmark.
