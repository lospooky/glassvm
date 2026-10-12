# Inputs, observations, and evidence

GlassVM makes inputs and requested observations explicit parts of an
execution request. A bundle validates and resolves them before constructing a
machine session.

## Inputs

`InputCatalog` declares the inputs a bundle accepts, their payload schemas,
delivery modes, and application coordinates. `InputSchedule` carries
time-varying values for one run. Inputs may be scheduled or, when the bundle
provides a live-input capability, delivered synchronously while the run is
active.

Frame, step, cycle, and tick are distinct coordinate types. A bundle describes
which boundaries it supports and when an input can be applied. The schedule's
ordinal orders entries in the submitted schedule; it is not a global event
sequence number.

The bundle maps accepted typed inputs to its own native input representation.
GlassVM transports the schema-qualified values but does not define a universal
control language for machines.

## Observation requests

`ObservationRequest` selects the evidence and capabilities for a run. It
includes:

- the requested evidence guarantee (`summary`, `complete_normalized`, or
  `oracle_ready`);
- normalized event selection and the requested state-access detail;
- requested bundle capability outputs;
- native evidence kinds;
- frame and snapshot capture modes;
- optional input-value evidence; and
- per-channel and total emission budgets with an overflow policy.

The bundle resolves the request against its normalizer catalog before
execution. A required capability with an unavailable prerequisite fails
preparation. An optional capability may be reported unavailable with its
reason. Dependencies are resolved as a closure, so the caller can inspect what
the selected capabilities require.

For Python callers, `configuration`, `input_schedule`, `execution_controls`,
and `observation` are JSON-compatible values passed to the selected provider.
The provider validates these values against its machine contract and
GlassVM's versioned schemas. Refer to that machine's bundle documentation for
its configuration fields, input IDs, and capability IDs.

## Evidence channels

The emission interface distinguishes these record types:

| Evidence | Meaning |
| --- | --- |
| Normalized events | Machine-independent execution facts selected by the caller. |
| Native evidence | Typed, machine-specific evidence with a bundle-owned schema. |
| Input-value evidence | Values of explicitly selected scheduled or live inputs. |
| Frames | Raw frame artifacts requested at supported frame boundaries. |
| Snapshots | Machine state artifacts requested for supported snapshot points. |
| Capability outputs | Derived values produced by the bundle's normalizer. |

These are independent channels. A frame artifact is not a normalized event;
a capability output is not the evidence used to derive it; and a snapshot is
not a replay package.

## Receipts and budgets

`PreparedObservation` records the negotiated request before execution.
`EvidenceReceipt` records what was fulfilled afterward, including channel and
capability statuses, item counts, logical byte counts, and any budget or sink
failure. These receipts should be used to distinguish evidence that was
complete, omitted, truncated, or failed.

Emission budgets constrain selected evidence while it is produced. If the
configured overflow policy is `fail`, exceeding a budget reports evidence
failure and can interrupt execution through the sink path. With
`allow_incomplete`, the run can continue with an incomplete evidence receipt.
The machine outcome and evidence outcome remain separately observable.
