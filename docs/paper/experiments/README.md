# Paper experiment harness

This directory holds the deterministic recipe manifest, raw single-trial
schema, isolated-process runner, Slice 4 pilot, and repeated Slice 5
selectable-observation measurements. The runner uses
only installed `glassvm.machine_bundles` providers through:

```python
runtime = glassvm.Runtime.discover()
prepared = runtime.prepare(...)
result = prepared.execute(output_path)
```

The bundle validates and resolves the typed request during preparation. The
Rust `FileRunSession` remains responsible for sink assembly, budgets,
execution, evidence receipt, recorder finalization, and publication. The
measurement runner does not decode segment records or materialize event
history. It reads only the bounded manifest/receipt metadata and stats the
published directory before deleting the temporary run directory.

Observation recipes are exact requests in
[`OBSERVATION_RECIPES.toml`](OBSERVATION_RECIPES.toml). Workload-specific event
kinds and capabilities resolve from [`../PAPER_WORKLOADS.toml`](../PAPER_WORKLOADS.toml).
Missing provider/unsupported recipe cells are written as explicit exclusions;
the runner never substitutes another request.

## Environment and invocation

Use Python 3.12 or newer with the base `glassvm` wheel and the selected
`glassvm-chip8`, `glassvm-pico8`, and/or `glassvm-tic80` wheel installed. The
machine fixture root is the `glassvm-machines` repository. For example:

```sh
uv run --python 3.14 --with glassvm==0.1.2 --with glassvm-chip8==0.1.2 \
  --with glassvm-pico8==0.1.2 --with glassvm-tic80==0.1.2 \
  python docs/paper/experiments/run_pilot.py \
    --glassvm-root "$PWD" \
    --machines-root ../glassvm-machines \
    --output-dir docs/paper/experiments/pilot \
    --repetitions 1 --frame-limit 600
```

Each machine/recipe/repetition runs in a fresh Python subprocess. The 27-cell
Slice 4 pilot is a harness/accounting check only. Slice 5 uses five
independent repetitions at 600 frames and a seeded shuffled trial order, stored
separately under `measurements/`; it is a fixed-workload observation-cost
comparison, not bounded-memory evidence. Reproduce it with:

```sh
uv run --python 3.14 --with glassvm==0.1.2 --with glassvm-chip8==0.1.2 \
  --with glassvm-pico8==0.1.2 --with glassvm-tic80==0.1.2 \
  python docs/paper/experiments/run_pilot.py \
    --glassvm-root "$PWD" \
    --machines-root ../glassvm-machines \
    --output-dir docs/paper/experiments/measurements \
    --manifest-name measurement_manifest.json \
    --repetitions 5 --frame-limit 600
uv run --python 3.14 python docs/paper/experiments/generate_observation_summary.py \
  --measurement-dir docs/paper/experiments/measurements
```

Run output directories are temporary; the raw trial retains the published flag
and physical byte count, not a stale path to deleted files.
The runner snapshots both checkout revisions and dirty states before writing
trial files, so the pilot's own outputs cannot mark later trials as dirty.
The default selects every recipe in the manifest. Regenerate and check the
summary from saved raw results with:

```sh
python docs/paper/experiments/generate_pilot_summary.py \
  --pilot-dir docs/paper/experiments/pilot
python docs/paper/experiments/generate_pilot_summary.py \
  --pilot-dir docs/paper/experiments/pilot --check
```

Preparation errors, worker crashes, and unavailable providers are retained as
versioned `failed` or `excluded` rows rather than silently omitted.

## Result accounting

`result.schema.json` defines the raw-run envelope. Each trial retains the four
outcome domains separately: machine execution result, evidence receipt,
recorder receipt, and publication result. Channel receipts provide requested
and fulfilled item/logical-byte counts; recorder receipts provide per-channel
segment, record, block, logical-byte, and encoded-byte counters when the
installed recorder implements that contract. Physical directory bytes are a
separate filesystem measurement and include metadata/framing files.

Timing is split into provider discovery, preparation, execute-and-publish,
worker process, and lifecycle intervals. `execute_and_publish_seconds` is not
presented as pure emulator time. Peak RSS is process high-water memory, not an
allocation counter. Allocator high-water is explicitly null because the
provider protocol exposes no portable allocator statistic.

`PreparedRun` remains opaque; these experiments do not require an identity
accessor. Reproducibility is recorded through the artifact digest, explicit
configuration, input schedule, execution controls, provider/package metadata,
exact observation request, and the `PreparedObservation` identity recovered
from published manifest metadata when available. No identity is synthesized
from raw Python objects.

The raw record distinguishes checkout references from installed package
provenance. The provider entry point and installed distribution versions are
known, but the current wheel metadata does not embed a source commit, so the
runner leaves `installed_distribution_source_revision` null rather than
attributing the current checkout to those binaries.

The result schema is versioned independently from recorder and evidence
schemas. A changed field meaning requires a result-schema version bump and
regeneration of all dependent tables/figures.

## Slice 6: run-length and memory scaling

