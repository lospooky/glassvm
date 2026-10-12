# Implementing a machine bundle

Machine bundles live in the companion
[`glassvm-machines`](https://github.com/lospooky/glassvm-machines) repository.
A bundle owns one machine's artifact rules, configuration, emulator state,
input semantics, and machine-specific evidence. It implements GlassVM's shared
contracts so callers can use the same preparation and execution lifecycle
across different machines.

## Bundle pieces

The central interface is `glassvm_core::MachineBundle`. A bundle provides:

- a `MachineDescriptor` with canonical identity, versions, variants, and
  semantic schemas;
- a `MachineContract` declaring artifact constraints and an `InputCatalog`;
- an `EmulatorBackend` for configuration schemas, execution limits, and
  prepared session construction;
- a normalizer catalog and per-run `Normalizer` implementation; and
- optional static-analysis, verification, and live-input services.

The static analyzer and verifier are optional. Live input is optional too, but
the bundle contract must not advertise live-delivered inputs unless the bundle
also supplies the corresponding live-input capability.

## Preparation

Preparation is the one request admission boundary. The default
`MachineBundle::prepare_run` validates the request envelope and machine ID,
admits artifact bytes against `MachineArtifactSpec`, validates configuration
and its defaults, resolves the typed input schedule, and validates
caller-imposed execution controls against the bundle's declared limit catalog.
It produces a `PreparedRun` with the resolved execution state and identity.

The caller then negotiates an `ObservationRequest` against the bundle's
`NormalizerCatalog`. `PreparedObservation` contains the requested capability
dependency closure, availability, and other selected prerequisites. Required
unavailable capabilities reject preparation; optional ones remain explicitly
unavailable. Construct the emulator session from the validated run and
observation values rather than bypassing preparation.

## Execution and normalization

`EmulatorSession` implements machine execution, frame stepping, reset, and
snapshot/restore behavior for the bundle's supported continuation boundary.
During `execute`, it emits requested evidence synchronously through an
`EmissionSink`.

A `Normalizer` receives borrowed emissions through `begin`, `observe`, and
`finalize`. It may keep reducer state needed for its requested capabilities,
but the interface does not provide a completed event or evidence collection.
Finalization may receive one negotiated final machine snapshot. Capability
outputs and their fulfillment receipts are reported separately.

## Contract validation

Implement `MachineBundle::validate_contract` successfully before registering
the bundle. It checks consistency between the descriptor and emulator,
validates the machine and normalizer catalogs, and checks the optional service
relationships. `glassvm_registry::Registry::register` performs this validation
and rejects a second bundle with the same machine ID.

Bundle conformance should exercise artifact and configuration rejection,
input schedule resolution, observation prerequisites, execution and emissions,
normalizer outputs, snapshots, reset, and deterministic continuation where
the bundle advertises those behaviors. Machine-specific truth and artifact
requirements should be documented alongside the bundle implementation.

## Keep machine semantics in the bundle

GlassVM carries typed IDs, schemas, coordinates, requests, and evidence. It
does not prescribe a universal machine configuration format, input-control
language, native-event ontology, or capability reducer. Keep those semantics
machine-owned and expose only the versioned metadata needed for callers to
select and interpret them.
