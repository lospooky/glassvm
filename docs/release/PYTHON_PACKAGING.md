# GlassVM Python packaging

The Python distribution has one generic runtime and one provider wheel per
publication bundle. The runtime discovers providers through installed entry
points; it does not contain a machine registry or machine-specific dispatch.

## Distribution packages

The publication names are:

- `glassvm`: generic runtime facade and provider discovery, imported as `glassvm`;
- `glassvm-chip8`: CHIP-8 provider wheel;
- `glassvm-pico8`: PICO-8 provider wheel; and
- `glassvm-tic80`: TIC-80 provider wheel; and
- `glassvm-machines`: metadata-only package for tested bundle selections.

The bundle modules named in provider metadata are implementation modules used
by entry-point discovery, not supported direct-import APIs. Callers use the
`glassvm` facade and select a machine by its canonical machine ID.

Each bundle wheel supplies one provider and statically links its machine
implementation. A compatible wheel installation does not require a Rust
compiler.

## Bundle selection

The facade remains machine-neutral. Bundle selection extras belong to the
downstream `glassvm-machines` distribution:

```toml
[project.optional-dependencies]
chip8 = ["glassvm-chip8==0.2.0"]
pico8 = ["glassvm-pico8==0.2.0"]
tic80 = ["glassvm-tic80==0.2.0"]
all = ["glassvm-chip8==0.2.0", "glassvm-pico8==0.2.0", "glassvm-tic80==0.2.0"]
```

The selection package depends on a tested `glassvm` facade version. Its extras
select prebuilt provider wheels through normal package dependencies. They do
not compile a bundle at install time or activate a fallback registry.

## Provider protocol

Every bundle registers exactly one entry point in the
`glassvm.machine_bundles` group. The entry point returns strict metadata:

```json
{
  "protocol": "glassvm.python_bundle",
  "protocol_version": {"major": 2, "minor": 0, "patch": 0},
  "machine_id": "chip8",
  "distribution": "glassvm-chip8",
  "module": "glassvm_chip8",
  "prepare_function": "prepare_run",
  "execute_function": "execute_prepared",
  "bundle_version": "0.2.0"
}
```

The protocol identifier is stable and unqualified. Its numeric version is
metadata, not a suffix in the protocol name. The runtime requires the exact
protocol identifier and supported numeric version, rejects unknown metadata,
and rejects duplicate machine IDs.

The provider operations have one responsibility each:

- `prepare_run` admits and resolves the complete execution request, producing
  opaque provider-owned prepared state; and
- `execute_prepared` consumes that state and delegates execution to the shared
  file-backed lifecycle, including bounded Python sink delivery and separate
  execution, evidence, delivery, recorder, and publication outcomes.

Preparation validates the artifact, configuration, input schedule, execution
controls, and observation request exactly once. Required observation failures
occur before execution starts. Optional unavailable capabilities remain
explicit in the negotiated result.

## Python lifecycle

The caller shape is identical for all installed bundles:

```python
import glassvm

runtime = glassvm.Runtime.discover()
prepared = runtime.prepare(
    machine_id="chip8",
    artifact=artifact,
    configuration=configuration,
    input_schedule=input_schedule,
    execution_controls=execution_controls,
    observation=observation,
)
result = prepared.execute(
    sinks=[consumer],
    file_recorder=glassvm.FileRecorder(output_path),
)
```

Python accepts ordinary bytes, dictionaries, and lists for caller ergonomics.
The provider converts them at the preparation boundary into typed Rust
contract values, validates them, resolves defaults and ordering, and only then
produces canonical CBOR identity material. Python insertion order and caller
syntax are never execution identity.

The prepared object is an opaque, provider-owned, in-process handle. It is not
serializable, durable, or replay material.

The result is structured rather than a path or success flag:

```text
machine_execution
evidence_fulfillment
python_sink_delivery
recorder_finalization
publication
published_run
```

The published reference is null unless recorder finalization and the existing
atomic publication gate succeed. Machine execution, evidence fulfillment,
Python sink delivery, recorder finalization, and publication remain separately
inspectable, including partial outcomes.

## Installation verification

The clean-room matrix is:

| Environment | Providers visible | Required result |
| --- | --- | --- |
| `glassvm` | none | Runtime imports; an unavailable machine fails clearly. |
| `glassvm` + `glassvm-chip8` | CHIP-8 | Typed preparation and a published run. |
| `glassvm-machines[chip8]` | CHIP-8 | Tested facade/provider combination. |
| `glassvm-machines[all]` | CHIP-8, PICO-8, TIC-80 | The same caller lifecycle works for every provider. |

Each environment also checks invalid artifact, configuration, input, limit,
and required observation requests fail at preparation. Discovery is derived
solely from installed entry points.

## Local wheel check

After building the facade, three provider wheels, and selector package into one
directory, a local equivalent of an index installation is:

```bash
uv pip install --find-links dist "glassvm-machines[chip8]"
uv pip install --find-links dist "glassvm-machines[all]"
```

The local `dist` directory only supplies wheel files for the check. Published
users resolve the same ordinary distribution dependencies from the package
index.
