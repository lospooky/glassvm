# GlassVM experimental gap closure plan and progress

Updated: 2026-11-08

Status: planning recorded; case-level matrix drafted, not frozen. Author
approved three cleared workloads per machine for CE1 and resolved zero-item
receipt semantics. The recorder-finalization/publication outcome split is
implemented and build-checked; F8-F/F8-P failure triggers are not yet approved
or run. No S1–S6 campaign trials have run under this plan.
A separate exploratory TIC-80 cart screen was run on 2026-11-08; its results
and rights limitations are recorded in
`experiments/tic80-corpus-pilot/README.md`. Q2–Q6 are resolved. Q1 has a
provisional CHIP-8 cohort recommendation; public-corpus screening is preferred
before the final freeze.

This document reconciles the **GlassVM: Experimental Gap Closure** brief and
the **New and revised experiments for §5 Evaluation** brief. Update its
checklists and execution ledger as each slice progresses.

## Scope and operating rules

The experiments address selective and bounded evidence, accounted absence and
separate outcome domains, and observations across different machine semantics.

- Produce reproducible artifacts and factual results before manuscript integration.
- Glass Garden and Glass Relay are outside this campaign.
- Reuse existing experiment infrastructure and conformance mechanisms.
- Predeclare parameters, expected contractual behavior, ordering, and repetitions.
- Run trials in independent processes and preserve negative and anomalous results.
- Report bugs with reproductions before attempting fixes.
- Distinguish expected behavior, observed behavior, and untested assumptions.
- Establish resource ceilings and artifact-retention rules before measurements.
- The current case-level proposal and freeze gates are in
  [`experiments/EXPERIMENT_MATRIX.md`](experiments/EXPERIMENT_MATRIX.md).

## Reconciliation of the briefs

| Topic | Combined disposition |
| --- | --- |
| Reruns | Restrict them to named methodological reasons or documented effects of approved code changes. |
| Campaign economics | Supplement E3's two-request corpus sweep with CE1's five-policy comparison across three cleared workloads per machine (author-approved target). E3 alone does not test downstream filtration or online reduction. |
| Retaining baseline | Use Rust for the primary E2 baseline because Figure 4 measures a Rust trial process. A Python retaining path needs a matched Python comparison. |
| Recorder failures | Separate creation, writing, finalization, and publication. Their effect on execution and fulfillment depends on the failure stage. Test F8's independence expectation rather than assuming it. |
| Failure coverage | Add controlled execution faults, missing emissions, and shared/transitive prerequisite cases to E1. |
| Unexpected defects | Pause the affected path, report immediately, and preserve the reproduction. Continue independent work. |
| Warmups | Preserve raw warmup records and exclude them explicitly from timing summaries. |
| Raw data and Git | Commit compact measurements, receipts, and regeneration material. Recordings may be deleted after measurement when they can be regenerated. |
| Writing | Produce separate draft methods/results paragraphs. Defer changes to §5, paper figures, tables, and claims. |

## Source, environment, and branch baseline

Prefer the evaluated implementation revisions where feasible.

| Repository | Evaluated revision |
| --- | --- |
| GlassVM | `3e621537118dc4855e6a4962b216612ada99b102` |
| glassvm-machines | `3a9d52385b82fc995510e85b6fd204afaf9960fd` |

Record distribution versions separately. The previous evaluation used `0.3.1`.
Verify the actual loaded provider and Rust dependency resolution. Record build
flags, binary identities, fixture digests, toolchains, CPU, OS, kernel, and
virtualization for the new trials.

Harness changes and test-only hooks receive separate recorded revisions and
diffs. Report proposed production changes before implementation and identify
existing measurements that would require rerunning.

Experimental scripts and results belong on the `paper-publication` branch in
the appropriate repository. Use the existing checkouts; do not create worktrees.

### Recorded start state (2026-11-08)

