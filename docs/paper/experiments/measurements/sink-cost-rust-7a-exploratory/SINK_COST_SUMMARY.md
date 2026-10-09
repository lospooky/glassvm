# Rust sink-cost decomposition

Three repetitions at a fixed 60,000-frame workload per machine.
Lifecycle time includes machine execution plus the selected sink path; the recorder row includes finalization and publication.
The NullSink and recorder rows use identical configuration, schedule, controls, and observation request.
Process wall time is reported separately and includes process startup and preparation.

| Machine | Disabled lifecycle (s) | Observation + NullSink (s) | Recorder lifecycle (s) | Observation vs disabled | Recorder vs NullSink | Recorder vs disabled | Evidence logical (MiB) | Recorder encoded (MiB) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| chip8 | 0.074 | 6.266 | 16.444 | +8381.0% | +162.4% | +22156.2% | 173.60 | 138.69 |
| pico8 | 32.499 | 34.072 | 38.187 | +4.8% | +12.1% | +17.5% | 50.55 | 40.33 |
| tic80 | 10.560 | 14.692 | 19.811 | +39.1% | +34.8% | +87.6% | 76.04 | 60.47 |

These are workload- and host-specific medians, not universal overhead factors. The disabled-to-NullSink contrast includes event/frame production and online normalization, while the NullSink-to-recorder contrast estimates the incremental recorder lifecycle cost. Recorder encoded volume is storage output, not process memory.
