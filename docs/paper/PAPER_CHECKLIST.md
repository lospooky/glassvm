# GlassVM paper ship-gate checklist

Goal: determine whether `lospooky/glassvm` and `lospooky/glassvm-machines` are ready to support the paper as a reproducible public artifact.

Do not redesign architecture unless a checklist item exposes a concrete claim-breaking defect. Prefer evidence, tests, measurements, and release hardening over new abstractions.

## 1. Repository / release boundary

- [x] Confirm `glassvm` and `glassvm-machines` are both public and buildable from clean clones.
- [x] Record exact `main` commit SHAs for both repositories.
- [x] Confirm `glassvm-machines` release denominator is exactly:
  - CHIP-8
  - PICO-8
  - TIC-80
- [x] Confirm Hexwell and Wyrd-16 remain outside the release denominator.
- [x] Confirm no release-facing docs still imply Hexwell/Wyrd are paper specimens.
- [x] Confirm no release-facing docs refer to obsolete architectural names or compatibility layers.
- [x] Confirm LICENSE and package metadata are consistently BSD-3-Clause except legitimate third-party fixture provenance.
- [x] Confirm no publishable package still contains local workspace-only dependency assumptions.
- [x] Confirm no release-facing crate or wheel accidentally depends on unpublished/private repository state.

Output:
- exact SHAs
- release-specimen list
- any stale release/documentation references found

## 2. CI and clean-room packaging

- [x] Confirm current `glassvm` CI passes on `main`.
- [x] Confirm current `glassvm-machines` Rust + conformance CI passes on `main`.
- [x] Confirm Python clean-room packaging passes for:
  - CPython 3.12
  - CPython 3.13
  - CPython 3.14
- [x] Confirm clean-room tests cover:
  - base facade only
  - CHIP-8 provider only
  - PICO-8 provider only
  - TIC-80 provider only
  - all providers installed together
- [x] Confirm provider discovery occurs only through `glassvm.machine_bundles`.
- [x] Confirm the generic facade has no static registry or bundle-specific imports.
- [x] Confirm Python provider protocol is exactly:
  - `glassvm.python_bundle`
  - numeric version metadata
  - `prepare_run`
  - `execute_prepared`
- [x] Confirm no `v1`-style chronology is embedded in protocol identifiers.
- [x] Update stale docs that still say isolated CI is pending.

Output:
- CI run links / run IDs
- exact jobs passed
- list of stale CI-status claims fixed

## 3. Architecture/conformance sanity pass

Do not refactor unless a violation is found.

Verify all three release bundles satisfy the same contract surface.

- [x] Artifact admission is explicit and validated.
- [x] Configuration is typed/schema-qualified and resolved during preparation.
- [x] Input catalog is explicit.
- [x] Scheduled inputs are typed and ordered by ordinal.
- [x] Execution controls are explicit and participate in prepared execution identity.
- [x] Observation is negotiated before execution.
- [x] Required prerequisites fail before `RunStarted`.
- [x] Optional unavailable capabilities downgrade explicitly with reasons.
- [x] Normalized events, native evidence, frames, snapshots, input values, capability outputs, evidence receipts, and recorder receipts remain distinct.
- [x] `PreparedRun` contains execution/input state only.
- [x] `PreparedObservation` contains negotiated observation state only.
- [x] Normalizers are bounded online reducers.
- [x] Normalizers do not query recorder storage.
- [x] Finalizers receive only explicitly negotiated final state.
- [x] Emissions are synchronous and borrowed.
- [x] Caller owns sink/persistence policy.
- [x] Query code consumes caller-supplied finite projections and does not reconstruct a canonical whole-run trace.
- [x] File-backed execution keeps:
  - execution outcome
  - evidence outcome
  - recorder outcome
  - publication outcome
  separately inspectable.
- [x] Atomic publication still requires finalized recorder state.
- [x] Interrupted staging remains inspectable/cleanable and is not silently resumed.
- [x] `NullSink` remains truly zero-retention.
- [ ] PICO-8 and TIC-80 visual reducers consume bounded graphics-activity and
  input-application evidence with exact prerequisites; TIC-80 retains its
  distinct sampled-input and trace summaries. See the alignment plan in
  `glassvm-machines/conformance/CAPABILITY_GRANULARITY_PLAN.md`.

Output:
- one conformance matrix across CHIP-8 / PICO-8 / TIC-80: [PAPER_CONFORMANCE_MATRIX.md](PAPER_CONFORMANCE_MATRIX.md)
- only report architecture defects if they threaten a paper claim

