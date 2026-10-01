# GlassVM publication contract

**Status:** release-facing contract summary

This document describes the clean publication boundary for GlassVM. It is the
short entry point for callers, bundle authors, and paper artifact reviewers.
The implementation status and verification history are maintained separately
from this release-facing summary.

## Scope

GlassVM is a machine-execution substrate. It supplies:

- a machine-neutral execution and input contract;
- preparation-time validation and observation negotiation;
- synchronous, bounded evidence emission;
- bundle-owned normalization and derived capability outputs; and
- a reference file recorder with explicit publication accounting.

The initial publication workspace contains three first-class bundles:

| Bundle | Machine role | Rust plugin |
| --- | --- | --- |
| CHIP-8 | baseline/reference bytecode virtual machine | [`glassvm-machines/chip8/plugin`](https://github.com/lospooky/glassvm-machines/tree/main/chip8/plugin) |
| PICO-8 | independently designed external validation machine | [`glassvm-machines/pico8/plugin`](https://github.com/lospooky/glassvm-machines/tree/main/pico8/plugin) |
| TIC-80 | independently designed external validation machine | [`glassvm-machines/tic80/plugin`](https://github.com/lospooky/glassvm-machines/tree/main/tic80/plugin) |

The common caller is not specialized for any of these machine semantics.

All three release bundles implement the same clean contracts and Python
provider protocol. CHIP-8 is the baseline/reference implementation. PICO-8
and TIC-80 are the independently designed external validation machines used
for the paper's cross-machine publication denominator. Hexwell and Wyrd-16 are
preserved on the [`parked/hexwell-wyrd16` branch](https://github.com/lospooky/glassvm-machines/tree/parked/hexwell-wyrd16)
and are not part of this release candidate.

## Contract layers

```text
MachineDescriptor
  -> machine identity, versions, and semantic schemas

MachineContract
  -> MachineArtifactSpec + InputCatalog

EmulatorBackend
  -> configuration schema + execution-limit catalog + prepared-session construction

ExecutionRequest
  -> artifact + configuration + input schedule + execution controls + observation

PreparedRun
  -> validated execution/input state

PreparedObservation
  -> negotiated evidence and capability requirements

EmulatorSession
  -> execution and synchronous emissions

Normalizer
  -> bundle-owned online reduction into capability outputs

FileRunSession
  -> caller-selected persistence, receipts, finalization, and atomic publication
```

These layers answer different questions. In particular, `PreparedRun` is not
an observation plan, receipt, recorder object, replay package, or durable
serialization format. `PreparedObservation` is prospective negotiation;
`EvidenceReceipt` and `RecorderReceipt` are retrospective accounting.

## Artifact, configuration, input, and controls

The execution request has four deliberately separate kinds of caller input:

```rust
ExecutionRequest {
    artifact,
    configuration,
    input_schedule,
    execution_controls,
    observation,
}
```

- `MachineArtifactSpec` declares and validates the persistent artifact used to
  instantiate execution. Its canonical names are `MachineArtifactSpec` and
  `ArtifactEncoding`.
- Machine configuration is an immutable, schema-qualified construction-time
  value declared by `EmulatorBackend::config_schema()`.
- `InputSchedule` contains time-varying typed inputs. Frame, step, cycle, and
  tick are distinct coordinates; an ordinal orders schedule entries and is
  not a cross-channel event sequence.
- Execution controls are caller-imposed limits and stopping controls. They
  are declared by the bundle's `ExecutionLimitCatalog`, have no implicit
  defaults, and participate in prepared execution identity.

Input identifiers and schema-family identifiers are canonical segmented IDs.
Input family membership is explicit catalog metadata; it is never inferred
from a concrete schema identifier. Full input values are emitted only through
explicitly negotiated `InputValueEvidence` selectors.

## Preparation

Preparation is the one admission boundary. A bundle resolves and validates:

1. artifact constraints;
2. configuration and its defaults;
3. input catalog membership and schedule order;
4. execution limits and their submitted combination;
5. the filtered live-input interface; and
6. the complete observation prerequisite closure.

Required observation capabilities and prerequisites fail preparation before
`RunStarted`. Optional capabilities with unsatisfied prerequisites remain
explicitly unavailable, with their reason preserved. Normalizers do not
discover missing inputs at runtime.

The resolved objects are immutable for the session:

```text
request
  -> PreparedRun
  -> PreparedObservation
  -> session construction
  -> execute
```

The prepared execution identity includes artifact, machine/configuration,
resolved scheduled inputs, execution controls, and the canonical prepared
live-input interface. It excludes observation and recorder implementation
details. Live values and per-run live-input instance IDs are runtime evidence,
not preparation identity.

## Evidence channels

Observation is caller-selected and channel-separated. A request may select
any supported combination:

| Channel | Meaning |
| --- | --- |
| Normalized events | Machine-independent event facts declared by the bundle. |
| Native evidence | Typed, bundle-owned machine evidence with declared schemas and kinds. |
| Frames | `FrameArtifact` values at frame boundaries; independent of `FrameCompleted` event selection. |
| Snapshots | Explicit resumable machine-state artifacts. |
| Input values | Explicit `InputValueEvidence` payloads for selected inputs or schema families. |
| Capability outputs | Derived normalizer results with canonical capability IDs. |

`FrameCompleted` is a temporal/control fact. `FrameArtifact` is requested raw
frame evidence. A capability output is a derived interpretation. A receipt
accounts for whether requested evidence was fulfilled. None of these concepts
is a substitute for another.

Emissions are borrowed, synchronous values delivered to a caller-owned sink.
The blessed path emits at the semantic boundary and does not retain a
whole-run event, frame, or evidence collection. Segment and block limits are
hard buffering bounds. A record larger than the block hard limit is rejected;
valid records satisfy:

```text
record size <= block hard limit <= segment hard limit
```

Evidence failure is separate from machine execution failure. A recorder may
successfully publish a run whose evidence receipt records incomplete or failed
channels; recorder failure remains a separate outcome.

## Bundle-owned normalization

`NormalizerCatalog` is the canonical declaration of normalized schemas,
native/event/frame/snapshot/final-state prerequisites, capability metadata,
and capability dependencies. Each bundle creates a per-run normalizer from
the prepared request.

Normalizers use the lifecycle:

```text
begin -> observe(emission)* -> finalize(optional final state)
```

Finalization receives only the explicitly negotiated final machine state when
the capability requires it. It never receives a hidden completed-evidence bag.
Derived metrics—including visual, memory, register, control-flow, and bundle
state/reaction motifs—belong to the bundle normalizer. Refinery or another
consumer may score, rank, archive, or search over those outputs, but does not
define their machine-specific extraction semantics.

## Persistence and publication

`FileRunSession` is a small orchestration helper. It owns the shared file-backed
lifecycle, not the evidence model:

```text
FileEmissionSink
  -> BudgetedSink
  -> session.execute()
  -> emit EvidenceReceipt
  -> finalize recorder
  -> atomically publish
```

The result keeps four domains separately inspectable:

```rust
FileRunResult {
    execution,
    evidence,
    recorder,
    published,
}
```

The published handle exists only when the finalized `RecorderReceipt` passes
the publication gate and staging is atomically renamed into place. Interrupted
staging runs are inspectable and cleanable but are not resumed in the initial
format. Recorder storage is not a replay package and is not read implicitly by
core execution or query code.

## Python distribution

The generic `glassvm_py` wheel contains the runtime facade only. Each bundle
wheel supplies one provider through the installed
`glassvm.machine_bundles` entry-point group. The provider protocol is:

```text
protocol: glassvm.python_bundle
protocol_version: { major, minor, patch }
prepare_function: prepare_run
execute_function: execute_prepared
```

The protocol identifier is unqualified and the numeric version is metadata;
protocol names do not encode chronology. Discovery sees installed entry
points only. Extras select ordinary prebuilt bundle-wheel dependencies; they
do not compile bundles at install time or activate a static registry.

The Python lifecycle is the same for every bundle:

```python
runtime = glassvm_py.Runtime.discover()
prepared = runtime.prepare(
    machine_id=machine_id,
    artifact=artifact,
    configuration=configuration,
    input_schedule=input_schedule,
    execution_controls=execution_controls,
    observation=observation,
)
result = prepared.execute(output_path)
```

Python dict/list ergonomics end at preparation. Providers immediately convert
them into typed Rust contract values and generate canonical identity material
only after validation and resolution. The prepared Python object is opaque,
provider-owned, and in-process only.

## Explicit non-goals

The publication contract does not contain:

- mutation or artifact-generation semantics;
- fitness extensions or Refinery scoring policy;
- bodies, environments, episodes, or actuation DSLs;
- a public native-event adapter object;
- whole-run materialization as the execution API;
- replay packages, manifests, or storage readers in core execution;
- checksums, signatures, or cryptographic audit machinery; or
- a static built-in bundle registry in the generic Python wheel.

These exclusions are part of the abstraction, not missing convenience APIs.

## Release evidence boundary

The standalone `glassvm` and `glassvm-machines` repositories provide clean
workspace baselines across the three release bundles and exercise local
entry-point discovery and recorder publication. Package-index publication,
paper-scale boundedness measurements, and regenerated paper figures remain
release gates. The claim map records those gates without promoting local
implementation evidence into publication claims.
