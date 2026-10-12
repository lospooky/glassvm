# Architecture

GlassVM separates machine semantics from the shared interfaces used to prepare
and observe an execution. A machine bundle owns the emulator and any
machine-specific interpretation. GlassVM defines the contracts that let a
caller submit a run, request evidence, and consume the resulting records.

## Repository components

```text
caller
  ├── Rust crates ── glassvm_registry ── MachineBundle
  └── Python facade ── installed provider entry point ── machine bundle
                                               │
                         ┌─────────────────────┴─────────────────┐
                         │                                       │
                  glassvm_core                         bundle-owned emulator
             requests, events, evidence,                         │
                  normalizers                              emissions
                         │                                       │
            ┌────────────┴───────────┐                           │
            │                        │                           │
      glassvm_query           glassvm_recorder ◄─────────────────┘
      finite projections       bounded file runs
```

The Python facade discovers providers from the `glassvm.machine_bundles` entry
point group. The Rust `Registry` is a separate, explicit registration helper:
it validates a bundle before adding it and rejects duplicate machine IDs. The
Python facade does not use that Rust registry to discover providers.

## Crate responsibilities

- **`glassvm_core`** defines machine descriptors and contracts, execution
  requests, typed inputs, preparation, observation negotiation, emissions,
  evidence receipts, and the emulator and normalizer traits.
- **`glassvm_normalizer_contract`** defines versioned capability schemas,
  requests, dependency declarations, outputs, and receipts without depending
  on machine execution code.
- **`glassvm_recorder`** adapts emissions to Python consumers and/or bounded
  reference files. It accounts for persistence and publication separately
  from machine execution and evidence fulfillment.
- **`glassvm_query`** computes projections over a finite event vector supplied
  by the caller. It does not open recorder files or retain a durable database.
- **`glassvm_registry`** validates and stores explicitly registered Rust
  bundles in memory.
- **`python/facade`** exposes `Runtime`, `PreparedRun`, and `FileRecorder` to
  Python and dispatches calls to installed providers.

## Execution lifecycle

```text
ExecutionRequest
      │
      ├── validate artifact, configuration, input schedule, and limits
      ├── negotiate ObservationRequest against the bundle's normalizer catalog
      ▼
PreparedRun + PreparedObservation
      │
      ├── construct the machine session
      ├── execute while synchronously emitting requested evidence
      ├── normalize selected evidence into capability outputs
      ├── produce an EvidenceReceipt
      └── optionally finalize a recorder and publish the run
```

Preparation is the admission boundary. The bundle validates the artifact and
configuration, resolves scheduled inputs and execution limits, and resolves
observation prerequisites before execution begins. `PreparedRun` represents
validated execution inputs and identity. `PreparedObservation` represents the
negotiated evidence contract. They are distinct values because execution
identity and observation selection answer different questions.

During execution, emissions are delivered synchronously to caller-owned sinks.
Bundle normalizers observe borrowed emissions and keep only their own reducer
state. Callers can choose to forward the emissions to Python consumers, a file
recorder, both, or neither.

## Ownership boundaries

The bundle owns machine state, transitions, machine-specific artifact and
configuration rules, input application, native evidence, and capability
reduction. GlassVM owns the shared request and receipt types, event vocabulary,
channel delivery, and recorder format. Consumers own retention, downstream
analysis choices, and any policy applied to capability outputs.

Errors and results preserve these boundaries. Machine execution, evidence
fulfillment, Python sink delivery, recorder finalization, and publication are
reported separately. For example, incomplete evidence does not imply that the
machine failed, and a completed execution does not imply that a file was
published.
