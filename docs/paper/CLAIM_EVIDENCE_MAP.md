# GlassVM paper claim-to-evidence map

**Status:** current release and paper evidence boundary, 2026-10-26

This map separates claims supported by the clean in-tree implementation from
claims that require independent release artifacts or paper-scale measurements.
The paper outline is maintained with the research workspace; this file is the
standalone release evidence boundary.

## Primary claims

| Claim | What the release must show | Current evidence | Status |
| --- | --- | --- | --- |
| **C1 — Machine abstraction.** GlassVM separates machine semantics from experimental orchestration. | One unchanged caller prepares and runs materially different bundles; bundle-specific semantics stay behind the bundle contract. | CHIP-8 is the baseline/reference implementation; PICO-8 and TIC-80 are the independently designed external validation machines. All three published 0.3.1 distributions pass fresh package-index execution through the shared facade. The focused Garden and Relay runs exercise generic downstream consumers across the same three machines. Hexwell and Wyrd-16 remain outside this paper denominator. | The 0.3.1 release workflows passed and the focused clean-room consumer proof passed for facade-only, each provider, and all providers. Final paper-wide regeneration is still open. Provider self-reported `bundle_version` remains `0.3.0` inside the published `0.3.1` wheels; this metadata discrepancy is recorded in the checklist and must be corrected before evidence sealing. |
| **C2 — Observation inversion.** An experiment requests the evidence it needs through an explicit contract. | Preparation resolves exact channel, event-kind, native, frame, snapshot, final-state, and capability prerequisites before execution; emission and derived outputs remain separate. | `ObservationRequest`, `PreparedObservation`, dependency closure, optional downgrade, bounded normalizers, typed evidence channels, and `EvidenceReceipt` are covered by shared conformance and release CI. The 0.3.1 installed-wheel consumer proof exercises preparation and separate execution/evidence/recorder/publication outcomes. | Contract/conformance coverage and 0.3.1 package-index execution are established. The focused Garden/Relay runs request and consume selected evidence; they do not replace the full observation matrix. Full 0.3.1 paper regeneration remains open. |
| **C3 — Practical selectable observation.** Observation cost is selectable, measurable, and suitable for long-running experimentation. | Measurements distinguish execution, normalization, sink, encoding, durable storage, memory, and file-count costs across observation selections and workloads. | Existing Slice 5 and Slice 6 measurements retain their exact historical source/package coordinates, raw data, manifests, summaries, and plots under [`experiments/measurements/`](experiments/measurements/). Focused 0.3.1 Garden/Relay results are additional consumer demonstrations, not performance or long-run boundedness measurements. | The run-length RSS result is evidence for its recorded fixtures, configurations, revisions, and WSL2 host only. It has not been regenerated against the 0.3.1 release packages. Keep it labeled with its recorded source coordinates; do not present it as a 0.3.1 measurement or universal memory bound. Allocator high-water was unavailable, and persisted evidence grew with emitted records. See [Slice 6](PAPER_RELEASE_SLICE_06.md). |

The downstream consumer demonstrations support the claim that callers can
consume selected evidence without GlassVM trace reconstruction; they do not
support the performance or storage-cost parts of C3. Derived metrics belong to
bundle-owned normalizers, while Refinery remains a consumer that scores or
searches over their outputs.

Glass Relay adds a focused C1/C2 example of shared experimental evaluation:
one Python consumer compares the same press/hold/release requirements across
three machines through normalized input acceptance and frame fingerprints.
It evaluates experiment-level requirements and introduces no machine-specific
capability or translation semantics. The original reference/delayed/stuck
cartridges are controls for the tester; matching measurements concern this
task and configuration, not general machine equivalence or execution speed.
The eighteen-run 0.3.0-wheel development matrix is historical and documented in
[`glass-relay-0.3.0/SUMMARY.md`](experiments/measurements/glass-relay-0.3.0/SUMMARY.md),
with exact scope in [`glass_relay/README.md`](experiments/glass_relay/README.md).
The focused `0.3.1` package run is recorded in
[`glass-relay-0.3.1/SUMMARY.md`](experiments/measurements/glass-relay-0.3.1/SUMMARY.md).
Its installed distributions are 0.3.1, while the providers report an internal
`bundle_version` of 0.3.0. The run is useful focused evidence, but that
metadata discrepancy must be corrected before it is treated as sealed release
provenance. Full paper regeneration remains a separate gate.

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

These are architectural and conformance results. The package-index checks
establish availability for the recorded `0.3.1` artifacts, not future
availability. They do not by themselves establish cross-host performance,
population-scale throughput, or the still-pending full 0.3.1 paper run.

## Evidence required for the paper and release

| Evidence item | Supports | Required artifact or test |
| --- | --- | --- |
| Shared-runner heterogeneous execution | C1 | A clean-room example whose caller is unchanged across CHIP-8, PICO-8, and TIC-80. |
| Preparation and failure behavior | C2 | Required-prerequisite rejection, optional downgrade, dependency closure, typed input/configuration admission, and pre-`RunStarted` failure tests in every released bundle. |
| Selective observation matrix | C2, C3 | The retained five-repetition, nine-recipe matrix is prior-version evidence. It records requested/fulfilled channels and separate receipts, but has not been regenerated against 0.3.1. |
| Bounded online execution | C3 | The clean 120-trial, three-machine/four-mode/four-length matrix observed approximately flat peak RSS for every tested pair. Source revisions, limits, raw results, and claim caveats are recorded in [Slice 6](PAPER_RELEASE_SLICE_06.md). This is prior-source empirical evidence, not a 0.3.1 measurement or universal guarantee. |
| Recorder/storage cost | C3 | [`experiments/measurements/OBSERVATION_SUMMARY.md`](experiments/measurements/OBSERVATION_SUMMARY.md) reports per-channel logical/encoded bytes, block/segment/file counts, physical directory bytes, and publication outcomes. Its recorded source/package coordinates govern the claims; the matrix has not been regenerated for 0.3.1. |
| Downstream analysis | C2 | Glass Garden and Glass Relay are focused 0.3.1 downstream consumers of selected evidence. They do not establish observation performance or replace the full observation matrix. |
| Independent publication | C1, C2, C3 | Both repositories are public; the five machine-neutral GlassVM crates and the facade, three provider, and selector Python distributions are published at `0.3.1`. Fresh 0.3.1 installs passed the focused consumer proof; provider-reported bundle versions remain stale as described above. |
| Paper regeneration | All | The versioned runner and scripts are present. The complete 0.3.1 evidence regeneration and final source sealing have not been run. |

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

The existing run-length result may be described only with its recorded source
pair, workloads, four observation modes, WSL2 host, and repetition counts. It
is not regenerated on the 0.3.1 packages, so do not attribute it to that release
wave. Any final manuscript claim using it must preserve those coordinates and
the single-repetition 600,000-frame endpoint. Do not generalize it to a
universal memory bound, cross-host performance, population-scale throughput, or
a completed machine-independent Refinery benchmark.

## Baseline boundary

Optional historical local records under
`baselines/20260818-pc-5900x-wsl2-amd-ryzen-9-5900x/` are a reproducible
baseline for their recorded collection state, not necessarily the current
release. They identify the host, toolchain, source
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
coordinates and their current Rust/Python CI are green. The original Slice 6
run-length measurement, CHIP-8 retention cleanup, and clean same-coordinate
three-machine rerun are complete. Slice 9 manuscript-wide regeneration and
immutable artifact sealing remain open gates. PyPI availability and
package-index-installed execution were verified by the Slice 5 collection.