| Repository | Branch / revision | Working-tree state |
| --- | --- | --- |
| GlassVM | `paper-publication`, `80e5dc4d4f411f7129998da033c366fbc5584eb6` | One commit ahead of origin; that commit adds the PICO-8 candidate corpus. Existing untracked paper, feedback, plan, and pilot artifacts are preserved. The evaluated code revision `3e621537118dc4855e6a4962b216612ada99b102` is an ancestor. |
| glassvm-machines | `paper-publication`, `3a9d52385b82fc995510e85b6fd204afaf9960fd` | Clean; exactly the evaluated revision. |
| program-diffusion | `master`, `ddbd21c5ac8992a44e806ac4aedc699eef750756` | Clean; source for both CHIP-8 corpus inventories. |

No branch switch, reset, stash, worktree, or trial run was performed. A first
read-only API audit found that each bundle exposes separate `prepare_run` and
`prepare_observation` operations, while Python's `execute_prepared` calls the
shared `FileRunSession`. Its result exposes machine execution, evidence
fulfillment, Python delivery, recorder finalization, and publication as
separate fields. Failure-stage attribution remains to be exercised in E1.

## S0 — Specification and environment

- [x] Reconcile the briefs and record this tracking plan.
- [x] Inspect the existing observation recipes and Rust memory-trial harness.
- [x] Resolve Q4: use the usual WSL2 host for timing.
- [x] Resolve Q5: test-only hooks may be added with explicit confirmation for each proposal.
- [x] Resolve Q6: pause the affected path and report defects.
- [x] Resolve Q3: delete recordings after measurement when rerunnable; propose resource limits from pilots.
- [x] Resolve Q2: run the larger E3 corpus sweep on CHIP-8 and a smaller CE1 comparison across all three machines.
- [ ] Resolve Q1 at its stated gate.
- [x] Confirm the existing `paper-publication` branches and record starting repository state; create no worktrees.
- [x] Audit public preparation, execution, sink, recorder, and result seams;
  record the recorder-finalization/publication classification gap in the matrix.
- [x] Draft a coverage matrix per case, machine, request, and policy, including
  proposed outcomes, repetitions, and required artifacts. Author freeze remains
  open.
- [ ] Freeze parameters, seeds, ordering, resource limits, and retention rules.
- [ ] Record environments and verify fixtures and actual loaded implementation.
- [ ] Define raw trial records, including missing/unavailable outcome domains.

## S1 / E1 — Controlled fault injection and accounting

Run at least three repetitions per applicable case and machine. Record
preparation separately from machine execution, evidence fulfillment, consumer
delivery, recorder finalization, and publication.

| Case | Coverage | Status |
| --- | --- | --- |
| F1 | Required unsupported capability and unsupported evidence kinds | Planned |
| F2 | Optional capability with an unsatisfied prerequisite | Planned |
| F2-D | Required/optional dependencies, shared prerequisites, transitive closure, and optional dependency failure | Planned |
| F3 | Python callback exception at a deterministic boundary | Planned |
| F4 | Rust sink error at a deterministic boundary | Planned |
| F5 | Graceful interruption and process kill before recorder finalization, where supported | Planned |
| F6 | Budget exhaustion under each supported overflow policy | Planned |
| F7 | Published finalized physical recording with incomplete evidence, where permitted | Planned |
| F8-C | Recorder creation failure | Public filesystem trigger verified; recorder failure remains separate from completed machine/evidence paths |
| F8-W | Recorder write failure during execution | Public hard-limit trigger verified; recorder failure remains separate from completed machine/evidence paths |
| F8-F | Recorder finalization failure | Paused pending an explicitly approved deterministic trigger |
| F8-P | Publication failure after finalization | Public destination-collision trigger verified; finalized receipt is preserved |
| F9 | Controlled machine/session execution fault | Planned |
| F10 | Missing, suppressed, or interrupted expected evidence | Zero-item channel may be complete when delivery succeeds; receipt alone does not prove occurrence. Suppression injection needs an approved hook. |

- [ ] Choose public-API mechanisms first and document any test-only hooks.
- [ ] Specify expected attribution for each stage of recorder failure.
- [ ] Run the applicable matrix in isolated processes.
- [ ] Retain requests, preparation results, injected conditions, partial evidence,
  process exits, and available receipts.