`run_memory_experiment.py` drives one release-mode Rust process per trial so
peak RSS is measured independently of Python/provider startup. The experimental
`paper_memory_trial` example lives in the companion `glassvm-machines` checkout
under `conformance/examples/`; the runner builds it with `--locked` before the
matrix. GlassVM contract and recorder crates resolve through their published
versions in the machine workspace lockfile.

The fixed matrix uses the same artifact, configuration, and empty input
schedule per machine at 600, 6,000, 60,000, and 600,000 requested frames. It
compares disabled observation, selected normalized events to `NullSink`, those
events plus hash frames and the declared visual capability to `NullSink`, and
that same visual request through the segmented file recorder. Three
repetitions are used through 60,000 frames and one at 600,000. Recorder output
is measured in a fresh temporary directory and removed after its receipts and
physical bytes are recorded.

For the final paper run, check out the exact GlassVM and machine-repository
commits intended for citation, and keep both checkouts clean. The runner records
the full commit IDs, exact tags when present, Rust/Cargo versions, machine
workspace lockfile digest, trial-binary digest, fixture digests, and host
metadata. The output directory must be outside both checkouts so writing results
does not make a source checkout dirty. Resolve the chosen refs to full commit
IDs and pass them explicitly:

```sh
GLASSVM_REV=$(git -C . rev-parse HEAD)
MACHINES_REV=$(git -C ../glassvm-machines rev-parse HEAD)
uv run --python 3.14 --no-project python \
  docs/paper/experiments/run_memory_experiment.py \
  --paper-root docs/paper \
  --machines-root ../glassvm-machines \
  --output-dir ../glassvm-paper-results/memory-scaling \
  --require-clean \
  --expected-glassvm-revision "$GLASSVM_REV" \
  --expected-machines-revision "$MACHINES_REV"
```

The runner builds the release example with Cargo's lockfile enforced. It refuses
dirty checkouts, unexpected revisions, and output directories whose saved
source/matrix metadata differs from the current invocation. An interrupted run
can be resumed only against the identical source, binary, workload, and matrix;
use a new output directory for any changed revision or settings. Do not reuse
the current dirty-checkout measurements as final results.
Regenerate validation, `SUMMARY.md`, `summary.json`, and both plots with:

```sh
uv run --python 3.14 --no-project --with matplotlib python \
  docs/paper/experiments/summarize_memory_experiment.py \
  --data-dir ../glassvm-paper-results/memory-scaling
```

The original three-machine matrix exposed CHIP-8 growth: disabled-observation
RSS rose from 5.65 MiB at 600 frames to 113.64 MiB at 600,000. The core
retained frame histories, metric samples, and native events. Those surfaces
have since been removed; CHIP-8 now counts scalar event facts online, and
frame-derived motifs are computed by its bounded bundle normalizer. The
CHIP-8-only 40-trial follow-up is in
[`measurements/chip8-retention-cleanup/`](measurements/chip8-retention-cleanup/):
all modes measured about 5.0–6.1 MiB from 600 to 600,000 frames on this host.
The full three-machine matrix still needs a same-source-coordinate rerun before
we use it to support a cross-bundle memory claim. Peak RSS is Linux `VmHWM`,
allocator high-water is unavailable, and the longest point has one repetition.
Both datasets, manifests, summaries, and SVG plots are retained independently.

## Slice 7a: Rust sink-cost decomposition

The focused run uses three repetitions at 60,000 frames for each of CHIP-8,
PICO-8, and TIC-80, comparing observation disabled, the fixed visual/event
request with `NullSink`, and that exact request with the segmented recorder.
The trial uses one mode-independent run ID per machine/repetition so receipt
accounting is comparable. The summarizer verifies matching artifact,
configuration, schedule, execution controls, run identity, execution outcome,
observation request, and `EvidenceReceipt` between NullSink and recorder.

Build and generate new trials (the source coordinate is captured in the
manifest; use a fresh directory for a changed checkout):

```sh
uv run --python 3.14 --no-project python \
  docs/paper/experiments/run_memory_experiment.py \
  --paper-root docs/paper \
  --machines-root ../glassvm-machines \
  --output-dir docs/paper/experiments/measurements/sink-cost-rust-7a \
  --frames 60000 --repetitions 3 \
  --mode disabled --mode normalizer_null --mode recorder
uv run --python 3.14 --no-project python \
  docs/paper/experiments/summarize_sink_cost.py \
  --data-dir docs/paper/experiments/measurements/sink-cost-rust-7a
```

Lifecycle time measures execution plus its selected sink path; recorder time
includes finalization/publication. Process-wall time is separate and includes
startup/preparation and the post-publication directory-size measurement. The
disabled-to-NullSink delta includes evidence production and normalization; it
is not a sink-only cost. The NullSink-to-recorder delta is the incremental
recorder lifecycle estimate. Treat these as workload/host-specific, and rerun
from clean pinned source revisions before final paper use.

The current exploratory matrix and generated table are in
[`measurements/sink-cost-rust-7a-corrected/`](measurements/sink-cost-rust-7a-corrected/).
The first pass is retained but explicitly invalidated because its mode-bearing
run IDs skewed logical-byte accounting.