## 4. Prohibited-surface audit

Search both repositories for old or forbidden surface.

At minimum check for accidental release-facing use of:

- [x] `ObservationPlan`
- [x] `RunConfig`
- [x] `TraceChunk`
- [x] `ReplayManifest`
- [x] `ReplayPackage`
- [x] `ReplayReceipt`
- [x] `ObservableAdapter`
- [x] `native_event_adapter`
- [x] `observable_adapter`
- [x] `record_function`
- [x] `record_run`
- [x] `body_provider`
- [x] `CapabilityBlob`
- [x] `glassvm_episode`
- [x] obsolete Python protocol names
- [x] compatibility aliases/fallbacks

Historical docs may contain old terminology if clearly quarantined as history. Release-facing/public API paths may not.

Output:
- exact hits
- classify each as:
  - acceptable historical reference
  - remove
  - claim-breaking

## 5. Paper claim map reconciliation

Review `docs/paper/CLAIM_EVIDENCE_MAP.md` against current repository reality.

For each claim, classify:

### C1: machine abstraction
- [x] Confirm same unchanged caller path works for CHIP-8, PICO-8, TIC-80.
- [x] Confirm bundle-specific semantics remain behind bundle contracts.
- [x] Confirm clean-room provider matrix is now complete.
- [x] Update status from “isolated CI pending” if still present.

### C2: observation inversion
- [x] Confirm all preparation/failure behavior required by the claim exists in tests.
- [x] Confirm required/optional prerequisite behavior is tested in every release bundle.
- [x] Confirm typed config/input admission is tested.
- [x] Confirm receipts expose requested/supported/fulfilled distinctions.

### C3: practical selectable observation
- [x] Mark architectural mechanism as complete.
- [x] Measure selectable-observation costs and run-length peak RSS across the three release specimens; keep claims scoped to measured workloads, source pins, and host.

Output:
- updated claim-evidence table
- exact remaining evidence gaps only

## 6. Heterogeneity table for the paper

Generate a machine-comparison table showing that the three specimens are materially different while using the same orchestration contract.

Capture at least:

- [x] artifact representation
- [x] execution model
- [x] configuration model
- [x] input model
- [x] timing/coordinate semantics
- [x] frame representation
- [x] normalized event families
- [x] native evidence families
- [x] snapshot support
- [x] capability/normalizer set
- [x] verifier/static-analysis support
- [x] Python provider surface

Output:
- machine-readable source data: [PAPER_HETEROGENEITY.json](PAPER_HETEROGENEITY.json)
- deterministic generator: [generate_heterogeneity.py](generate_heterogeneity.py)
- generated tables: [Markdown](PAPER_HETEROGENEITY.md) / [LaTeX](PAPER_HETEROGENEITY.tex)
- execution evidence and scope limits: [PAPER_RELEASE_SLICE_03.md](PAPER_RELEASE_SLICE_03.md)

## 7. Selective-observation benchmark matrix

Build a reproducible benchmark harness.

Slice 4 harness and pilot status:

- [x] Add the isolated-process provider runner, nine explicit recipes, versioned result schema, and summary regeneration.
- [x] Smoke all nine recipes on CHIP-8, PICO-8, and TIC-80 (27 independent eight-frame trials); all published with complete evidence and recorder status.
- [x] Preserve separate machine, evidence, recorder, and publication outcomes; capture request, prepared-observation identity, timing, RSS, and physical output size.
- [x] Record the reproducible execution inputs explicitly (artifact digest, configuration, schedule, controls, and provider/package metadata), separately from observation and its negotiated identity. `PreparedRun` remains opaque; exposing its internal identity is not required for this experiment.
- [x] Regenerate the 27-cell pilot through published `0.1.2` Python distributions and validate recorder receipt `1.1.0` per-channel counters end-to-end.

The pilot validates wiring, not performance or run-length boundedness. The
full benchmark matrix below remains open. See
[`PAPER_RELEASE_EXECUTION_PLAN.md`](PAPER_RELEASE_EXECUTION_PLAN.md) and the
[pilot report](experiments/pilot/PILOT_SUMMARY.md).

For representative workloads on each release machine, run:

- [ ] observation disabled
- [ ] summary/minimal observation
- [ ] selected normalized events
- [ ] complete normalized events
- [ ] selected native evidence
- [ ] frame hashes / compact frame evidence if supported
- [ ] full frame capture
- [ ] capability outputs
- [ ] representative mixed observation profile

For each mode record:

