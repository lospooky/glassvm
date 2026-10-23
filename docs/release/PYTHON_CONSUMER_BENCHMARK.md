# Python emission consumer conformance and benchmark

The recorder crate owns the synchronous Python boundary: Rust emissions are
converted to owned values, buffered under item and logical-byte limits, then
delivered to consumer callbacks. The bridge exposes its actual peak pending
batch occupancy in `PythonSinkDeliveryOutcome`; the counters describe this
transient emission batch, not a retained run projection.

## Conformance

With the `python` feature enabled, recorder tests exercise a third-party-style
Python object implementing the three documented callbacks. They check that:

- every typed emission channel reaches `emit_batch` as an owned value;
- capability outputs and capability receipts remain distinct callbacks;
- batches obey the configured item bound and the bridge reports its actual
  peak buffered items and logical bytes;
- the Python consumer can run alongside the Rust reference recorder and still
  reach recorder finalization and atomic publication; and
- a raised Python exception becomes a structured Python delivery failure,
  without being labeled as a machine fault or recorder-format failure.

Run the conformance tests with:

```sh
cargo test -p glassvm_recorder --features python -- --test-threads=1
```

## Reproducible benchmark harness

The synthetic workload emits a configurable number of normalized events. Each
event has a 16 KiB payload so the byte-bound case exercises a meaningful
bounded-buffer limit. The harness compares:

- observation disabled;
- selected observation with the core `NullSink`;
- selected observation with the Rust reference recorder;
- selected observation with an embedded Python batched callback at item limits
  1, 8, 32, and 128, with a high byte ceiling; and
- Python batching plus the Rust reference recorder, using the default policy.

The byte-bounded row sets an effectively unbounded item limit and uses the
default Python logical-byte limit. Thus item-count and byte-count behavior can
be examined independently. The harness does not change the default batching
policy; select API defaults only after benchmarking representative real
machine runs.

Run it with:

```sh
cargo bench -p glassvm_recorder --features python --bench python_emission_consumer
```

Optional environment variables:

```sh
GLASSVM_BENCH_EVENTS=1000
GLASSVM_BENCH_REPETITIONS=1
```

The executable writes CSV to stdout. Columns include elapsed wall time and
ratio to the observation-disabled row, delivered batch/item/logical-byte
counters, actual peak buffered item/logical-byte counts, and process high-water
RSS when `/proc/self/status` is available. RSS is process-wide high-water, not
a per-row attribution; compare it only across separate fresh process runs.
The benchmark is a bridge/lifecycle microbenchmark, not a machine-emulator
throughput claim. Its timings should not be used as paper results without a
controlled runner, repeated runs, and the selected machine workloads.

For the installed public facade/provider path and the CHIP-8, PICO-8, and
TIC-80 paper workloads, use
[`../paper/experiments/run_python_sink_cost.py`](../paper/experiments/run_python_sink_cost.py).
That experiment compares observation-disabled, observed/no-external-consumer,
Python consumer, reference recorder, and Python-plus-recorder modes in
independent processes. It records actual consumer batch accounting and checks
that machine outcomes and selected-observation evidence receipts stay equal
across observed modes. The short implementation smoke run is not paper-grade;
the checked-in 7b command uses repeated 60,000-frame trials from clean pinned
builds.
