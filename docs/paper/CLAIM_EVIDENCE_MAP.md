# GlassVM paper claim-to-evidence map

**Status:** current release and paper evidence boundary, 2026-10-04

This map separates claims supported by the clean in-tree implementation from
claims that require independent release artifacts or paper-scale measurements.
The paper outline is maintained with the research workspace; this file is the
standalone release evidence boundary.

## Primary claims

| Claim | What the release must show | Current evidence | Status |
| --- | --- | --- | --- |
| **C1 — Machine abstraction.** GlassVM separates machine semantics from experimental orchestration. | One unchanged caller prepares and runs materially different bundles; bundle-specific semantics stay behind the bundle contract. | CHIP-8 is the baseline/reference implementation; PICO-8 and TIC-80 provide independently designed external validation specimens. Shared conformance and installed-wheel tests exercise the same prepared-run and facade paths across the three release bundles. Hexwell and Wyrd-16 are preserved on a separate parked branch and are outside this release denominator. | Architecture and three-machine behavior are verified at GlassVM `cf7dee8444e1f5968f7206f65335663855c981e2` and Machines `56563bd0dfb92a67d272ed4d8ab4868e448b265d`; current CPython 3.12/3.13/3.14 CI is green. Python package-index publication and install proof remain open. |
| **C2 — Observation inversion.** An experiment requests the evidence it needs through an explicit contract. | Preparation resolves exact channel, event-kind, native, frame, snapshot, final-state, and capability prerequisites before execution; emission and derived outputs remain separate. | `ObservationRequest`, `PreparedObservation`, exact dependency closure, optional downgrade, bounded normalizers, typed frame/native/input-value channels, and `EvidenceReceipt` tests are exercised by shared conformance cases for every release bundle. The installed-wheel clean-room path checks preparation and separate result domains. | Preparation and evidence semantics are verified across all three bundles; the current 3.12/3.13/3.14 source-distribution-to-wheel and provider matrix is green. Package-index publication and install proof remain open. |
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
  `glassvm.machine_bundles`; all three release providers use the same
  preparation/execution shape in the published clean-room CI matrix.
- Each provider source distribution can be rebuilt into a wheel from the
  archive itself. The current matrix performs this rebuild before exercising
  base-only, per-provider, and all-provider installations.

These are architectural and conformance results. They do not by themselves
establish cross-host performance, population-scale throughput, or package
index availability.

## Evidence required for the paper and release

| Evidence item | Supports | Required artifact or test |
| --- | --- | --- |
| Shared-runner heterogeneous execution | C1 | A clean-room example whose caller is unchanged across CHIP-8, PICO-8, and TIC-80. |
| Preparation and failure behavior | C2 | Required-prerequisite rejection, optional downgrade, dependency closure, typed input/configuration admission, and pre-`RunStarted` failure tests in every released bundle. |
| Selective observation matrix | C2, C3 | No observation, summary, selected normalized events, complete normalized events, selected native evidence, frame capture, and capability-output runs with explicit receipts. |
| Bounded online execution | C3 | Long-run RSS/allocation measurements showing memory bounded by configured sink/reducer state rather than run length. |
| Recorder/storage cost | C3 | Logical bytes, encoded bytes, block/segment counts, file counts, and publication outcomes under identical workloads and observation requests. |
| Downstream analysis | C2, C3 | Refinery or equivalent caller consuming selected normalized projections and capability outputs without requiring whole-run GlassVM materialization. |
| Independent publication | C1, C2, C3 | Both repositories are public and the five machine-neutral GlassVM crates are published at `0.1.0`. Bundle source archives and wheels build from the split repositories without GlassVM path overrides. The four Python distributions are not yet published to PyPI, so package-index-only installation remains open. |
| Paper regeneration | All | Versioned scripts regenerate every table and figure from public result files. |

## Permitted claim language before those gates close

The release and paper drafts may state:

> GlassVM provides a machine-neutral execution substrate with preparation-time
> observation negotiation, bundle-owned online normalization, independently
> selectable evidence channels, and a bounded reference recorder. The current
> release workspace demonstrates these contracts across CHIP-8, PICO-8, and
> TIC-80.

They may describe CHIP-8 as the baseline/reference bundle and PICO-8 and TIC-80
as the independent validation machines. Hexwell and Wyrd-16 are parked outside
the current release branch and should not be counted as publication specimens.
The checked-in baseline remains a host-specific regression artifact.

They must not yet state that the machine Python distributions are published to
PyPI, that the system has paper-scale boundedness or population-scale
throughput, or that a machine-independent Refinery benchmark is complete.
Those statements require the evidence items above.

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

CB-01 through CB-12 are complete in the implementation ledger. Slice 2's
source audit and cross-bundle conformance matrix are recorded in
[`PAPER_RELEASE_SLICE_02.md`](PAPER_RELEASE_SLICE_02.md). Both public source
coordinates and their current Rust/Python CI are green. PyPI publication,
empty-cache release verification, paper-scale measurement, and paper artifact
regeneration remain open gates.