- [x] wall-clock runtime (isolated worker-process and lifecycle timings)
- [x] execute-and-publish duration separately; pure emulator-only duration is not exposed
- [x] item counts per channel
- [x] logical bytes per channel
- [x] encoded bytes per recorder channel
- [x] recorder block count
- [x] segment count
- [x] file count
- [x] publication result
- [x] evidence receipt status
- [x] recorder receipt status
- [x] peak RSS (process high-water, including interpreter/provider startup)
- [x] allocator high-water checked; unavailable from the Python provider protocol

Use identical machine workload/config/input when comparing observation modes. **Complete:** five repetitions per cell, 600-frame bound, nine exact recipes, three machines; all returned work coordinates matched within each machine and all 135 runs published with complete receipts. See [`experiments/measurements/OBSERVATION_SUMMARY.md`](experiments/measurements/OBSERVATION_SUMMARY.md).

Output: **Complete** — raw versioned result files and one generated normalized result table are retained under `experiments/measurements/`.

## 8. Run-length bounded-memory experiment

This is a paper-critical C3 experiment.

Completed across four geometrically increasing execution lengths:

- [x] 600 frames
- [x] 6,000 frames
- [x] 60,000 frames
- [x] 600,000 frames (one repetition; three repetitions at shorter lengths)

run at least:

- [x] observation disabled
- [x] selected normalized events + `NullSink`
- [x] selected events, hash frames, and bounded visual normalizer + `NullSink`
- [x] same normalizer request + durable recorder

Measure:

- [x] peak RSS (`VmHWM` in isolated Linux processes)
- [x] allocator high-water checked; not exposed by this path
- [x] logical emitted volume
- [x] durable encoded volume
- [x] runtime

Check:

- [x] The original 120-trial matrix exposed CHIP-8 RSS growth in all four paths, including disabled observation (5.65 MiB at 600 frames; 113.64 MiB at 600,000). PICO-8 and TIC-80 stayed approximately flat on this host.
- [x] Remove CHIP-8's retained per-frame hash history, unique-frame set, per-transition visual metric histories, and all-run native event log. Keep only scalar event counters in the engine; let the bounded visual normalizer consume requested frame artifacts.
- [x] Remove the legacy `chip8.interestingness` and `chip8.trajectory_identity` native capability outputs; keep frame hashes as a capture mode and visual motifs as a derived capability. Version the changed `chip8.execution_summary` schema.
- [x] Repeat all four CHIP-8 observation/sink paths at the four run lengths (40 trials). RSS remained about 5.0–6.1 MiB across the matrix; recorder bytes are still reported separately and grow with persisted evidence.
- [x] Prepare the clean-versioned regeneration path: require exact clean checkout revisions, build the locked release example, record source/tag/lockfile/binary digests, and reject stale or mismatched resume data. Validate equality of frame/cycle totals, boot outcome, and termination.
- [x] Repeat the full 120-trial, three-machine matrix from clean checkouts at one common source coordinate. All 120 runs and evidence receipts completed; all 30 recorder runs finalized and published; observation-mode execution outcomes matched per machine and frame limit.

Original baseline data remains under
[`experiments/measurements/memory-scaling/`](experiments/measurements/memory-scaling/).
The CHIP-8 cleanup follow-up, raw trials, manifest, generated summary, and plots
are under
[`experiments/measurements/chip8-retention-cleanup/`](experiments/measurements/chip8-retention-cleanup/).
The clean three-machine rerun, raw trials, manifest, generated summary, and
plots are under
[`experiments/measurements/memory-scaling-three-machine-a7dd374-3edb0ae/`](experiments/measurements/memory-scaling-three-machine-a7dd374-3edb0ae/)
and documented in [Slice 6](PAPER_RELEASE_SLICE_06.md). Across all three
machines and four modes, endpoint median peak RSS stayed within 1.5× from 600
to 600,000 frames on the recorded WSL2 host. This is workload- and host-bounded
evidence, not a universal memory guarantee; durable encoded evidence grew with
run length, as expected.

## 9. Sink-cost decomposition

The Rust decomposition is complete for the current exploratory coordinate:
three repetitions at 60,000 frames on CHIP-8, PICO-8, and TIC-80. The same
artifact/configuration/schedule/controls were used; NullSink and recorder
requests, run IDs, execution results, and evidence receipts match. Results are
in [`experiments/measurements/sink-cost-rust-7a-corrected/SINK_COST_SUMMARY.md`](experiments/measurements/sink-cost-rust-7a-corrected/SINK_COST_SUMMARY.md).
These dirty-checkout results must be regenerated from clean pinned revisions
before paper use.

