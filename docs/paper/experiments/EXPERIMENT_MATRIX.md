# Experiment coverage matrix and parameter proposal

Updated: 2026-11-09

Status: **partial execution**. The approved CE1 selection matrix and E1's
required-capability rejection case have been run; remaining experiment cells
and their individual freeze gates are still open.

This document turns the approved experiment briefs and tracking plan into
case-level coverage. It is a proposal, not authorization to add test hooks or
begin resource-heavy trials. Existing published 0.3.1 implementations and
fixtures were the original comparison baseline. The completed CE1 and F1
trials instead used published GlassVM 0.3.2 and machine bundles 0.3.3 after
the approved outcome repairs; see their retained provenance.

## Common protocol

- Use the usual WSL2 host. Record OS/kernel, CPU, virtualization, toolchain,
  Python/uv versions, installed distributions, provider metadata, source
  revisions, build flags, fixture digests, and trial binary/package digests.
- Run each measured case in a fresh process. Preserve preparation, execution,
  evidence fulfillment, Python delivery, recorder finalization, and publication
  as separate outcome fields. Record an absent outcome as `not_started`,
  `not_requested`, or `unknown` with a reason; do not fill it with success.
- Pin the current published machine workloads, configuration, empty input
  schedule, and the existing nine recipes in `PAPER_WORKLOADS.toml` and
  `experiments/OBSERVATION_RECIPES.toml`. E3 uses 600 frames. E2/E5/E6 retain
  their separately stated lengths and workloads.
- For randomized scheduling, propose seed `20261108`, record the realized order,
  and balance machine/mode positions. This is a proposal; confirm before freeze.
- Correctness and corpus runs: three independent repetitions per retained cell
  unless the row below gives a screening-only count. Timing: use the exact
  repetitions already specified in the tracking plan. Keep warmups in raw data
  and exclude them from summaries.
- Record both the exact submitted request and resolved identities, requested
  and emitted channel counts/bytes, receipts, termination, wall-time
  components, peak RSS, and any available recording size. Retain compact JSON
  trials, manifests, summaries, request/configuration files, and fixture
  digests. Delete large recordings only after verifying regeneration.
- Establish disk/RAM/time ceilings from a representative pilot before the
  corpus sweep or 600,000-frame runs. Do not infer ceilings from older results.

## E1 — Fault and accounting cases

Run three independent reproductions per applicable case and machine. Start with
public request/API mechanisms. Any new test-only hook requires a concrete
proposal and the user's explicit approval before implementation.

