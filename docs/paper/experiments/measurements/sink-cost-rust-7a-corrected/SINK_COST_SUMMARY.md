# Rust sink-cost decomposition

Three repetitions at a fixed 60,000-frame workload per machine.
Lifecycle time includes machine execution plus the selected sink path; the recorder row includes finalization and publication.
The NullSink and recorder rows use identical configuration, schedule, controls, and observation request.
Process wall time is reported separately and includes process startup and preparation.
Source checkouts recorded clean: no (exploratory; regenerate from clean pinned revisions before paper use).

| Machine | Disabled (s) | Observation + NullSink (s) | Recorder (s) | Observation delta (s, %) | Recorder delta vs NullSink (s, %) | Recorder delta vs disabled (s, %) | Evidence logical (MiB) | Recorder encoded (MiB) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| chip8 | 0.072 | 6.219 | 16.504 | +6.147, +8537.6% | +10.285, +165.4% | +16.432, +22821.7% | 170.28 | 135.36 |
| pico8 | 33.087 | 34.598 | 38.432 | +1.511, +4.6% | +3.834, +11.1% | +5.345, +16.2% | 49.29 | 39.07 |
| tic80 | 10.606 | 15.080 | 19.959 | +4.474, +42.2% | +4.879, +32.4% | +9.352, +88.2% | 74.15 | 58.58 |

Process-wall medians (seconds; includes startup, preparation, and post-publication directory-size measurement):

| Machine | Disabled | Observation + NullSink | Recorder |
|---|---:|---:|---:|
| chip8 | 0.07 | 6.22 | 16.51 |
| pico8 | 33.09 | 34.60 | 38.43 |
| tic80 | 10.61 | 15.08 | 19.96 |

These are workload- and host-specific medians, not universal overhead factors. The disabled-to-NullSink contrast includes event/frame production and online normalization, while the NullSink-to-recorder contrast estimates the incremental recorder lifecycle cost. Percentages with a very small disabled baseline should be read with their absolute deltas. Recorder encoded volume is storage output, not process memory.
