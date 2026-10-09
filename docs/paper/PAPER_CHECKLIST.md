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
- [ ] Keep empirical performance/boundedness claim open until measurements exist.

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
- [ ] Repeat the full 120-trial, three-machine matrix against one clean source coordinate before making a cross-bundle bounded-memory claim.

Original baseline data remains under
[`experiments/measurements/memory-scaling/`](experiments/measurements/memory-scaling/).
The CHIP-8 cleanup follow-up, raw trials, manifest, generated summary, and plots
are under
[`experiments/measurements/chip8-retention-cleanup/`](experiments/measurements/chip8-retention-cleanup/).
The cleanup result is positive for CHIP-8 and this host/workload; the matching
three-machine rerun remains the paper gate.

## 9. Sink-cost decomposition

Measure the same prepared workload under:

- [ ] execution / observation disabled
- [ ] observation + `NullSink`
- [ ] observation + Python consumer
- [ ] observation + bounded Rust recorder

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

Nice-to-have but recommended before paper freeze.

Using the same fixture, `PreparedRun`, and `PreparedObservation`:

- [ ] execute through canonical Rust recorder path
- [ ] execute through Python consumer/provider path
- [ ] compare canonical logical records per channel
- [ ] preserve per-channel order
- [ ] do not invent global cross-channel ordering
- [ ] compare receipts and capability outputs
- [ ] do not compare raw `.gvmseg` bytes as semantic identity

Output:
- deterministic equivalence report
- failing channel + first mismatch if non-equivalent

## 11. Package-index-only install proof

Package-index Python execution in a fresh `uv` environment was exercised by the
135 Slice 5 trials using
`uv run --with glassvm==0.1.2 --with glassvm-chip8==0.1.2 --with
glassvm-pico8==0.1.2 --with glassvm-tic80==0.1.2`; the raw trials record the
installed distribution versions and entry-point provider metadata. The cartridge
bytes were read from the documented fixture checkout and passed as caller-owned
artifact data; no bundle implementation was loaded from that checkout. This is
not yet the separate zero-checkout consumer-project proof: the runner and
fixture manifest came from the GlassVM checkout. The Rust external-consumer
build and fully checkout-free packaging proof below remain open.

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

For Rust:

- [ ] verify released crates resolve without local path overrides
- [ ] verify minimal external consumer project builds against published versions

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

Keep this deliberately small.

- [ ] create one generic caller consuming selected normalized projections and/or capability outputs
- [ ] run it against all three release machines without bundle-specific orchestration branches
- [ ] demonstrate useful analysis without whole-run GlassVM materialization
- [ ] do not reopen Refinery architecture
- [ ] do not add new GlassVM abstractions for this demo

Output:
- small reproducible script
- one result table/figure
- short description of what the generic downstream consumer obtains

## 14. Paper regeneration

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

- [ ] every quantitative manuscript claim maps to a versioned result file
- [ ] every figure is generated from checked-in/public raw data
- [ ] every table is generated rather than manually transcribed
- [ ] scripts record source commit SHAs
- [ ] scripts record machine/toolchain metadata
- [ ] no result depends on private paths or unavailable fixtures

Output:
- one command that regenerates all paper artifacts

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