Remaining consumer comparison:

- [x] execution / observation disabled
- [x] observation + `NullSink`
- [x] observation + Python consumer (complete three-machine matrix; see Slice 7b)
- [x] observation + bounded Rust recorder

Python-consumer implementation status (checked 2026-10-12): the bounded
synchronous bridge is implemented in `glassvm_recorder`, wired through the
CHIP-8/PICO-8/TIC-80 providers, and smoke-tested with both consumer-only and
consumer-plus-recorder runs. The bridge reuses each compact JSON record
encoding for byte accounting and Python decoding; this is transport plumbing,
not a performance result. GlassVM `0.2.0`, including the recorder Python
feature, and all three provider wheels are now published. Machine lockfiles
resolve the published GlassVM crates, and a fresh Python 3.12 environment
installed and exercised the published facade, individual providers, and
`glassvm-machines[all]`; see [Slice 8](PAPER_RELEASE_SLICE_08.md). This smoke
validates packaging and lifecycle wiring, not consumer performance.

Slice 7b execution record: 45 isolated trials (three machines, five modes,
three repetitions) completed using the published `0.2.0` facade and provider
wheels in Python 3.14.3 on WSL2. Both paper-branch source checkouts were clean;
the exact revisions and provider metadata are in the run manifest and raw
trials. The generated summary and data are in
[`experiments/measurements/python-sink-cost-0.2.0/`](experiments/measurements/python-sink-cost-0.2.0/).

The Python consumer added a median 1.957 s (31.4%) for CHIP-8, 0.442 s (1.3%)
for PICO-8, and 1.155 s (8.0%) for TIC-80 relative to observed execution with
no external sink. PICO-8's observed/no-external-sink median was 33.497 s,
versus 14.375 s for TIC-80 and 6.239 s for CHIP-8 on their respective fixtures.
These are descriptive end-to-end timings, not normalized cross-machine speed
claims: each machine has distinct semantics and workload. In particular, the
PICO-8 wall time is dominated by its execution baseline, not incremental Python
consumer delivery. Full manuscript-wide regeneration remains deferred.

Completed Slice 7b gates:

- [x] Publish the recorder crate version containing the Python feature as part
  of GlassVM `0.2.0`.
- [x] Resolve machine lockfiles against published registry crates and exercise
  published provider wheels without local package/source overrides.
- [x] Run the clean pinned three-machine, 60,000-frame, three-repetition
  Python-consumer matrix; retain raw trials and review the generated summary.

Purpose:

separate
- machine execution cost
- emission cost
- Python consumer cost
- encoding/storage cost

Output:
- table with absolute runtime and relative overhead
- preserve same prepared workload wherever possible

## 10. Rust ↔ Python semantic-equivalence check

Completed 2026-10-12. For each machine, one prepared execution
used a Python provider and synchronously fanned the same emissions
to a streaming Python consumer and the Rust `FileRecorder`. The Rust recorder
reader decoded the published channels; the harness compared canonical logical
values and per-channel order. See
[`experiments/measurements/semantic-equivalence-0.2.0-prepared-identity/REPORT.md`](experiments/measurements/semantic-equivalence-0.2.0-prepared-identity/REPORT.md)
and [`experiments/run_semantic_equivalence.py`](experiments/run_semantic_equivalence.py).

Results: CHIP-8 (2,568 normalized / 1 native / 600 frame / 1 capability
records), PICO-8 (600 / 1 / 600 / 1), and TIC-80 (1,200 / 1 / 600 / 1) all
matched. Capability receipts, `EvidenceReceipt`, and terminal machine metrics
also matched. Snapshot and input-value evidence were not requested.

Both `PreparedRunIdentity` and prepared-observation identity matched exactly
between Python `RunStarted` and the Rust recorder manifest. The opaque
provider-owned prepared object remains in-process and non-serializable; only
the existing canonical identity value is emitted as provenance.

- [x] compare canonical logical records per channel
- [x] preserve per-channel order
- [x] do not invent global cross-channel ordering
- [x] compare receipts and capability outputs
- [x] do not compare raw `.gvmseg` bytes as semantic identity
- [x] independently compare `PreparedRunIdentity` without making
  provider-owned prepared state serializable or public

Output:
- [x] deterministic equivalence report and first-mismatch diagnostics

## 11. Package-index-only install proof