- [ ] Distinguish supervisor observations after a kill from returned outcomes.
- [ ] Report deviations and unsupported cases with reproducible commands.
- [ ] Produce the five-domain matrix and F2/F6/F7 receipt reports.

## S2 / E3 + CE1 — Corpus and campaign economics

### E3 corpus sweep

#### CHIP-8 corpus decision record (2026-11-08)

- Provisional fallback: use the complete 75-entry classic `chip8` profile from
  the project-local autonomous corpus at program-diffusion revision
  `ddbd21c5ac8992a44e806ac4aedc699eef750756`. The corpus documents original
  project authorship, no-key-input design, and prior deterministic verification.
  Its 75 ROM digests are unique; related authored variants remain included and
  must not be described as independent authorship or statistically independent
  samples.
- Prefer independently authored candidates from
  `program-diffusion/chip8_roms/web_corpus/` if the eligibility screen supports
  a useful cohort. That corpus contains 797 externally sourced records; 448
  have no statically detected key-input instructions. Within the 318 records
  classified by the corpus verifier as classic CHIP-8, 45 also have explicit
  CC0/MIT/GPL terms: 38 CC0-1.0, two MIT, and five GPL variants. The metadata
  lists 16 author names. Forty are CC0 or MIT. Twenty have a static draw flag.
  A stricter intersection also requiring `machine_hint=chip8` yields 21.
- The verifier classifies 24 of the 45 as classic CHIP-8 even though their
  metadata hint says SCHIP or XO-CHIP; their reports list no extension
  instructions. Resolve this profile discrepancy during eligibility review.
  The 45 remain candidates, not approved fixtures: static absence of key-input
  instructions does not establish that a ROM progresses without input, and
  source/license provenance still needs review. No dynamic eligibility run or
  public-ROM corpus sweep has been performed. Do not redistribute the external
  ROM bytes as part of this work without a separate rights decision.
- The candidate inventory is recorded in
  `experiments/chip8-public-candidates.tsv`, keyed by ROM ID and SHA-256. It
  contains metadata references only; no external ROM bytes were copied.
- Final cohort, count, source revision, ROM digests, and exact-duplicate policy
  remain subject to author approval before E3 trials.

- [ ] Finish screening public CHIP-8 corpus candidates for no-input execution,
  exact CHIP-8 profile, source provenance, and usable license terms; then freeze
  the author-approved corpus, count, revision, digests, and duplicate policy.
- [ ] Freeze configuration, empty schedule, and 600-frame execution controls.
- [ ] Run each entry in its own process under selective normalized events plus
  hashes, and all normalized events plus full frames.
- [ ] Record preparation, all five outcomes, RSS, logical evidence bytes, and
  execution/lifecycle wall time.
- [ ] Report admission and termination categories, paired comparable runs,
  median/p95/max distributions, cumulative volumes, and ratios.
- [ ] Account for rejected programs and early termination in campaign totals.

### CE1 five-policy comparison

This supplements E3 to fulfill the first brief. Run the smaller comparison across
CHIP-8, PICO-8, and TIC-80; freeze the specific workloads before measurement.

- [ ] Select and freeze three cleared workloads per machine (author-approved
  target); establish provenance and rights before measurement.
- [ ] Declare explicit comprehensive supported requests for each bundle.
- [ ] Declare the downstream filtering/reduction algorithm.
- [ ] Pair identical execution material across five policies:
  comprehensive observation with recording; selective observation with recording;
  comprehensive observation followed by downstream filtering/reduction;
  selective observation with online reduction and compact retention; disabled.
- [ ] Measure requested, emitted, processed, retained, and recorded quantities
  separately, including encoded bytes and published directory size.
- [ ] Measure trial/controller RSS, wall time, batching/backpressure, and producer
  and consumer costs where independently instrumentable.
- [ ] Vary program count and run count, and sweep length on the smaller set.
- [ ] Report measured source-selection versus downstream-filtering costs.

