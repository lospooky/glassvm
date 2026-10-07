# Slice 4: experiment harness and result contract

Execution date: 2026-10-07. Status: **implementation and pilot harness green; recorder-counter end-to-end validation remains open**.

## Deliverables

- [`experiments/OBSERVATION_RECIPES.toml`](experiments/OBSERVATION_RECIPES.toml): nine explicit recipes covering disabled, lifecycle/minimal, selected/all normalized, selected native, hash/full frame, visual capability, and mixed evidence.
- [`experiments/run_trial.py`](experiments/run_trial.py): one fresh process per measurement, using installed entry-point providers and the public `Runtime.discover -> prepare -> execute` path.
- [`experiments/run_pilot.py`](experiments/run_pilot.py): isolated-process orchestration and result accounting checks.
- [`experiments/result.schema.json`](experiments/result.schema.json): versioned single-trial result envelope, preserving execution, evidence, recorder, and publication as distinct outcomes.
- [`experiments/generate_pilot_summary.py`](experiments/generate_pilot_summary.py): deterministic pilot-summary regeneration.
- [`experiments/pilot/`](experiments/pilot/): 27 raw single-run results and generated [summary](experiments/pilot/PILOT_SUMMARY.md).

## Pilot scope and result

The pilot ran one 8-frame trial for each of nine recipes on each of CHIP-8,
PICO-8, and TIC-80 using isolated Python 3.12 environments with the published
GlassVM and bundle distributions at version `0.1.1`. All 27 preparations
succeeded; every run reported complete evidence, complete/finalized recorder
status, and successful publication. CHIP-8 reached its configured frame bound
with `Timeout`; PICO-8 and TIC-80 reported `frame_limit`. These are short
contract-wiring checks, not performance samples or bounded-memory evidence.

The runner fixes artifact hash, structured configuration/seed, empty input
schedule, and execution-control shape across recipes for a given machine. It
records exact submitted requests, channel receipt values, machine result,
recorder receipt, physical directory size, discovery/preparation/execute and
process timings, process peak RSS, host/toolchain/package metadata, and source
revision/dirty state. Run directories are temporary; no segment record is
decoded or retained. Physical file bytes are measured independently from
logical and encoded receipt bytes.

## Exit-gate status

1. **Recorder counters:** the public `0.1.1` wheels use recorder receipt schema
   `1.0.0`, which has aggregate bytes/segments but no per-channel record,
   block, or logical/encoded-byte counters. The current local recorder source
   adds those fields under receipt schema `1.1.0`; its Rust accounting tests
   are separate from this wheel-based pilot. Therefore the summary explicitly
   marks persisted per-channel counters unavailable. It does not derive them
   from `EvidenceReceipt` or parse storage segments in Python.
2. **Prepared execution identity:** resolved by decision. The Python
   `PreparedRun` remains opaque; no identity accessor or reference is needed
   for these experiments. Each trial records the artifact digest, explicit
   configuration, schedule, execution controls, provider/package metadata,
   exact observation request, and prepared-observation identity. These are
   reproducibility inputs, not a replacement serialization of `PreparedRun`.

Only the recorder-counter validation requires further work. No packages were
published as part of this slice.

## Validation

- The isolated 27-cell pilot completed all recipes across all three providers.
- JSON Schema validation passed for all 27 raw result files.
- Five Python unit tests pass for typed configuration encoding, frame/event
  independence, extension event tagging, and versioned outcome separation.
- `run_pilot.py` rejects internally inconsistent per-channel/aggregate
  counters whenever the installed receipt schema provides those counters.
- The recorder source's `1.1.0` bounded counter tests were already run as part
  of the preceding recorder-counter change; no Rust source changed in Slice 4.

The measured full-frame files demonstrate why physical bytes are reported
separately: TIC-80's eight full frame artifacts occupied substantially more
physical encoded storage than hash-only capture. No general storage or runtime
claim is inferred from this tiny pilot.
