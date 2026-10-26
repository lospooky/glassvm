# GlassVM paper claim-to-evidence map

**Status:** current release and paper evidence boundary, 2026-10-12

This map separates claims supported by the clean in-tree implementation from
claims that require independent release artifacts or paper-scale measurements.
The paper outline is maintained with the research workspace; this file is the
standalone release evidence boundary.

## Primary claims

| Claim | What the release must show | Current evidence | Status |
| --- | --- | --- | --- |
| **C1 — Machine abstraction.** GlassVM separates machine semantics from experimental orchestration. | One unchanged caller prepares and runs materially different bundles; bundle-specific semantics stay behind the bundle contract. | CHIP-8 is the baseline/reference implementation; PICO-8 and TIC-80 provide independently designed external validation specimens. Shared conformance and installed-wheel tests exercise the same prepared-run and facade paths across the three release bundles. Hexwell and Wyrd-16 are preserved on a separate parked branch and are outside this release denominator. | Architecture and three-machine behavior were exercised with GlassVM HEAD `28b02ec04fc525c1fdca886c01f9436167979d13` (working tree dirty only for experiment/docs changes at collection) and Machines `1aca8f625f141adb72b959e413a11fb9e371cf3e` (clean); current CPython 3.12/3.13/3.14 CI is green. The four 0.1.2 Python distributions were installed from the package index and exercised by the repeated measurement matrix. |
| **C2 — Observation inversion.** An experiment requests the evidence it needs through an explicit contract. | Preparation resolves exact channel, event-kind, native, frame, snapshot, final-state, and capability prerequisites before execution; emission and derived outputs remain separate. | `ObservationRequest`, `PreparedObservation`, exact dependency closure, optional downgrade, bounded normalizers, typed frame/native/input-value channels, and `EvidenceReceipt` tests are exercised by shared conformance cases for every release bundle. The installed-wheel path checks preparation and separate result domains. | Preparation and evidence semantics are verified across all three bundles; the current 3.12/3.13/3.14 source-distribution-to-wheel and provider matrix is green. All three published 0.1.2 bundle wheels and the generic facade were exercised from package-index installs. |
| **C3 — Practical selectable observation.** Observation cost is selectable, measurable, and suitable for long-running experimentation. | Measurements distinguish execution, normalization, sink, encoding, durable storage, memory, and file-count costs across observation selections and workloads. | Slice 5 records 135 published runs: nine exact observation recipes × three machines × five repetitions at 600 frames. Slice 6 now includes a clean 120-trial run-length matrix at one common source coordinate, in addition to the initial diagnostic matrix and CHIP-8 cleanup follow-up. Raw data, manifests, summaries, and plots are retained under [`experiments/measurements/`](experiments/measurements/). | On the common clean source pair and this WSL2 host, median peak RSS stayed within 1.5× between 600 and 600,000 frames for every machine/mode pair (three repetitions at shorter lengths; one at 600,000). This supports an observed-RSS statement scoped to the tested fixtures/configurations, revisions, and host—not a universal memory bound or cross-host performance claim. Allocator high-water was unavailable; persisted encoded evidence grew with emitted records. See [Slice 6](PAPER_RELEASE_SLICE_06.md). |

The machine-independent downstream analysis demonstration supports C2 and C3;
it is not a separate GlassVM claim. Derived metrics belong to bundle-owned
normalizers, while Refinery remains a consumer that scores or searches over
their outputs.

Glass Relay adds a focused C1/C2 example of shared experimental evaluation:
one Python consumer compares the same press/hold/release requirements across
three machines through normalized input acceptance and frame fingerprints.
It evaluates experiment-level requirements and introduces no machine-specific
capability or translation semantics. The original reference/delayed/stuck
cartridges are controls for the tester; matching measurements concern this
task and configuration, not general machine equivalence or execution speed.
The eighteen-run 0.3.0-wheel development matrix is documented in
[`glass-relay-0.3.0/SUMMARY.md`](experiments/measurements/glass-relay-0.3.0/SUMMARY.md),
with exact scope in [`glass_relay/README.md`](experiments/glass_relay/README.md).
Final paper regeneration remains a separate gate.

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
| Selective observation matrix | C2, C3 | [`experiments/measurements/`](experiments/measurements/) retains five repetitions for all nine exact recipes on CHIP-8, PICO-8, and TIC-80 at a 600-frame bound, including requested/fulfilled channels and separate receipts. |
| Bounded online execution | C3 | The clean 120-trial, three-machine/four-mode/four-length matrix observed approximately flat peak RSS for every tested pair; source revisions, limits, raw results, and claim caveats are recorded in [Slice 6](PAPER_RELEASE_SLICE_06.md). This is empirical evidence for the measured conditions, not a universal guarantee. |
| Recorder/storage cost | C3 | [`experiments/measurements/OBSERVATION_SUMMARY.md`](experiments/measurements/OBSERVATION_SUMMARY.md) reports per-channel logical/encoded bytes, block/segment/file counts, physical directory bytes, and publication outcomes for identical machine execution material. |
| Downstream analysis | C2, C3 | Refinery or equivalent caller consuming selected normalized projections and capability outputs without requiring whole-run GlassVM materialization. |
| Independent publication | C1, C2, C3 | Both repositories are public; the five machine-neutral GlassVM crates and all four Python distributions are available at `0.1.2`. Package-index-installed Python distributions executed successfully in the Slice 5 matrix. |
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

They may state that measured peak RSS stayed approximately flat over the
600–600,000-frame range for the three pinned workloads, four tested observation
modes, source pair, and WSL2 host, while explicitly noting the single longest
repetition and growth in persisted evidence. They must not generalize this to a
universal memory bound, cross-host performance, population-scale throughput, or
a completed machine-independent Refinery benchmark.

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
coordinates and their current Rust/Python CI are green. The original Slice 6
run-length measurement, CHIP-8 retention cleanup, and clean same-coordinate
three-machine rerun are complete. Slice 9 manuscript-wide regeneration and
immutable artifact sealing remain open gates. PyPI availability and
package-index-installed execution were verified by the Slice 5 collection.