Package-index Python execution was first exercised by the 135 Slice 5 trials
using `0.1.2`. After the `0.2.0` release, package-index installation was
verified first with repository-backed conformance fixtures, then independently
with the no-checkout consumer project documented in
[`PAPER_RELEASE_SLICE_08.md`](PAPER_RELEASE_SLICE_08.md). The latter downloads
the pinned public workloads directly from upstream, verifies the expected
hashes, and runs using only installed wheels. Its base-only, each-provider, and
all-provider environments passed. A minimal external Rust consumer also built
and ran against the five published crates with no local path dependencies.

Package-index Python execution (evidenced by Slice 5):

- [x] install generic `glassvm` Python facade from package index
- [x] install CHIP-8 provider from package index
- [x] discover CHIP-8 by entry point
- [x] prepare and execute
- [x] produce receipts
- [x] produce published run

Repeat independently for:

- [x] PICO-8
- [x] TIC-80
- [x] all three installed together

External consumer project:

- [x] Run from a fresh temporary project with no GlassVM or machine checkout on its import path.
- [x] Download pinned public workload fixtures independently and verify their expected SHA-256 values.
- [x] Verify facade-only, each provider individually, and all provider wheels through the external caller.

For Rust:

- [x] verify released crates resolve without local path overrides
- [x] verify minimal external consumer project builds against published versions

External proof sources and lockfiles:
[`experiments/external-consumer-proof/`](experiments/external-consumer-proof/).
See [Slice 8](PAPER_RELEASE_SLICE_08.md) for exact versions, environment, and
results.

Output:
- commands
- exact versions
- clean-environment logs
- package URLs/identifiers

## 12. Immutable paper artifact

Before manuscript submission:

- [ ] create immutable tags/releases for exact evaluated commits
- [ ] ensure paper cites those exact coordinates
- [ ] record crate/wheel versions
- [ ] record toolchain versions
- [ ] record benchmark host characteristics
- [ ] record fixture hashes
- [ ] record benchmark configuration hashes
- [ ] ensure result files include source commit SHAs
- [ ] ensure paper scripts refuse or warn on mismatched source coordinates

Output:
- one `PAPER_ARTIFACT.md` or equivalent manifest containing all immutable coordinates

## 13. Downstream machine-independent consumer demo

Completed 2026-10-12 as Glass Garden. One generic Python sink consumer requests
frame hashes and normalized events only, then produces three SVG postcards and
one summary table. It uses a previous fingerprint, 20 fixed frame-coordinate
buckets, bounded event-kind counts, and the final `EvidenceReceipt`; it does
not read recorder files or retain a run's events or frames.

The same caller ran through the installed CHIP-8, PICO-8, and TIC-80 providers
with complete evidence receipts. The postcards report only changed adjacent
fingerprint pairs by bucket and top normalized event-kind counts. They make no
performance, complexity, or behavioral-equivalence claim.

- [x] create one generic caller consuming selected normalized event kinds and frame hashes
- [x] run it against all three release machines without bundle-specific orchestration branches
- [x] demonstrate bounded incremental analysis without whole-run GlassVM materialization
- [x] do not reopen Refinery architecture
- [x] do not add new GlassVM abstractions for this demo

Artifacts: [`experiments/glass_garden.py`](experiments/glass_garden.py),
[`experiments/test_glass_garden.py`](experiments/test_glass_garden.py), and
[`experiments/measurements/glass-garden-0.2.0/SUMMARY.md`](experiments/measurements/glass-garden-0.2.0/SUMMARY.md),
which links the three generated postcards.

Output:
- [x] small reproducible script
- [x] result table and three postcards
- [x] short description of the generic downstream projection

### Cross-machine comparative extension: Glass Relay

Glass Relay adds a common interactive task to the downstream demonstration.
One generic Python tester checks response delay, stable hold, and display
restoration for three purpose-built cartridge variants on each release machine.
Input bindings/configurations are explicit experiment data. Every frame carries
the desired input value, accommodating both latched keys and TIC-80's
frame-scoped gamepad masks through the same schedule-construction path.

- [x] Reproducible reference, three-frame-delayed, and stuck cartridge variants for CHIP-8, PICO-8, and TIC-80; sources stay on the Machines paper branch.
- [x] One bounded consumer over `InputApplied` and fingerprint-only frame artifacts, with no machine/variant evaluation branches.
- [x] Compare trial measurements across machines and across independent repetitions; keep prepared identities as supplied diagnostic provenance.
- [x] Report uncertainty as inconclusive, preserve outcome domains, and avoid cross-channel order or cross-machine fingerprint comparisons.
- [x] Run the 3-machine × 3-variant × 2-repetition matrix on installed 0.3.0 wheels: reference response/restoration 0/0; delayed 3/0; stuck restoration absent. All controlled outcomes and common protocol measurements match.
- [x] Add focused attribution/uncertainty tests and a `relay` stage to the single paper runner, including `all`.