| Case | Stimulus and coverage | Expected inspection | Gate / status |
| --- | --- | --- | --- |
| F1 | Request an unsupported required capability and unsupported normalized/native evidence kind. | Preparation fails before `RunStarted`; no execution or publication outcome is reported as successful. | After the event-kind negotiation repair, 27/27 unsupported requests were rejected and 3/3 supported controls accepted using locally built bundle wheels; execution was not invoked. |
| F2 | Request a capability optionally while omitting one declared prerequisite. | Preparation succeeds; capability is explicitly unavailable with its reason; unrelated execution proceeds. | Nine independent Python runs completed and receipts reported the optional input summary unavailable with its missing-channel reason. Prospective status is checked by Rust bundle conformance; Python preparation details are opaque. |
| F2-D | Exercise required/optional dependency closure, shared prerequisites, transitive dependencies, and an optional dependency failure. | Resolved prerequisite set and unavailable reasons; optional dependency does not escalate to global required failure. | Nine independent processes passed using a synthetic capability diamond over real bundle input catalogs; preparation only. |
| F3 | Raise from a Python consumer on an ordinary emission batch, capability output, and capability receipt. | Python delivery failure is structured and separate from machine execution, evidence, recorder, and publication outcomes. | 27 independent trials passed across the three machines and callback boundaries. Execution was interrupted by evidence failure; recorder finalization and publication did not occur. |
| F4 | Return a Rust sink error at an ordinary emission. | The budgeted evidence receipt records the rejected channel/item and failure; session code receives the sink error. | Nine independent per-machine sessions passed with a caller-owned rejecting consumer. Recorder/publication were not requested; Rust capability callbacks remain infallible. |
| F5 | Externally kill a child process while its Python consumer is blocked at `RunFinished`, before evidence/recorder receipts and finalization. | The child returns no fabricated result; partial staging remains inspectable and no published run exists. | Nine independent per-machine child processes were killed at the approved barrier. Staging remained; no result or published run appeared. Graceful stop is unsupported. |
| F6 | Exhaust one evidence budget under each supported overflow policy (`Fail`, `AllowIncomplete`). | `Fail` and `Truncated` paths, affected channel, observed bytes/items, machine result, and publication reported independently. | **Both policy tests passed in all 3 repetitions** |
| F7 | Complete recording while evidence fulfillment is incomplete. | Complete `RecorderReceipt` may coexist with incomplete/failed `EvidenceReceipt`; publication follows the actual publication gate. | **Passed in all 3 source-suite repetitions** |
| F8-C | Make recorder creation fail through a filesystem parent-path collision. | Execution/evidence status up to the failure point; recorder and publication outcomes remain distinct. | **Passed in all 3 repetitions:** execution and Python delivery complete; recorder fails and publication is not attempted |
| F8-W | Reject a recorder channel write with the smallest valid hard block limit. | Machine result, evidence fulfillment, recorder failure, and publication status at the failure point. | **Passed in all 3 repetitions:** execution/evidence delivery remain complete; recorder fails and publication is not attempted |
| F8-F | Inject a recorder finalization error after channel segments close but before the recorder receipt is written. | Machine/evidence outcomes remain complete; recorder finalization fails, staging remains inspectable, and publication is not attempted. | **Approved test-only-trigger test passed in all 3 repetitions** |
| F8-P | Fail atomic rename/publication after the recording has been finalized. | Finalized recorder receipt remains distinguishable from publication failure; a post-rename sync warning retains the published reference. | **Passed in all 3 repetitions:** finalized receipt survives the public destination-collision failure |
| F9 | Cause a controlled session/machine execution failure after preparation. | Machine execution failure does not erase already available evidence or mislabel recorder/publish outcome. | **Passed in all 3 repetitions; all three bundle conformance suites also passed** |
| F10 | Compare a legitimate zero-emission channel with budget-truncated evidence. | Zero items can be complete without proving occurrence; an allowed budget overflow is explicitly truncated/incomplete. | **Zero-frame and truncation tests passed in all 3 repetitions.** No silent-drop or expected-count claim is made. |

Repetition logs and the run manifest are in
[`measurements/e1-repetitions-0.3.2`](measurements/e1-repetitions-0.3.2/SUMMARY.md).
These are source-level test-suite repetitions, not a new program-behavior or
performance experiment.

### Recorder boundary defect and proposed repair

The original `FileRunRecorder::finalize()` closed channels, wrote the recorder
receipt, synced staging, renamed the directory, synced the parent, and opened
the published run in one result. `FileRunSession` therefore classified every
error as both recorder-finalization failure and not-published, including errors
after the rename gate.

**Repair implemented:** finalization now returns a finalized staging value;
publication separately validates that staging and attempts the atomic rename.
A rename failure preserves `RecorderFinalizationOutcome::Finalized(receipt)`
and reports `PublicationOutcome::NotPublished`. Once rename succeeds, the
published handle is constructed from the prevalidated contents. A later parent
directory sync failure reports `PublishedWithWarning` and keeps the handle. The
Python facade accepts the separate `not_requested`, `not_attempted`,
`not_published`, `published`, and `published_with_warning` outcomes. No
recorder format or staging recovery semantics changed.

