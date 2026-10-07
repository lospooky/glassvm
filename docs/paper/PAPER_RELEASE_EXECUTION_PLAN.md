# GlassVM paper release execution plan

Planning date: 2026-10-04.

This plan breaks down the remaining work in [PAPER_CHECKLIST.md](PAPER_CHECKLIST.md)
into executable slices. The checklist remains the acceptance contract, and
[CLAIM_EVIDENCE_MAP.md](CLAIM_EVIDENCE_MAP.md) remains the claim-to-evidence map.
An unchecked checklist item is not proof that an implementation is missing.
Close items with inspectable evidence rather than declarations of completion.

## Release scope and stopping rule

The release denominator is CHIP-8, PICO-8, and TIC-80. CHIP-8 is the reference
implementation; PICO-8 and TIC-80 are external machine validation specimens.
Hexwell and Wyrd-16 remain on their parked branch and outside this release.

Freeze the architecture around the approved contracts. Freeze machine features
once the selected paper workloads run correctly through those contracts.
Full TIC-80 or PICO-8 compatibility is not the release gate. Describe the tested
subsets precisely, including Lua runtime differences and unsupported features.
Add implementation work only for a concrete workload, conformance, or
claim-breaking defect.

The workload denominator is now frozen as one publicly sourced, zero-input
cartridge per machine. The exact artifacts, pins, configurations, selected
events/capabilities, and 600-frame gate are recorded in
[PAPER_WORKLOADS.toml](PAPER_WORKLOADS.toml). The PICO-8 specimen is explicitly
a title-normalized derivative, not an unmodified upstream run. The selected
TIC-80 graphics work remains in scope: native palette operations, palette-only
BDR, and default 4-bpp affine textured triangles; nonzero depth remains
unsupported.
Paper artifacts are kept uniformly under each bundle's `fixtures/paper/`,
with native extensions and upstream filenames; unmodified inputs remain under
`fixtures/source/public/`.

The TIC-80 palette, palette-only BDR, and default 4-bpp affine triangle work
has been reference-reviewed locally and retained in the evaluated subset.
Nonzero depth remains unsupported. Depth/perspective and additional graphics
features are not automatic next slices.

## Planning snapshot

The following records the state observed during the checklist review. Refresh
these facts when executing the relevant slice; these are not final artifact
coordinates.

