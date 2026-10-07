# Paper experiment harness

This directory holds the deterministic recipe manifest, raw single-trial
schema, isolated-process runner, and Slice 4 pilot outputs. The runner uses
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

Each machine/recipe/repetition runs in a fresh Python subprocess. Change the
recipes/repetition count for the collection phase only after the pilot is
validated. Run output directories are temporary; the raw trial retains the
published flag and physical byte count, not a stale path to deleted files.
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