Formatting and `cargo check` for `glassvm_recorder` and the Python facade pass.
`cargo fmt --all -- --check` and `cargo test -p glassvm_recorder
--all-features` pass as well (37 tests). The core all-features suite passes
(72 unit tests plus its prohibited-surface integration check).
The focused F8-P public-trigger test passes: a synchronous online consumer
creates the requested destination on `RunFinished`, after which staging
finalization succeeds and atomic publication reports `NotPublished` while
preserving the recorder receipt. No test-only hook was added.

**Recorder/evidence separation repair:** `RunEmissionSink` now latches a file
recorder create/write error as recorder failure and continues delivery to the
online evidence path. `BudgetedSink` therefore does not misclassify that
failure as evidence-delivery failure or interrupt machine execution. Public
trigger tests pass for F8-C (a file blocks parent-directory creation) and
F8-W (a valid one-byte hard block limit rejects the emitted record); both
confirm machine execution, evidence fulfillment, and Python delivery remain
complete while recorder failure and `NotAttempted` publication are reported
separately. The explicitly approved F8-F test-only trigger fails after channel
segments close and before the recorder receipt is written. Its focused test
confirms complete execution/evidence, failed recorder finalization, no
publication attempt, and inspectable staging without a recorder receipt.

## E3 — CHIP-8 corpus and evidence-volume sweep

### Candidate screen proposal

Begin with the 45 public-corpus metadata candidates already inventoried in
`experiments/chip8-public-candidates.tsv`. Three selected CE1 fixtures are
already retained in the machine repository. For additional candidates, resolve
their CHIP-8 profile discrepancy and verify source/license and redistribution
evidence before adding their bytes. Run one isolated empty-input screening trial
per candidate with a 600-frame bound and frame hashes. Keep every candidate
with admitted classic CHIP-8 execution, no execution/evidence failure, and the
600-frame bound outcome declared by the workload. Static “no key opcode” is not enough;
record any key-wait or early-stop behavior as an exclusion reason. Do not
require changing frame hashes: static but valid autonomous programs remain
useful evidence-volume specimens. Freeze the passing IDs, exact digests, source
and license references, and duplicate policy before the measured sweep. If the
public screen yields no suitable cohort, use the documented 75-entry local
classic corpus only after explicitly approving that provenance choice.

### Measured matrix

For each frozen eligible program, run two paired requests at 600 frames:

1. workload-selected normalized events + frame hashes;
2. all normalized events + full frames.

Use three fresh-process repetitions per program/request. Randomize paired-run
order by the common seed while keeping pairs close enough to limit host drift.
Report excluded and early-terminating candidates separately; do not silently
remove them from the denominator. Measure prepared/execution outcomes, frames,
logical evidence bytes, emitted counts, peak RSS, wall time, and (where enabled)
recorder logical/encoded bytes and physical directory bytes. Set storage limits
after a representative full-frame pilot, before expanding to the cohort.
The current candidate list has 45 entries, so the proposal entails up to 45
single screening runs plus six measured trials per eligible ROM; if all 45
qualify, the upper bound is 315 runs. This is a planning count, not a result.

## CE1 — Five evidence policies across three machines

### Workload question

Three workloads per machine were selected and pinned in the machine repository's
CE1 cohort manifests. Candidate-pool provenance and rights limits are recorded
in the main tracking plan and the cohort manifests.

### Policy cells

The completed matrix used 600 frames and the same program, configuration,
input schedule, and execution bound across five approved policies: disabled;
selected events plus hashes; selected events plus full frames; all events plus
hashes; and all events plus full frames. Three repetitions per workload and
policy produced 135 successful trials. Requests, receipts, raw records, and
summary are in `glassvm-machines/conformance/paper/ce1/`.

The following earlier proposal specifies the still-open downstream-filtering
and online-reduction comparison. P3 and P4 were not part of the completed
selection matrix; do not label that matrix as a source-versus-filter study.