Exploratory TIC-80 workload scouting (separate from CE1): 180 public catalog
candidates were screened on the installed TIC-80 provider. Thirty passed a
ten-frame, empty-input, changing-fingerprint screen with no direct input API
calls found by a source scan. This is not a frozen CE1 workload set. Only five
selected carts carry a clear license statement in embedded source; three have
template/ambiguous wording and 22 state no license. The downloaded carts remain
Git-ignored and local-only pending rights review. See
[`experiments/tic80-corpus-pilot/`](experiments/tic80-corpus-pilot/).

## S3 / E2 — Retaining baseline and longest-run repetitions

- [ ] Implement a Rust sink owning every delivered emission, capability output,
  and capability receipt.
- [ ] Match the existing reducer/recorder observation request exactly.
- [ ] Run 600, 6,000, 60,000, and 600,000 frames with three repetitions through
  60,000 and at least one at 600,000, under agreed resource ceilings.
- [ ] Bring reducer/null and recorder paths to three repetitions at 600,000,
  recording environment and source comparability.
- [ ] Preserve resource-limited terminations and largest completed lengths.
- [ ] Produce raw RSS and figure-ready retaining-baseline data.

Figure 4 clarifications found in the harness await retained-run verification:

- RSS comes from `/proc/self/status` (`VmHWM`) inside the Rust trial executable.
  Embedded Lua is included where applicable; Python orchestration is separate.
- Recorder mode requests workload-specific normalized kinds, frame hashes, and
  the required visual capability, with default evidence budgets and `fail`
  overflow policy.
- [ ] Confirm both statements against retained final-run requests and records.

## S4 / E4 — Observation-dependent execution differences

- [ ] Inspect execution metrics and any observation-independent final-state digest.
- [ ] Report an insufficient comparison seam and propose the minimal addition
  before modifying production code.
- [ ] Run all nine requests × three machines × three repetitions (81 trials).
- [ ] Require equal prepared execution identity within each comparison.
- [ ] Compare execution-side results independently of evidence selection.
- [ ] Compare frame fingerprints only where requested.
- [ ] Report differences and the limits of the available comparison.

## S5 / E7 — Extended nonempty-channel equivalence

- [ ] Extend fan-out comparison to all nine requests per machine.
- [ ] Run three repetitions per machine/request (81 trials, proposed).
- [ ] Use the existing Glass Relay reference carts and their scheduled
  press/release input as the input-driven E7 fixtures for CHIP-8, PICO-8, and
  TIC-80. Verify supported combinations and nonempty channel emission before
  measurement.
- [ ] Add at least one mixed request per machine with nonempty scheduled input,
  input-value evidence, and snapshots, with three repetitions (nine additional
  trials proposed, subject to supported combinations).
- [ ] Exercise item/byte limits and a valid oversized singleton.
- [ ] Compare prepared identities, execution metrics, receipts, capability
  outputs, logical records, and within-channel ordering.
- [ ] Report unsupported combinations and complete mismatch reproductions.

## S6 / E5 + E6 — Timing methodology

### E5 Rust lifecycle timing

- [ ] Run a new balanced block of disabled, reducer/null, and recorder modes at
  60,000 frames, at least three repetitions per machine/mode.
- [ ] Separate preparation, execution, processing, finalization, and publication
  timing where the interfaces permit independent measurement.
- [ ] Report medians, observed ranges, and raw values.

### E6 Python delivery timing

- [x] Select the usual WSL2 host for timing; record its host details during execution.
- [ ] Predeclare CPU affinity, ordering, background-load checks, and warmups.
- [ ] Run five modes × three machines × ten measured repetitions (450 trials),
  plus recorded warmups excluded from summaries.
- [ ] Report medians and IQRs or a predeclared bootstrap interval.
- [ ] Assess whether the data supports an overhead estimate; recommend cutting
  the experiment if its result remains inconclusive.

## S7 / C1 — Arithmetic and final factual handoff

