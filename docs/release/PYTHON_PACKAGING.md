# GlassVM Python packaging

The Python distribution has one generic runtime and one provider wheel per
publication bundle. The runtime discovers providers through installed entry
points; it does not contain a machine registry or machine-specific dispatch.

## Distribution packages

The publication names are:

- `glassvm_py`: generic runtime facade and provider discovery;
- `glassvm-machine-chip8`: CHIP-8 provider, imported as `glassvm_py_chip8`;
- `glassvm-machine-hexwell`: Hexwell provider, imported as
  `glassvm_py_hexwell`; and
- `glassvm-machine-wyrd16`: Wyrd-16 provider, imported as
  `glassvm_py_wyrd16`.

Each bundle wheel supplies one provider and statically links its machine
implementation. A compatible wheel installation does not require a Rust
compiler.

## Extras

Extras are ordinary dependency selection:

```toml
[project.optional-dependencies]
chip8 = ["glassvm-machine-chip8==0.1.0"]
hexwell = ["glassvm-machine-hexwell==0.1.0"]
wyrd16 = ["glassvm-machine-wyrd16==0.1.0"]
all = [
    "glassvm-machine-chip8==0.1.0",
    "glassvm-machine-hexwell==0.1.0",
    "glassvm-machine-wyrd16==0.1.0",
]
```

An extra selects prebuilt bundle wheels. It does not compile a bundle at
install time, load a runtime plugin, or activate a fallback registry.

## Provider protocol

Every bundle registers exactly one entry point in the
`glassvm.machine_bundles` group. The entry point returns strict metadata:

```json
{
  "protocol": "glassvm.python_bundle",
  "protocol_version": {"major": 1, "minor": 0, "patch": 0},
  "machine_id": "chip8",
  "distribution": "glassvm-machine-chip8",
  "module": "glassvm_py_chip8",
  "prepare_function": "prepare_run",
  "execute_function": "execute_prepared",
  "bundle_version": "0.1.0"
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
  file-backed lifecycle.

Preparation validates the artifact, configuration, input schedule, execution
controls, and observation request exactly once. Required observation failures
occur before execution starts. Optional unavailable capabilities remain
explicit in the negotiated result.

## Python lifecycle

The caller shape is identical for all installed bundles:

```python
import glassvm_py

runtime = glassvm_py.Runtime.discover()
prepared = runtime.prepare(
    machine_id="chip8",
    artifact=artifact,
    configuration=configuration,
    input_schedule=input_schedule,
    execution_controls=execution_controls,
    observation=observation,
)
result = prepared.execute(output_path)
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
execution
evidence_receipt
recorder_receipt
published_run
```

The published reference exists only after recorder finalization succeeds and
the atomic publication gate completes. Machine execution, evidence
fulfillment, recorder finalization, and publication remain separately
inspectable.

## Installation verification

The clean-room matrix is:

| Environment | Providers visible | Required result |
| --- | --- | --- |
| base only | none | Runtime imports; an unavailable machine fails clearly. |
| `chip8` | CHIP-8 | Typed preparation and a published run. |
| `hexwell` | Hexwell | Typed preparation and a published run. |
| `wyrd16` | Wyrd-16 | Typed preparation and a published run. |
| `all` | all three | The same caller lifecycle works for every provider. |

Each environment also checks invalid artifact, configuration, input, limit,
and required observation requests fail at preparation. Discovery is derived
solely from installed entry points.

## Local wheel check

After building the four wheels into one directory, a local equivalent of an
index installation is:

```bash
uv pip install --find-links dist "glassvm_py[chip8]"
uv pip install --find-links dist "glassvm_py[all]"
```

The local `dist` directory only supplies wheel files for the check. Published
users resolve the same ordinary distribution dependencies from the package
index.