This focused working-tree validation does not replace final regeneration or
establish general machine equivalence. The exact protocol and claim limits are
in [`experiments/glass_relay/README.md`](experiments/glass_relay/README.md).
Results: [`glass-relay-0.3.0/SUMMARY.md`](experiments/measurements/glass-relay-0.3.0/SUMMARY.md).

## 14. Paper regeneration

Local follow-up (2026-10-21): unreleased PICO-8/TIC-80 builds now expose
independently selectable per-frame `glassvm.graphics_activity` summaries.
Focused conformance, bounded Python projections, and short before/after
matrices passed; no full paper regeneration ran. See the machines repository's
`conformance/GRAPHICS_ACTIVITY.md` and the execution plan's **Immediate next work**.
The released 0.3.0 evidence remains unchanged. New observability/selection
claims require reviewed release coordinates, updated pins, and affected
regeneration before sealing.

The next local slice also adds `glassvm.execution_activity` and
`glassvm.rng_activity`, with fixed-size per-frame accounting and exact
selection. It includes the separately approved TIC-80 seeded-execution fix
(`tic80-lua-semantics.v3`) and rejection of no-argument host-entropy reseeding.
Core/bundle conformance and targeted installed-wheel/spot checks passed;
`glassvm-machines/conformance/RUNTIME_ACTIVITY.md` records scope and results.
The new execution identity and evidence are not retroactively part of 0.3.0.

The visual-capability decision is to align PICO-8 and TIC-80 with CHIP-8's
broader bounded frame/display/input profile, not narrow existing capabilities
to frames only. Their reducers now include selected graphics API-call and
input-application facts; TIC-80 retains its frame-level input sample and trace
counts. Targeted conformance and Python consumer/recorder verification remains
open, so this is not yet a completed parity or regenerated-paper claim.

Create a single reproducible path that regenerates every paper number/figure/table.

Expected shape:

```text
paper/
  experiments/
  results/
  tables/
  figures/
  regenerate.sh
```

Verify:

- [x] Provide one documented runner that creates a disposable locked Python 3.12 environment, disables uv cache reuse, and isolates Cargo registry state and build targets; pin the published facade and machine distributions to 0.3.0.
- [x] Verify all three workloads rebuild byte-for-byte, run the harness tests and generated-table freshness check, and build a standalone Rust consumer using only the five published crates.io 0.3.0 crates from a fresh Cargo home.
- [x] Re-run the base-only / individual provider / all-provider Python consumer matrix using separate fresh Python 3.12 environments and no uv cache.
- [ ] every quantitative manuscript claim maps to a versioned result file
- [ ] every figure is generated from checked-in/public raw data
- [ ] every table is generated rather than manually transcribed
- [ ] scripts record source commit SHAs
- [ ] scripts record machine/toolchain metadata
- [ ] no result depends on private paths or unavailable fixtures

Output:
- one command that regenerates all paper artifacts

Status (2026-10-12): `experiments/README.md` documents the unified
[`run_paper.py`](run_paper.py) entry point and its `all` pipeline. Preflight
passed from a newly-created Python environment and empty Cargo home/target;
the standalone Rust consumer resolved all five 0.3.0 crates from crates.io,
and all five Python install selections passed in isolated environments. The
full clean 0.3.0 measurement regeneration has not been run; result sealing and
claim reconciliation remain open.

## 15. Final ship gate

Return a final report with exactly these sections:

### GREEN
Items fully supported by public implementation or reproducible evidence.

### AMBER
Items implemented but missing final release/evidence sealing.

### RED
Any actual blocker that invalidates one of C1, C2, or C3.

### PAPER CLAIMS NOW SAFE TO MAKE
Exact claim wording supported by evidence.

### CLAIMS STILL NOT SAFE TO MAKE
Exact claims that remain unsupported.

### REQUIRED BEFORE SUBMISSION
Only remaining mandatory tasks.

### OPTIONAL POLISH
Non-blocking improvements.

Do not propose architectural expansion unless a RED item cannot be closed without it.

The default assumption is that GlassVM architecture is frozen and the remaining work is evidence production, release sealing, and paper reproducibility.~~~~