| Area | Observed state |
| --- | --- |
| Public repositories | Both `lospooky/glassvm` and `lospooky/glassvm-machines` are public. |
| GlassVM local/public head | `cf7dee8444e1f5968f7206f65335663855c981e2`. |
| Machine repository public/local head | `56563bd0dfb92a67d272ed4d8ab4868e448b265d`. |
| Pending machine changes | None; source branch is clean and synchronized with `origin/main`. |
| Published CI | [GlassVM run 37216452075](https://github.com/lospooky/glassvm/actions/runs/37216452075) and [Machines run 37216276015](https://github.com/lospooky/glassvm-machines/actions/runs/37216276015) passed; Rust/conformance and CPython 3.12, 3.13, 3.14. Machine CI rebuilt wheels from sdists and exercised base/per-bundle/all-bundle installs. |
| Maturity inventory | Twelve implementation/conformance dimensions are verified per release machine; packaging/provenance and paper evidence remain open per machine. |
| Python package indexes | The selected facade distribution/import is `glassvm`; its PyPI lookup returned HTTP 404 on 2026-10-07. The selected bundle names are `glassvm-chip8`, `glassvm-pico8`, and `glassvm-tic80`; lookups returned 404 for chip8/tic80, while pico8 timed out and needs a retry. The prior 2026-10-04 lookup checked `glassvm-py` and the former `glassvm-machine-*` names; those returned HTTP 404. Configure Trusted Publishers against the selected names. The five GlassVM contract crates are published at `0.1.0`; machine Rust packages are not targets. |
| Source archive audit | All four Python sdists rebuilt to wheels locally on CPython 3.14; all-provider clean-room smoke passed. Empty-cache builds remain open. |
| Paper tooling | No experiment harness or regeneration pipeline was found in the split repositories. |
| Documentation | Release-candidate status and the claim/evidence map have been reconciled; paper audit documents remain local and uncommitted. |

The Slice 1 source and clean-room matrix are now validated at the public
coordinates above. The paper checklist/plan and workload manifest remain
uncommitted for later review; they were intentionally excluded from machine
source commits.

## Execution sequence

| Slice | Outcome | Prerequisite | Checklist sections |
| --- | --- | --- | --- |
| 1 | Reviewed implementation, selected workloads, published green source coordinates | None | 1–3 |
| 2 | Release audit, conformance matrix, reconciled claim map | 1 | 1–5 |
| 3 | Generated machine heterogeneity table | 2 | 6 |
| 4 | Reproducible measurement harness and result format | 1–3 | 7, 14 |
| 5 | Selective-observation measurements | 4 | 7 |
| 6 | Run-length bounded-memory evidence | 4; use 5 to calibrate practical lengths | 8 |
| 7 | Consumer costs, semantic equivalence, generic downstream demo | 4–6; Python-consumer scope resolved | 9, 10, 13 |
| 8 | Released packages and package-index-only installation proof | 2; supported release coordinates settled | 11 |
| 9 | Regenerated paper artifacts, immutable coordinates, final claim report | 3, 5–8 | 12, 14, 15 |

Slice 8 packaging preparation may run alongside experiments after source and
version choices are stable. Final publication must use the evaluated source.
Build the regeneration scaffold in slice 4; complete it with the real results
in slice 9.

At each slice boundary, record changed files, relevant checks, evidence paths,
remaining decisions, and logical commits. Preserve unrelated worktree changes.
Do not mark later slices complete on the strength of an earlier smoke test.

## Slice 1: freeze the evaluated implementation and workload scope

Execution record: [PAPER_RELEASE_SLICE_01.md](PAPER_RELEASE_SLICE_01.md).
Completed 2026-10-04. Workload and graphics-scope decisions are resolved.
Targeted reference regressions, the 600-frame three-machine workload gate, the
full workspace test suite, formatting, fixture checksums, changed-package
Clippy, and public exact-source CI are green. Workspace-wide Clippy still stops
at a pre-existing CHIP-8 verifier lint; see the execution record.

Machine changes were committed as `8486fc7` (`Correct TIC-80 graphics
behavior`) and `66860ca` (`Add pinned paper workloads`), then pushed along with
the two earlier continuation/retention commits. The published machine SHA is
`66860cad9db978d6399f4f1a7a0e016e61c82b92`. The paper planning/checklist
documents remain uncommitted for review.

### Tasks

- [x] Inventory the committed and uncommitted differences from public `main` in both repositories; see the source inventory in the execution record.
- [x] Review the pending TIC-80 APIs against the reference, including texture addressing, callback timing, clipping, and unsupported paths. Scope limits are recorded in the execution record.
- [x] Define the supported machine subsets and finite public workload set, including fixture provenance, configuration, empty input schedules, and intended execution length; see `PAPER_WORKLOADS.toml`.
- [x] Confirm each workload requests its visual capability's declared normalized-event prerequisites, emits changing frame hashes, fulfills one capability output/receipt, and reaches 600 frames.
- [x] Resolve workload-blocking defects and record remaining unsupported features accurately; no additional feature work is authorized by this gate.
- [x] Commit the reviewed machine source changes in logical units (`8486fc7`, `66860ca`); keep the paper checklist/plan documents out of these commits for later review.
- [x] Push the four machine commits, including PICO-8 continuation and TIC-80 retention fixes, and run CI on the exact source coordinate.
- [x] Recheck clean-clone Rust/conformance and CPython 3.10/3.13 base, individual-provider, and all-provider jobs for those coordinates.

### Deliverables and exit gate

Produce a workload manifest and a supported-subset statement for all three
machines, plus exact source SHAs and passing CI links. Every evaluated behavior
must exist in the published source. Additional machine features require a
specific acceptance need.

Exit evidence: machine source `66860cad9db978d6399f4f1a7a0e016e61c82b92`,
GlassVM facade source `5aa7d2a60210f173b344958e1338e6f8500c2a81`, and passing
[CI run 37214867564](https://github.com/lospooky/glassvm-machines/actions/runs/37214867564).

Workload decision resolved: one suitable public no-input cartridge per release
machine. Keep the PICO-8 title normalization explicit in all generated claims;
do not present that artifact as byte-identical upstream evidence.

## Slice 2: reconcile release and conformance evidence

Execution record: [PAPER_RELEASE_SLICE_02.md](PAPER_RELEASE_SLICE_02.md).
Completed 2026-10-04 for source audit, conformance mapping, licensing,
dependency/source-closure, and public CI. PyPI publication and empty-cache
release verification remain open and are not represented as completed.

### Tasks

- [x] Audit repository boundaries and release-facing descriptions against the three-machine denominator.
- [x] Audit Cargo/Python license metadata and root licenses for BSD-3-Clause consistency; preserve third-party fixture provenance.
- [x] Inventory intended Rust crate and Python distribution release targets and inspect their dependency/source closure.
- [x] Distinguish valid sibling source paths inside a complete sdist from path-only dependencies that cannot be published as Rust crates.
- [x] Run both independent prohibited-surface checks and inspect release-facing manifests/docs that those source checks do not scan. Classify historical hits separately.
- [x] Assemble the CHIP-8/PICO-8/TIC-80 conformance matrix linking each checklist section 3 property to a concrete test or implementation anchor: [PAPER_CONFORMANCE_MATRIX.md](PAPER_CONFORMANCE_MATRIX.md).
- [x] Verify required rejection, optional downgrade, exact prerequisite closure, and typed configuration/input admission in every released bundle.
- [x] Verify separate execution/evidence/recorder/publication outcomes, bounded normalization, state-only continuation, atomic publication, and interrupted staging behavior.
- [x] Update stale CI/public-repository claims and reconcile C1/C2 evidence without marking C3 measurements complete.
- [x] Reconcile the maturity inventory with actual evidence; preserve any packaging or paper gates still open.

### Deliverables and exit gate

Produce the conformance matrix, release-audit findings with dispositions, and
an updated claim map. Close implementation items only when the specific
behavior is evidenced; every unresolved claim-breaking finding must name a
repair and verification task.

Resolved release choices: publish the five machine-neutral GlassVM contract
crates only; keep machine-specific Rust crates `publish = false`; advertise a
Python minimum of 3.12; test CPython 3.12, 3.13, and 3.14. The tested platform
claim remains Ubuntu 24.04 x86_64 only. The four Python distributions are still
unpublished, and empty-cache source publication has not been verified.

## Slice 3: generate the heterogeneity table

Execution record: [PAPER_RELEASE_SLICE_03.md](PAPER_RELEASE_SLICE_03.md).
Completed 2026-10-07 for the three-machine comparison and deterministic
Markdown/LaTeX generation. The table is pinned to the current public source
revisions; it establishes contract heterogeneity and the common caller
lifecycle, not compatibility breadth or empirical performance.

### Tasks

- [x] Extract artifact representation, execution/configuration/input models, and timing/coordinate semantics.
- [x] Extract frame schemas, normalized/native evidence families, snapshot support, capabilities, static services, and Python provider metadata.
- [x] Explain the material machine differences and the shared caller lifecycle using the declarations and supported subsets.
- [x] Preserve the Lua compatibility-runtime qualification for PICO-8 and TIC-80; avoid instruction-exact or full-console claims.
- [x] Save the comparison as machine-readable data and generate Markdown and LaTeX-ready tables from it.

### Deliverables and exit gate

One source data file and generated tables cover every checklist section 6
dimension. Each entry links to a declaration or supported-subset document at
the evaluated source revision. Re-running generation produces identical
output. See the Slice 3 execution record for validation and limitations.

## Slice 4: build the experiment harness and result contract

Measurement boundary decision: the Python harness orchestrates runs through
the installed provider API; Rust `FileRunSession` continues to own
`BudgetedSink -> FileEmissionSink`. Recorder schema `1.1.0` now exposes bounded
per-channel segment, record, block, logical-byte, and encoded-block-payload
counters in `RecorderReceipt`. These counters aggregate closed segment footers
into the fixed six-channel set rather than retaining a run-length-sized footer
collection. The harness should consume these receipt counters instead of
introducing a parallel Python sink. `encoded_bytes` excludes segment/block
framing, and remains distinct from physical file size.

Implementation record: the Slice 4 harness, nine explicit observation recipes,
versioned single-trial result schema, and 27-cell eight-frame pilot now exist in
[`experiments/`](experiments/), with regeneration instructions in its README.
The initial pilot used published `0.1.1` wheels and could not observe
per-channel recorder counters. GlassVM `0.1.2` publishes recorder receipt schema
`1.1.0`; the machine wheels were rebuilt against those crates, and the pilot is
being regenerated from installed `0.1.2` distributions. The harness now requires
schema `1.1.0` and checks per-channel totals against recorder aggregates, without
inferring persistence counters from `EvidenceReceipt` or reading stored records.
The identity-reporting requirement is resolved without changing the opaque
Python facade: trials record the artifact digest, explicit configuration,
schedule, execution controls, provider/package metadata, and
`PreparedObservation` identity. The experiment does not need to expose the
internal `PreparedRun` identity object or add an identity-reference API.

### Tasks

- [x] Create experiment/results locations, a versioned raw-result contract, and regeneration entry points for trial data and the pilot summary.
- [x] Implement one shared execution driver that submits typed JSON requests through the installed-provider preparation and file-backed execution lifecycle.
- [x] Define recipes for disabled, minimal, selected/all normalized, selected native, hash/full frames, visual capability, and mixed observation.
- [x] Resolve all nine recipes on CHIP-8, PICO-8, and TIC-80 in the pilot; no unsupported recipe cells occurred. Missing provider cells are represented as explicit exclusions.
- [x] Fix artifact/configuration/input/seed/control material across observation comparisons and record reproducibility metadata plus negotiated observation identity. Keep `PreparedRun` opaque; no identity accessor is required.
- [x] Separate provider discovery, preparation, execute-and-publish, worker-process, and process-lifecycle timing. Pure emulator-only time is not exposed and is not inferred.
- [x] Define raw fields for evidence/recorder counters, outcomes, runtime, peak RSS, and optional allocator high-water; unavailable counters remain explicitly null/unavailable.
- [x] Launch each trial in an independent Python 3.14 process and record repetition policy, host/toolchain/package versions, provider metadata, and both source revisions/dirty state.
- [x] Keep the measurement driver bounded: it retains receipts and metadata only, scans file sizes incrementally, and never reads segment records.
- [ ] Regenerate the 27-cell pilot from published `0.1.2` distributions and validate per-channel recorder accounting under schema `1.1.0`.
- [x] Run the small 27-cell pilot and validate request preparation, separate outcomes, receipt completion, publication, schema shape, and evidence/physical-byte accounting boundaries.

### Deliverables and exit gate

A runnable harness, workload/observation manifests, a versioned result schema,
pilot results, and a regeneration scaffold are present. The prepared-run
identity boundary is resolved by retaining the opaque facade and recording
execution inputs directly. Slice 4 remains open until the regenerated pilot
with recorder receipt `1.1.0` validates persisted per-channel accounting. Every
reported pilot metric has defined units and a measurement source; missing
fields are explicit rather than reconstructed from another channel.

Fixture/configuration fingerprints are reproducibility metadata. They do not
introduce an authenticity, signature, or cryptographic audit protocol.

## Slice 5: collect selectable-observation measurements

### Tasks

- [ ] Execute each supported observation recipe for each selected machine workload.
- [ ] Use the same resolved machine execution material across mode comparisons and verify actual completed frames/steps.
- [ ] Collect repetitions using the recorded policy and retain the individual observations.
- [ ] Record item/byte accounting, storage layout counts, publication outcome, and evidence/recorder receipts for every run.
- [ ] Check that evidence budgets and recorder limits are explicit; distinguish intentionally incomplete evidence from successful complete runs.
- [ ] Produce a normalized result table and identify anomalies requiring investigation before interpreting overhead.

### Deliverables and exit gate

Raw result files and a generated observation-cost table. Every row names the
workload, request, source coordinates, actual completed work, and outcome.
Do not infer bounded memory from these fixed-length comparisons.

## Slice 6: establish run-length bounded-memory evidence

### Tasks

- [ ] Select several geometric lengths, targeting 1x/10x/100x/1000x where practical, and record any resource-based exclusions.
- [ ] For each workload, run disabled observation, observation with NullSink, bounded normalizers, and the durable recorder.
- [ ] Keep machine parameters and evidence recipes fixed across the length sweep; extend only the declared stopping bound/input schedule as needed and record those changes.
- [ ] Measure peak RSS, runtime, logical volume, durable volume, and allocation high-water if available in separate processes.
- [ ] Verify each run reaches the intended length, and record evidence truncation or resource failures explicitly.
- [ ] Investigate whether observed memory growth comes from machine state, Lua/runtime allocation, reducer state, sink buffering, or the measurement harness.
- [ ] Generate memory-versus-length and emitted/stored-volume-versus-length plots.
- [ ] Write C3 claim language constrained to the measured workloads, hosts, lengths, and configured bounds.

### Deliverables and exit gate

Raw measurements, two generated plots, and a supported boundedness statement.
If memory grows with length, expose and diagnose the growth before claiming
bounded pipeline behavior. Growing durable output is reported independently.
This is the central remaining empirical gate for C3.

## Slice 7: consumer costs, equivalence, and downstream use

### 7a. Rust sink-cost decomposition

- [ ] Compare disabled observation, the same observation with NullSink, and the same observation with the bounded recorder.
- [ ] Preserve the execution material and use explicit requests so differences isolate the intended costs.
- [ ] Report absolute runtime, relative overhead, and the limits of the decomposition.

### 7b. Resolve the Python-consumer experiment

The current Python facade exposes preparation and file-backed execution. It
does not expose a live Python emission-consumer API. Provider timing therefore
measures Python orchestration and the shared recorder lifecycle, not an isolated
Python sink cost.

- [ ] Decide whether to defer the live Python-consumer row with a corresponding checklist/claim adjustment, or approve a separate narrowly scoped binding task.
- [ ] If deferred, label any provider timing accurately and report the exclusion.
- [ ] If a live consumer is required, implement and verify that boundary as a separate slice before measuring it; this plan does not silently authorize a new streaming subsystem.

### 7c. Rust/Python semantic equivalence

- [ ] Execute equivalent canonical requests through the Rust recorder and installed Python provider paths.
- [ ] Check matching execution and prepared-observation identities; keep provider-owned prepared objects opaque.
- [ ] Select/decode channel records using the recorder reader, then compare canonical logical values and per-channel order.
- [ ] Compare capability outputs and receipts; explicitly define comparison treatment for operational metadata.
- [ ] Report the first mismatching channel/record if equivalence fails. Do not compare raw segment bytes or invent a global event ordering.

### 7d. Generic downstream demonstration

- [ ] Implement a small caller consuming selected normalized evidence or capability outputs through the shared interface.
- [ ] Run the same orchestration against all three release bundles.
- [ ] Use incremental analysis or an explicitly finite selected projection and generate one useful result table or figure.

### Deliverables and exit gate

A cost table with an honest Python boundary, an equivalence report, and a
reproducible downstream demo. The equivalence check is recommended by checklist
section 10; its omission must remain explicit rather than being marked complete.

## Slice 8: publish packages and prove independent installation

### Tasks

- [ ] Reconcile package versions with the reviewed/evaluated source and already published registry coordinates.
- [ ] Verify the source closure of each intended sdist and release crate using isolated builds with fresh dependency caches.
- [ ] Build the declared wheel/platform set and verify each bundle distribution supplies exactly one installed entry-point provider.
- [ ] Publish shared GlassVM dependencies before the machine packages that consume them; keep extras as ordinary package dependencies.
- [ ] Install the facade/providers from the package index in environments with no repository checkout or local overrides.
- [ ] Supply public fixtures independently and verify base-only, each provider alone, and all providers together through the same caller lifecycle.
- [ ] Record exact package versions, installation commands/logs, entry-point discovery, receipts, and published-run results.
- [ ] Build a minimal external Rust consumer against the released versions without local path overrides.

### Deliverables and exit gate

Package identifiers, release artifacts, and logs proving index-only Rust/Python
use. A local wheel directory is useful earlier evidence but does not close this
gate. Resolve any release authority needed at publication time.

If version or source changes affect machine behavior, preparation, evidence,
or storage accounting, rerun the affected experiments before sealing claims.

## Slice 9: seal and regenerate the paper artifact

### Tasks

- [ ] Complete the regeneration command for all numbers, tables, and figures using public raw result files.
- [ ] Ensure every quantitative claim identifies its result source and every supported-subset/conformance claim identifies its evidence.
- [ ] Record exact evaluated source SHAs, package versions, toolchains, host characteristics, fixtures, and experiment configurations in PAPER_ARTIFACT.md.
- [ ] Make experiment/regeneration scripts detect source-coordinate mismatches and record the provenance of regenerated outputs.
- [ ] Generate the final claim map and reconcile the checklist/maturity inventory with the completed evidence.
- [ ] Verify regeneration without private paths, unavailable fixtures, or hand-transcribed tables.
- [ ] Create immutable tags/releases for the exact evaluated coordinates and use those coordinates in the manuscript.
- [ ] Produce the final ship-gate report using the checklist's required sections.

### Deliverables and exit gate

One regeneration command, public raw results and generated artifacts,
PAPER_ARTIFACT.md, immutable release coordinates, and the final report:

- GREEN
- AMBER
- RED
- PAPER CLAIMS NOW SAFE TO MAKE
- CLAIMS STILL NOT SAFE TO MAKE
- REQUIRED BEFORE SUBMISSION
- OPTIONAL POLISH

Optional polish and broader machine compatibility must not become implicit
submission requirements. Claims remain limited to the verified contracts,
selected specimens, and measured conditions.

## Immediate next work

Slice 2 is complete at the pinned source coordinates. Begin Slice 3: extract
the three bundles' declared and implemented heterogeneity into a machine-
readable source and generate the paper's Markdown/LaTeX-ready comparison.
