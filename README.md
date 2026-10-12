# GlassVM

GlassVM provides machine-neutral contracts for running machine bundles and
selecting the evidence a run should produce. A bundle supplies the machine's
semantics; GlassVM provides shared execution, input, observation, and evidence
interfaces.

Machine implementations are maintained in the separate
[`glassvm-machines`](https://github.com/lospooky/glassvm-machines) repository.
This repository contains the shared Rust crates and a Python facade that finds
installed bundles through Python entry points.

## What is in this repository

| Path | Purpose |
| --- | --- |
| `crates/core` | Machine, execution, input, observation, event, and evidence contracts. |
| `crates/normalizer-contract` | Shared capability and normalizer catalog types. |
| `crates/recorder` | Bounded file recording and run publication. |
| `crates/query` | In-memory projections over caller-supplied execution events. |
| `crates/registry` | Explicit Rust registration and validation of machine bundles. |
| `python/facade` | Python runtime for discovering and calling installed bundle providers. |

GlassVM does not include a built-in machine implementation. The generic Python
facade can be installed without any providers; install a compatible bundle
provider separately to execute a machine.

## Python quickstart

Install the GlassVM facade and a provider package for the machine you want to
run. Provider packages and their machine-specific artifact requirements are
documented in the
[`glassvm-machines` repository](https://github.com/lospooky/glassvm-machines).

```python
import json
from pathlib import Path

import glassvm

runtime = glassvm.Runtime.discover()
providers = json.loads(runtime.bundles())
print([provider["machine_id"] for provider in providers])

# Choose an installed provider and a valid artifact for that machine.
machine_id = "chip8"
artifact = Path("program.rom").read_bytes()

prepared = runtime.prepare(machine_id, artifact)
result = prepared.execute()
print(result["machine_execution"]["status"])
```

`Runtime.prepare` also accepts JSON-compatible `configuration`,
`input_schedule`, `execution_controls`, and `observation` values. The selected
machine provider validates those values against its contract. A prepared run is
an in-process handle; prepare it again when you need a new process or run.

To persist a run, prepare an execution and pass a destination path to
`execute`:

```python
recorded = runtime.prepare(machine_id, artifact).execute(
    file_recorder=glassvm.FileRecorder("runs/example"),
)
if recorded["published_run"] is not None:
    print(recorded["published_run"]["path"])
```

## Build and development

The Rust workspace requires a Rust toolchain that supports edition 2024.

```bash
cargo test --workspace --locked --no-fail-fast
```

Build the Python extension from this checkout with Python 3.12 or newer:

```bash
cd python/facade
python -m venv .venv
. .venv/bin/activate
python -m pip install maturin
maturin develop
```

The base facade has no providers by itself. Provider installation is a
separate step.

## Documentation

Start at the [documentation index](docs/README.md).