| Policy | Requested evidence and retention | Main quantities |
| --- | --- | --- |
| P1 — comprehensive + recording | All normalized event kinds + frame hashes; the reference recorder retains the selected evidence. | Produced, processed, recorded, encoded and physical bytes; RSS and lifecycle time. |
| P2 — selective + recording | Workload-selected normalized kinds + frame hashes; the recorder retains that evidence. | Same counts/bytes; P1/P2 ratios. |
| P3 — comprehensive then filter | Same request as P1; a caller-owned online consumer filters to the P2 event set, counts event kinds and frame-hash changes, and retains only that compact summary; no recorder in the primary cell. | Produced versus processed versus retained bytes; consumer and lifecycle costs. |
| P4 — selective online reduction | Same request as P2; the same consumer computes the same summary online and retains only compact output. | Produced/processed/retained quantities and costs, paired with P3. |
| P5 — observation disabled | No evidence request; same execution material and controls. | Execution baseline, process RSS and wall time. |

For the outstanding P3/P4 comparison, the proposal is three repetitions per
policy/workload, each in a fresh process. The
same compact summary from P3 and P4 makes downstream filtration comparable to
selection before execution. To show campaign-size arithmetic without presenting
it as measurement, report per-run values scaled to 10 and 100 independent runs.
If the brief requires observed multi-run campaign scaling rather than arithmetic
projections, add measured 10-run and 100-run batches only after the first cohort
pilot and resource review. At the proposed three workloads per machine, the
earlier proposed five-mode matrix would have contained 135 trials. The actual
completed 135-trial matrix used the approved selection policies listed above;
additional downstream comparison counts must be specified separately.

## E2, E4, E7, E5, and E6 coverage

| Experiment | Frozen cells / proposed count | Primary comparison and outputs |
| --- | --- | --- |
| E2 retaining baseline | 3 machines × retaining, reducer/null, recorder modes × 600, 6,000, 60,000, 600,000 frames; three reps through 60,000, at least one at 600,000. Bring reducer/null and recorder to three reps at 600,000. | RSS and retained/recorded evidence versus run length; separately report incomplete/resource-limited trials. |
| E4 observation dependence | Existing 9 recipes × 3 machines × 3 reps = 81 runs. | Compare `PreparedRunIdentity` and machine execution results within machine across observation contracts; keep `PreparedObservationIdentity` distinct. |
| E7 fan-out equivalence | Existing 9 recipes × 3 machines × 3 reps = 81 runs, plus one mixed input/value/snapshot request × 3 machines × 3 reps = 9 proposed runs. | Rust/Python logical evidence, identities, capabilities, receipts, and per-channel order; no cross-channel ordering claim. Use Glass Relay input carts. |
| E5 lifecycle timing | Disabled, reducer/null, recorder × 3 machines × 3 reps = 27 at 60,000 frames. | Preparation, execution/consumer, finalization, and publication timing where separately measurable. |
| E6 Python delivery | Existing five modes × 3 machines × 10 measured reps = 450, plus one recorded warmup per cell (proposed). | `observation_disabled`, `observed_no_external_sink`, `python_consumer`, `rust_recorder`, and combined consumer+recorder. Report medians/IQR and batches/items/bytes/RSS. |

E7's mixed request must first be negotiated on all three bundles. If any
requested combination is unsupported, record the preparation result and propose
the nearest semantically valid common request; do not substitute one machine's
schema silently.

## Freeze gates before first measured trial

1. Approve the final CHIP-8 cohort and identify three cleared CE1 specimens per
   machine with verified provenance/rights.
2. For every proposed hook above, review its exact seam and failure trigger and
   explicitly approve it before implementation. Public-API cases may proceed
   without hooks after the matrix is frozen.
3. Run one representative E3 full-frame pilot and one E1 path pilot to set
   numeric resource ceilings. Record those values before cohort expansion.
4. Freeze actual randomized execution order, warmup handling, raw-record schema,
   storage cleanup verification, and regeneration commands in the manifest.

These gates apply to the remaining unrun cells. The CE1 selection matrix and
F1 required-capability trials were separately specified and completed; their
retained manifests are the authority for their actual parameters and results.
