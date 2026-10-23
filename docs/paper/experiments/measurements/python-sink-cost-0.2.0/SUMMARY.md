# Python consumer sink-cost experiment

This is a host/workload-specific end-to-end lifecycle comparison. The
`observed_no_external_sink` row is not a direct `NullSink` microbenchmark;
the public provider lifecycle still applies the shared budgeted bridge.
Recorder rows include finalization and atomic publication.

| Machine | Mode | Median lifecycle (s) | Delta vs observed/no external sink (s) | Delta (%) |
|---|---|---:|---:|---:|
| chip8 | observation_disabled | 0.074 | -6.165 | -98.8% |
| chip8 | observed_no_external_sink | 6.239 | 0.000 | 0.0% |
| chip8 | python_consumer | 8.196 | 1.957 | 31.4% |
| chip8 | rust_recorder | 16.259 | 10.020 | 160.6% |
| chip8 | python_consumer_and_rust_recorder | 18.226 | 11.987 | 192.1% |
| pico8 | observation_disabled | 32.354 | -1.144 | -3.4% |
| pico8 | observed_no_external_sink | 33.497 | 0.000 | 0.0% |
| pico8 | python_consumer | 33.939 | 0.442 | 1.3% |
| pico8 | rust_recorder | 38.691 | 5.194 | 15.5% |
| pico8 | python_consumer_and_rust_recorder | 38.213 | 4.715 | 14.1% |
| tic80 | observation_disabled | 10.244 | -4.130 | -28.7% |
| tic80 | observed_no_external_sink | 14.375 | 0.000 | 0.0% |
| tic80 | python_consumer | 15.529 | 1.155 | 8.0% |
| tic80 | rust_recorder | 19.424 | 5.049 | 35.1% |
| tic80 | python_consumer_and_rust_recorder | 20.691 | 6.316 | 43.9% |

Raw trials record per-run batch/item/logical-byte counts, peak bounded
batch size, RSS high-water where available, receipts, provider metadata,
and both checkout revisions. Regenerate from clean pinned builds before
using these values as paper evidence.