- [ ] Calculate full-frame/fingerprint ratios from Table 2 for each machine.
- [ ] Calculate 1,000-run, 600-frame campaign volumes under both choices.
- [ ] Label arithmetic separately from measured campaign results.
- [ ] Distinguish rounded table inputs from underlying measurement inputs.
- [ ] Compare E3 totals with predictions matched to corpus eligibility/outcomes.
- [ ] Produce the cross-experiment report and remaining-gap list.
- [ ] Identify supported claims and claims requiring further evidence.

## Deliverables required per experiment

- [ ] Versioned scripts, configurations, fixtures/references, digests, and hooks.
- [ ] Environment manifests, exact commands, and source/build/provider provenance.
- [ ] Raw trial measurements, outcomes, identities, and available receipts.
- [ ] Machine-readable summary, table, and figure data where relevant.
- [ ] Separate draft methods/results paragraphs grounded in executed trials.
- [ ] Failure, anomaly, unsupported-case, and reproduction report.
- [ ] Regeneration command and artifact-retention policy.
- [ ] Logical commits on the relevant `paper-publication` branch.

## Outstanding author decisions

| ID | Question | Proposed default | Gate |
| --- | --- | --- | --- |
| Q1 | Which CHIP-8 corpus location/revision, count, and duplicate policy? | Prefer independently authored public ROMs if the public-corpus screen yields a suitable, legally usable no-input cohort. Provisional fallback: all 75 `chip8`-profile entries from `program-diffusion/chip8_roms/autonomous_corpus/manifest.json` at revision `ddbd21c5ac8992a44e806ac4aedc699eef750756`; exact ROM SHA-256 deduplication (the 75 digests are unique), retaining authored variants and disclosing that they are related. | E3 specification, before trials |
| Q2 | Should CE1 cover three machines or CHIP-8 only? | **Resolved:** run the larger E3 corpus sweep on CHIP-8 and a smaller five-policy CE1 comparison across all three machines. | Freeze CE1 workloads before measurement. |
| Q3 | What wall-clock, RAM, disk, and retention ceilings apply? Must large recordings go into Git? | **Resolved:** recordings may be deleted after measurement provided they can be regenerated. Commit compact measurements, receipts, and regeneration material. Propose numerical resource limits from pilot runs as work progresses; do not guess them in advance. | Agree on limits before resource-heavy measurements; establish regeneration before deletion. |
| Q4 | Timing host | **Resolved:** use the usual WSL2 host. Record host details for E5 and E6. | Before timing trials |
| Q5 | Test-only hooks and wrappers | **Resolved:** allowed with explicit user confirmation for each proposed hook or wrapper and appropriate commits. Report production changes first. | Confirmation before each hook |
| Q6 | Does a contract defect pause the whole campaign or only the affected path? | **Resolved:** pause the affected path and report immediately; continue independent work. | E1 execution |
| Q7 | How should E1 handle the combined recorder-finalization/publication result? | **Resolved and implemented:** finalize the staged recording separately, publish by atomic rename, and report post-rename sync problems as publication warnings. Recorder create/write failures are also latched separately from evidence delivery and machine execution. | F8-C/F8-W/F8-P public triggers verified; obtain explicit approval for an F8-F test-only trigger |
| Q8 | Does a requested channel with zero items prove that the behavior did not occur? | **Resolved:** no. A channel may be complete if its delivery path completed with zero items; the receipt accounts for fulfillment, not occurrence. Suppression detection needs an explicit test condition. | F10 matrix and any proposed hook |

Proposed defaults are not author decisions. Resolve mandatory gates before
dependent work. Mechanism questions, including final-state comparison and
input-value/snapshot support, belong in the S0 API audit.

## Execution ledger

