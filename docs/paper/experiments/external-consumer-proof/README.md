# External consumer proofs

These deliberately small projects verify that the published GlassVM public
interfaces can be consumed without a GlassVM or machine source checkout.
They are run from a fresh temporary copy; the Python fixture fetcher downloads
the three pinned public workloads directly from upstream and verifies the
resulting workload SHA-256 values before execution.

## Python consumer

Requirements: `uv` and CPython 3.12 or newer. From the GlassVM checkout, run
the matrix driver. It copies only this consumer project into a fresh temporary
directory for each installation selection, then executes the consumer there:

```sh
bash docs/paper/experiments/external-consumer-proof/python/run_matrix.sh
```

The script prints its temporary output root and leaves the consumer project,
fixture bytes, virtual environments, and published runs there for inspection.
The provider installs are intentionally separate from the generic facade
dependency:

```sh
glassvm==0.2.0
glassvm-chip8==0.2.0
glassvm-pico8==0.2.0
glassvm-tic80==0.2.0
```

Each successful provider run prepares typed configuration and an empty typed
input schedule through the installed entry point, requests normalized events
and hash frames, then uses the shared Python facade to execute and publish
through the Rust recorder. The consumer checks execution, evidence, recorder,
and publication separately. Workload source revisions and expected output
hashes are pinned in `prepare_fixtures.py`; PICO-8's explicitly documented
title-only normalization is applied before its expected hash is checked.

## Rust consumer

The isolated Cargo project names the five published GlassVM crate versions
exactly, uses no path or Git dependencies, and includes a lockfile generated
outside the GlassVM Cargo workspace:

```sh
proof_root="$(mktemp -d)"
cp -R docs/paper/experiments/external-consumer-proof/rust "$proof_root/rust"
cd "$proof_root/rust"
cargo check --locked
cargo run --locked
```

The executable references public types from all five crates: core,
normalizer-contract, query, recorder, and registry.
