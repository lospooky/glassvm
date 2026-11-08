# Experiment coverage matrix and parameter proposal

Updated: 2026-11-08  
Status: **proposal for review; parameters not frozen; no trials run under this matrix.**

This document turns the approved experiment briefs and tracking plan into
case-level coverage. It is a proposal, not authorization to add test hooks or
begin resource-heavy trials. Existing published 0.3.1 implementations and
fixtures are the comparison baseline unless a separately approved test-only
revision is needed.

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
| F1 | Request an unsupported required capability and unsupported normalized/native evidence kind. | Preparation fails before `RunStarted`; no execution or publication outcome is reported as successful. | Public preparation path; proposed |
| F2 | Request a capability optionally while omitting one declared prerequisite. | Preparation succeeds; capability is explicitly unavailable with its reason; unrelated execution proceeds. | Public preparation path; proposed |
| F2-D | Exercise required/optional dependency closure, shared prerequisites, transitive dependencies, and an optional dependency failure. | Resolved prerequisite set and unavailable reasons; optional dependency does not escalate to global required failure. | Existing conformance seams first; any synthetic provider/hook needs approval |
| F3 | Raise from a Python consumer on an ordinary emission batch, capability output, and capability receipt. | Python delivery failure is structured and separate from machine execution, evidence, recorder, and publication outcomes. | Consumer can raise through public API; proposed |
| F4 | Return a Rust sink error at an ordinary emission and at a capability callback. | Same outcome separation as F3; preserve delivered-item count and failure boundary. | Small Rust harness may be required; show exact harness proposal for approval |
| F5 | Terminate the process before recorder finalization, including graceful stop and external kill. | Supervisor records exit/partial directory; returned outcomes are absent after kill and are not fabricated. | Deterministic stop point may require a hook; approval required |
| F6 | Exhaust one evidence budget under each supported overflow policy (`Fail`, `AllowIncomplete`). | `Fail` and `Truncated` paths, affected channel, observed bytes/items, machine result, and publication reported independently. | Public budgets; proposed |
| F7 | Complete recording while evidence fulfillment is incomplete. | Complete `RecorderReceipt` may coexist with incomplete/failed `EvidenceReceipt`; publication follows the actual publication gate. | Public API if a supported incomplete evidence request can be constructed |
| F8-C | Make recorder creation fail. | Execution/evidence status up to the failure point; recorder and publication outcomes remain distinct. | **Paused:** current fan-out classifies recorder creation errors as evidence-delivery failures; central separation repair needed before exercising this path |
| F8-W | Fail a recorder write while execution is emitting. | Machine result, evidence fulfillment, recorder failure, and publication status at the failure point. | **Paused:** current fan-out classifies recorder write errors as evidence-delivery failures; central separation repair needed before exercising this path |
| F8-F | Fail recorder close/receipt finalization before publication. | Recorder is not finalized; publication was not attempted. | Outcome split implemented; deterministic failure trigger still needed |
| F8-P | Fail atomic rename/publication after the recording has been finalized. | Finalized recorder receipt remains distinguishable from publication failure; a post-rename sync warning retains the published reference. | **Public trigger verified:** an online consumer creates the reserved destination on `RunFinished`; focused recorder test confirms finalized receipt survives `NotPublished` |
| F9 | Cause a controlled session/machine execution failure after preparation. | Machine execution failure does not erase already available evidence or mislabel recorder/publish outcome. | Exact deterministic fault mechanism pending; hook approval if needed |
| F10 | Compare a legitimate zero-emission channel with suppressed or interrupted expected evidence. | A requested channel with zero items may be `Complete` when the selected delivery path completed; the receipt does not establish that behavior occurred. Separately determine whether an injected drop can be accounted for. | Semantics **resolved by author**; a suppression hook requires a separate concrete proposal and approval |

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
The focused F8-P public-trigger test passes: a synchronous online consumer
creates the requested destination on `RunFinished`, after which staging
finalization succeeds and atomic publication reports `NotPublished` while
preserving the recorder receipt. No test-only hook was added. F8-C and F8-W
are paused because recorder create/write errors currently propagate through
the evidence sink and are misclassified as evidence-delivery failures; fix
that shared outcome boundary before running those cases. F8-F still needs a
deterministic trigger; propose its exact test-only failure point for explicit
approval before adding one.

## E3 — CHIP-8 corpus and evidence-volume sweep

### Candidate screen proposal

Begin with the 45 public-corpus metadata candidates already inventoried in
`experiments/chip8-public-candidates.tsv`. Before execution, manually resolve
their CHIP-8 profile discrepancy and verify source/license evidence. Do not
copy or redistribute ROM bytes. Run one isolated empty-input screening trial
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

The current manifest has one established workload per machine. A cross-machine
program-population comparison needs more than one specimen per machine, but the
candidate pools have different provenance and rights status. **Author-approved
target:** three cleared, executable workloads per machine, including the
established fixture where suitable. If three cleared
specimens are unavailable for PICO-8 or TIC-80, either narrow the claim to
repeated runs of the established fixture or pause CE1; do not treat repetitions
of one program as a multi-program campaign.

### Policy cells

Use the same program, configuration, input schedule, execution bound, and
downstream summary algorithm within each paired block. The proposal uses 600
frames for all CE1 cells; confirm this is sufficient for the selected carts.

| Policy | Requested evidence and retention | Main quantities |
| --- | --- | --- |
| P1 — comprehensive + recording | All normalized event kinds + frame hashes; the reference recorder retains the selected evidence. | Produced, processed, recorded, encoded and physical bytes; RSS and lifecycle time. |
| P2 — selective + recording | Workload-selected normalized kinds + frame hashes; the recorder retains that evidence. | Same counts/bytes; P1/P2 ratios. |
| P3 — comprehensive then filter | Same request as P1; a caller-owned online consumer filters to the P2 event set, counts event kinds and frame-hash changes, and retains only that compact summary; no recorder in the primary cell. | Produced versus processed versus retained bytes; consumer and lifecycle costs. |
| P4 — selective online reduction | Same request as P2; the same consumer computes the same summary online and retains only compact output. | Produced/processed/retained quantities and costs, paired with P3. |
| P5 — observation disabled | No evidence request; same execution material and controls. | Execution baseline, process RSS and wall time. |

Proposed repetitions: three per policy/workload, each in a fresh process. The
same compact summary from P3 and P4 makes downstream filtration comparable to
selection before execution. To show campaign-size arithmetic without presenting
it as measurement, report per-run values scaled to 10 and 100 independent runs.
If the brief requires observed multi-run campaign scaling rather than arithmetic
projections, add measured 10-run and 100-run batches only after the first cohort
pilot and resource review. At the proposed three workloads per machine, the
primary matrix is 3 machines × 3 workloads × 5 policies × 3 repetitions = 135
trials. If only one established workload per machine is cleared, it is 45
trials and supports a repeated-run comparison, not a program-population claim.

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

Until these gates are resolved, all values in this document remain proposals;
no S1–S6 campaign result should be represented as collected.