| Date | Slice | Action and result | Artifact / revision | Next gate |
| --- | --- | --- | --- | --- |
| 2026-11-08 | Planning | Reconciled briefs and recorded this checklist. No new trials executed. | This document, initially uncommitted on `manuscript` | Q1–Q6 and S0 specification |
| 2026-11-08 | Planning | Author selected the usual WSL2 host for timing experiments. | Q4 resolved in this document | Record host details before E5/E6 |
| 2026-11-08 | Planning | Author allowed test-only hooks and wrappers with explicit confirmation before each addition and appropriate commits. | Q5 resolved in this document | Confirm each concrete hook before implementation |
| 2026-11-08 | Planning | Author chose to pause the affected path and report defects. | Q6 resolved in this document | Continue independent work where possible |
| 2026-11-08 | Planning | Author approved deleting recordings after measurement when rerunnable, and proposing numerical resource limits from pilots. | Q3 resolved in this document | Establish regeneration before deletion and agree on limits before resource-heavy runs |
| 2026-11-08 | Planning | Author approved the larger CHIP-8 E3 corpus sweep and a smaller CE1 five-policy comparison across all three machines. | Q2 resolved in this document | Freeze CE1 workloads before measurement |
| 2026-11-08 | Planning | Author set CE1's target at three cleared workloads per machine and confirmed a zero-item requested channel can be complete when delivery succeeds; a receipt does not establish behavior occurrence. | CE1 workload count and F10 semantics recorded in `experiments/EXPERIMENT_MATRIX.md` | Verify three specimens per machine; propose any suppression hook for approval |
| 2026-11-08 | Planning audit | Proposed a two-stage recorder finalization/publication boundary after finding that the current combined `finalize()` path cannot classify post-rename failures independently. No code changed. | Q7 and paused F8-F/F8-P in this document | Author review before repair implementation |
| 2026-11-08 | Recorder outcome repair | Split finalized staging from atomic publication, retained finalized receipts on publication failure, and represented post-rename sync failure as published with a warning. Updated Python result validation for `not_requested`, `not_attempted`, and `published_with_warning`. Formatting and crate/facade builds pass; tests and fault trials were not run. | `crates/recorder/src/lib.rs`, `python/facade/src/lib.rs` | Propose deterministic F8 failure triggers for explicit approval; then resume E1 cases |
| 2026-11-08 | Recorder/evidence separation and public triggers | Latched recorder create/write failures inside the shared fan-out so they do not become evidence-delivery failures or interrupt machine execution. Verified F8-C using a filesystem parent-path blocker, F8-W using the smallest valid recorder block limit, and F8-P using an online consumer that creates the publish destination at `RunFinished`. Recorder all-features suite passes (33 tests), formatting passes, and the Python facade check passes. | `crates/recorder/src/lib.rs`, `experiments/EXPERIMENT_MATRIX.md` | Propose the exact F8-F test-only hook for explicit approval |
| 2026-11-08 | Planning | Author selected the existing Glass Relay reference carts for E7's input-driven cases. | E7 fixture decision recorded | Verify supported combinations and nonempty evidence before measurement |
| 2026-11-08 | Planning | Drafted a case-level coverage matrix and parameter proposal for E1/E2/E3/E4/E5/E6/E7/CE1. No campaign trials or new hooks were run or added. | `experiments/EXPERIMENT_MATRIX.md` | Resolve freeze gates; pause F8-F/F8-P pending outcome-boundary decision |
| 2026-11-08 | Planning audit | Found `FileRunRecorder::finalize()` combines receipt finalization, atomic rename, parent sync, and opening the published run; `FileRunSession` reports every error as recorder-finalization failure and not-published. | Finding recorded in `experiments/EXPERIMENT_MATRIX.md` | User decision on characterizing current behavior or repairing the shared contract before F8-F/F8-P |
| 2026-11-08 | PICO-8 corpus pilot | Screened 30 autonomous-animation candidates from the author's 2017 tweetcart archive on the published 0.3.1 Python packages; all 30 failed during cartridge loading. The existing Fire Intro control completed 10 frames. The affected candidate path is paused for review; no final corpus was selected. | `experiments/pico8-corpus-pilot/README.md`, `pilot.py`, `results.json`, `control.json` | Review failures and examine independent candidate sources; preserve reported compatibility limitations. |
