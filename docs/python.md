# Python facade

The `glassvm` Python package is a small facade over separately installed
machine providers. It contains no emulator implementation and does not
hard-code machine IDs.

## Install and discover providers

Install the generic `glassvm` distribution and a provider for each machine you
intend to run. Provider packages are built and released from the companion
[`glassvm-machines`](https://github.com/lospooky/glassvm-machines) repository.

```python
import json

import glassvm

runtime = glassvm.Runtime.discover()
providers = json.loads(runtime.bundles())
for provider in providers:
    print(provider["machine_id"], provider["bundle_version"])
```

Discovery reads Python entry points from the `glassvm.machine_bundles` group.
Each provider supplies metadata and two operations: `prepare_run` and
`execute_prepared`. The facade checks the provider protocol and metadata,
rejects duplicate machine IDs, and sorts the discovered providers by ID.
Provider code owns machine-specific preparation and execution.

An empty provider list is valid. Calling `prepare` for a machine without an
installed provider raises `LookupError`.

## Prepare and execute

```python
from pathlib import Path

prepared = runtime.prepare(
    machine_id="chip8",
    artifact=Path("program.rom").read_bytes(),
    configuration=None,
    input_schedule=None,
    execution_controls=None,
    observation=None,
)

result = prepared.execute()
print(result["machine_execution"]["status"])
print(result["evidence_fulfillment"]["status"])
```

The machine ID and artifact must match an installed provider. Optional request
arguments must be JSON-compatible; the provider validates their content.
Omitting an optional value passes JSON `null` to that provider. Its contract
defines whether that value selects a default or is rejected. Preparation
rejects invalid requests before execution starts.

The `PreparedRun` object is an opaque, in-process handle. It is not a durable
run record, snapshot, or serialization format. Prepare a new handle for a new
process or a new execution request.

## Python emission consumers

Pass a list or tuple of consumers to `execute` to receive emissions
synchronously:

```python
class Consumer:
    def emit_batch(self, emissions):
        for emission in emissions:
            process_emission(emission)

    def record_capability_outputs(self, outputs):
        process_outputs(outputs)

    def record_capability_receipts(self, receipts):
        process_receipts(receipts)


result = prepared.execute(sinks=[Consumer()])
```

Each consumer must provide all three callable methods. Emission batches
contain owned Python values; no Rust borrow crosses into Python. Delivery is
synchronous and applies backpressure during callbacks. A callback exception
is reported in `python_sink_delivery`; it is not merged into machine-execution
or recorder status.

## File recording and result fields

The facade accepts a `FileRecorder` request value and leaves the file format
and run lifecycle to the Rust recorder:

```python
result = prepared.execute(
    sinks=[Consumer()],
    file_recorder=glassvm.FileRecorder("runs/run-001"),
)
```

The returned mapping has separate `machine_execution`,
`evidence_fulfillment`, `python_sink_delivery`, `recorder_finalization`, and
`publication` fields, plus `published_run`. The published-run reference is
present only after successful recorder finalization and publication. See
[Recording](recording.md) for the file lifecycle.

## Build the facade from source

From the repository root:

```bash
cd python/facade
python -m venv .venv
. .venv/bin/activate
python -m pip install maturin
maturin develop
```

The facade requires Python 3.12 or newer. This builds the generic runtime;
install machine providers separately to execute machine artifacts.
