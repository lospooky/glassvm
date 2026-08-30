# GlassVM paper claim-to-evidence map

**Status:** current release and paper evidence boundary, 2026-08-30

This map separates claims supported by the clean in-tree implementation from
claims that require independent release artifacts or paper-scale measurements.
The paper outline is maintained with the research workspace; this file is the
standalone release evidence boundary.

## Primary claims

| Claim | What the release must show | Current evidence | Status |
| --- | --- | --- | --- |
| **C1 — Machine abstraction.** GlassVM separates machine semantics from experimental orchestration. | One unchanged caller prepares and runs materially different bundles; bundle-specific semantics stay behind the bundle contract. | CHIP-8, Hexwell, and Wyrd-16 implement the clean core contract. Registry conformance exercises one machine-independent prepared-run path across all three. | In-tree demonstrated; package-release proof pending. |
| **C2 — Observation inversion.** An experiment requests the evidence it needs through an explicit contract. | Preparation resolves exact channel, event-kind, native, frame, snapshot, final-state, and capability prerequisites before execution; emission and derived outputs remain separate. | `ObservationRequest`, `PreparedObservation`, dependency closure, optional downgrade, bounded normalizers, typed frame/native/input-value channels, and `EvidenceReceipt` tests are present in `glassvm_core`. | In-tree substantial; full public artifact and Python evidence matrix pending. |
| **C3 — Practical selectable observation.** Observation cost is selectable, measurable, and suitable for long-running experimentation. | Measurements distinguish execution, normalization, sink, encoding, durable storage, memory, and file-count costs across observation selections and workloads. | Synchronous borrowed emissions, hard block/segment/buffer limits, independent channels, finalized `RecorderReceipt`, atomic publication, and recorder round-trip tests provide engineering support. | Mechanism demonstrated; paper-scale measurements pending. |

The machine-independent downstream analysis demonstration supports C2 and C3;
it is not a separate GlassVM claim. Derived metrics belong to bundle-owned
normalizers, while Refinery remains a consumer that scores or searches over
their outputs.

## What the clean implementation establishes

The current publication workspace establishes these properties:

- `MachineContract` contains only `MachineArtifactSpec` and `InputCatalog`.
- `EmulatorBackend` declares configuration and caller-imposed execution-limit
  schemas and constructs sessions only from `PreparedRun` plus
  `PreparedObservation`.
- Preparation validates artifact, configuration, schedule, input interface,
  limits, and observation prerequisites before `RunStarted`.
- `PreparedRun` contains validated execution/input state and execution
  identity. `PreparedObservation` contains negotiated observation state.
- Normalizers consume borrowed emissions incrementally and may receive only
  the explicitly negotiated final state during finalization.
- Normalized events, native evidence, frames, snapshots, input-value evidence,
  capability outputs, `EvidenceReceipt`, and `RecorderReceipt` remain distinct
  channels or objects.
- The reference recorder enforces hard buffering limits, writes typed channel
  records, requires a finalized receipt, and publishes through atomic rename.
- Query code accepts caller-supplied finite event projections; it does not read
  recorder storage or reconstruct an aggregate run.
- The generic Python runtime discovers installed providers through
  `glassvm.machine_bundles`; the three bundle providers use the same
  preparation/execution shape in local smoke coverage.

These are architectural and conformance results. They do not by themselves
establish cross-host performance, population-scale throughput, or package
index availability.

## Evidence required for the paper and release

| Evidence item | Supports | Required artifact or test |
| --- | --- | --- |
| Shared-runner heterogeneous execution | C1 | A clean-room example whose caller is unchanged across CHIP-8, Hexwell, and Wyrd-16. |
| Preparation and failure behavior | C2 | Required-prerequisite rejection, optional downgrade, dependency closure, typed input/configuration admission, and pre-`RunStarted` failure tests in every released bundle. |
| Selective observation matrix | C2, C3 | No observation, summary, selected normalized events, complete normalized events, selected native evidence, frame capture, and capability-output runs with explicit receipts. |
| Bounded online execution | C3 | Long-run RSS/allocation measurements showing memory bounded by configured sink/reducer state rather than run length. |
| Recorder/storage cost | C3 | Logical bytes, encoded bytes, block/segment counts, file counts, and publication outcomes under identical workloads and observation requests. |
| Downstream analysis | C2, C3 | Refinery or equivalent caller consuming selected normalized projections and capability outputs without requiring whole-run GlassVM materialization. |
| Independent publication | C1, C2, C3 | Extracted `glassvm` and `glassvm-machines` repositories, released crates/wheels, and clean environments with no monorepo paths. |
| Paper regeneration | All | Versioned scripts regenerate every table and figure from public result files. |

## Permitted claim language before those gates close

The release and paper drafts may state:

> GlassVM provides a machine-neutral execution substrate with preparation-time
> observation negotiation, bundle-owned online normalization, independently
> selectable evidence channels, and a bounded reference recorder. The current
> workspace demonstrates these contracts across CHIP-8, Hexwell, and Wyrd-16.

They may describe the three bundles as first-class in-tree reference bundles
and report the checked-in baseline as a host-specific regression artifact.

They must not yet state that GlassVM has independently published repositories,
package-index releases, paper-scale boundedness, population-scale throughput,
or a completed machine-independent Refinery benchmark. Those statements
require the evidence items above.

## Baseline boundary

Optional local records under
`baselines/20260818-pc-5900x-wsl2-amd-ryzen-9-5900x/` are a reproducible
current-state regression baseline. They identify the host, toolchain, source
state, fixtures, configuration, observation recipe, result digest, runtime,
RSS, and emitted-volume measurements for the recorded smoke matrix. Generated
baseline data is intentionally not part of the clean source repository.

They are not cross-host performance evidence, a durable-storage acceptance
report, a proof of run-length-independent memory, or a population-scale
throughput result. Historical Refinery storage concerns remain outside the
GlassVM publication evidence boundary.

## Implementation anchors

| Contract area | In-tree anchor |
| --- | --- |
| Bundle, artifact, and emulator boundary | [`crates/core/src/bundle.rs`](../../crates/core/src/bundle.rs) |
| Preparation and execution identity | [`crates/core/src/preparation.rs`](../../crates/core/src/preparation.rs) |
| Observation negotiation | [`crates/core/src/observation_contract.rs`](../../crates/core/src/observation_contract.rs) |
| Emission and budgets | [`crates/core/src/emission.rs`](../../crates/core/src/emission.rs) |
| Online normalization | [`crates/core/src/normalizer.rs`](../../crates/core/src/normalizer.rs) |
| Reference persistence | [`crates/recorder/src/lib.rs`](../../crates/recorder/src/lib.rs) |
| Projection queries | [`crates/query/src/lib.rs`](../../crates/query/src/lib.rs) |
| Python provider discovery | [`python/facade/src/lib.rs`](../../python/facade/src/lib.rs) |
| Cross-bundle conformance | Maintained by the companion `glassvm-machines` repository. |

## Current milestone position

CB-01 through CB-12 are complete in the implementation ledger. The standalone
repository baselines are now established; package publication, paper-scale
measurement, and paper artifact regeneration remain the next release gates.
