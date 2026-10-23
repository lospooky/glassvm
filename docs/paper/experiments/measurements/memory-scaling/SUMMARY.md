# Run-length memory and evidence summary

All trials reached the requested frame count. Observation modes produced identical execution frame/cycle totals within each machine and run length.
Peak RSS is the per-process Linux `VmHWM`; three repetitions were used through 60,000 frames and one at 600,000 frames.
Evidence bytes are logical receipt totals. Encoded bytes are reported separately from finalized recorder receipts.

| Machine | Mode | Frames | Reps | Peak RSS median (MiB) | Runtime median (s) | Evidence logical (MiB) | Recorder encoded (MiB) |
|---|---|---:|---:|---:|---:|---:|---:|
| chip8 | disabled | 600 | 3 | 5.65 | 0.03 | 0.00 | — |
| chip8 | disabled | 6,000 | 3 | 6.63 | 0.29 | 0.00 | — |
| chip8 | disabled | 60,000 | 3 | 15.28 | 2.82 | 0.00 | — |
| chip8 | disabled | 600,000 | 1 | 113.64 | 28.58 | 0.00 | — |
| chip8 | events_null | 600 | 3 | 5.73 | 0.09 | 1.43 | — |
| chip8 | events_null | 6,000 | 3 | 6.54 | 0.87 | 14.46 | — |
| chip8 | events_null | 60,000 | 3 | 15.00 | 8.41 | 145.86 | — |
| chip8 | events_null | 600,000 | 1 | 121.25 | 85.75 | 1470.89 | — |
| chip8 | normalizer_null | 600 | 3 | 5.55 | 0.10 | 1.73 | — |
| chip8 | normalizer_null | 6,000 | 3 | 6.45 | 0.92 | 17.43 | — |
| chip8 | normalizer_null | 60,000 | 3 | 15.14 | 9.18 | 175.72 | — |
| chip8 | normalizer_null | 600,000 | 1 | 113.47 | 91.58 | 1771.74 | — |
| chip8 | recorder | 600 | 3 | 6.33 | 0.23 | 1.71 | 1.36 |
| chip8 | recorder | 6,000 | 3 | 7.69 | 1.97 | 17.22 | 13.70 |
| chip8 | recorder | 60,000 | 3 | 16.28 | 19.52 | 173.60 | 138.69 |
| chip8 | recorder | 600,000 | 1 | 116.03 | 196.13 | 1750.58 | 1397.58 |
| pico8 | disabled | 600 | 3 | 5.37 | 0.28 | 0.00 | — |
| pico8 | disabled | 6,000 | 3 | 5.35 | 3.29 | 0.00 | — |
| pico8 | disabled | 60,000 | 3 | 5.47 | 33.26 | 0.00 | — |
| pico8 | disabled | 600,000 | 1 | 5.33 | 328.68 | 0.00 | — |
| pico8 | events_null | 600 | 3 | 5.20 | 0.28 | 0.21 | — |
| pico8 | events_null | 6,000 | 3 | 5.63 | 3.36 | 2.12 | — |
| pico8 | events_null | 60,000 | 3 | 5.60 | 34.15 | 21.53 | — |
| pico8 | events_null | 600,000 | 1 | 5.46 | 340.21 | 218.16 | — |
| pico8 | normalizer_null | 600 | 3 | 5.24 | 0.29 | 0.50 | — |
| pico8 | normalizer_null | 6,000 | 3 | 5.56 | 3.39 | 5.08 | — |
| pico8 | normalizer_null | 60,000 | 3 | 5.61 | 34.62 | 51.35 | — |
| pico8 | normalizer_null | 600,000 | 1 | 5.58 | 353.19 | 518.69 | — |
| pico8 | recorder | 600 | 3 | 6.07 | 0.35 | 0.50 | 0.40 |
| pico8 | recorder | 6,000 | 3 | 6.43 | 3.78 | 5.00 | 4.01 |
| pico8 | recorder | 60,000 | 3 | 6.48 | 38.33 | 50.55 | 40.33 |
| pico8 | recorder | 600,000 | 1 | 6.44 | 386.41 | 510.68 | 410.66 |
| tic80 | disabled | 600 | 3 | 5.09 | 0.11 | 0.00 | — |
| tic80 | disabled | 6,000 | 3 | 5.13 | 1.07 | 0.00 | — |
| tic80 | disabled | 60,000 | 3 | 5.12 | 10.84 | 0.00 | — |
| tic80 | disabled | 600,000 | 1 | 5.12 | 104.39 | 0.00 | — |
| tic80 | events_null | 600 | 3 | 5.20 | 0.12 | 0.47 | — |
| tic80 | events_null | 6,000 | 3 | 5.10 | 1.13 | 4.76 | — |
| tic80 | events_null | 60,000 | 3 | 5.16 | 11.80 | 48.12 | — |
| tic80 | events_null | 600,000 | 1 | 5.16 | 112.65 | 486.97 | — |
| tic80 | normalizer_null | 600 | 3 | 5.15 | 0.15 | 0.76 | — |
| tic80 | normalizer_null | 6,000 | 3 | 5.20 | 1.53 | 7.64 | — |
| tic80 | normalizer_null | 60,000 | 3 | 5.21 | 15.02 | 77.24 | — |
| tic80 | normalizer_null | 600,000 | 1 | 5.29 | 152.45 | 780.44 | — |
| tic80 | recorder | 600 | 3 | 6.02 | 0.23 | 0.74 | 0.60 |
| tic80 | recorder | 6,000 | 3 | 5.93 | 2.08 | 7.52 | 6.01 |
| tic80 | recorder | 60,000 | 3 | 6.06 | 20.39 | 76.04 | 60.47 |
| tic80 | recorder | 600,000 | 1 | 6.09 | 202.97 | 768.43 | 615.71 |

## Figures

- `memory-vs-run-length.svg`
- `evidence-volume-vs-run-length.svg`

## Interpretation

PICO-8 and TIC-80 peak RSS remained within a narrow range from 600 through 600,000 frames for all four modes on this host. CHIP-8 did not: disabled-observation RSS rose from 5.65 MiB at 600 frames to 113.64 MiB at 600,000; the other CHIP-8 modes show the same run-length trend. Because the increase is present with observation disabled, this is not evidence of reducer or recorder retention alone.

The CHIP-8 engine retains per-frame `frame_hash_history`, a `unique_frame_hashes` set, and per-transition vectors in `FrameMetricsAccumulator` (including changed-pixel/region samples and topology samples). These legacy metrics/history are updated even when observation is disabled. Source inspection identifies these as unbounded run-length state; allocator profiling was unavailable, so their individual RSS contributions were not isolated. End-to-end run-length-independent memory is therefore not supported for the three-machine release set by this experiment; the CHIP-8 retention path must be bounded or excluded explicitly before making that claim.

Recorder encoded bytes grow with run length and are reported separately from RSS. The 600,000-frame endpoint has one repetition; the smaller lengths have three. Results are specific to the listed fixtures/configurations and this WSL2 host, not a cross-host benchmark.
