# Slice 4: experiment harness and result contract

Execution date: 2026-10-08. Status: **complete for the pilot gate; full measurement matrix remains open**.

## Deliverables

- [`experiments/OBSERVATION_RECIPES.toml`](experiments/OBSERVATION_RECIPES.toml): nine explicit recipes covering disabled, lifecycle/minimal, selected/all normalized, selected native, hash/full frame, visual capability, and mixed evidence.
- [`experiments/run_trial.py`](experiments/run_trial.py): one fresh process per measurement, using installed entry-point providers and the public `Runtime.discover -> prepare -> execute` path.
- [`experiments/run_pilot.py`](experiments/run_pilot.py): isolated-process orchestration and result accounting checks.
- [`experiments/result.schema.json`](experiments/result.schema.json): versioned single-trial result envelope, preserving execution, evidence, recorder, and publication as distinct outcomes.
- [`experiments/generate_pilot_summary.py`](experiments/generate_pilot_summary.py): deterministic pilot-summary regeneration.
- [`experiments/pilot/`](experiments/pilot/): 27 raw single-run results and generated [summary](experiments/pilot/PILOT_SUMMARY.md).

## Pilot scope and result

The initial pilot ran one 8-frame trial for each of nine recipes on each of
CHIP-8, PICO-8, and TIC-80 using the published `0.1.1` distributions. It proved
provider wiring and publication but could not report persisted per-channel
counters because those wheels used recorder receipt schema `1.0.0`. GlassVM and
all three machine distributions have now been released at `0.1.2`; the machine
wheels resolve recorder `0.1.2`, which emits receipt schema `1.1.0`. The 27-row
result set below was regenerated from those installed packages. These remain
short contract-wiring checks, not performance samples or bounded-memory evidence.

The runner fixes artifact hash, structured configuration/seed, empty input
schedule, and execution-control shape across recipes for a given machine. It
records exact submitted requests, channel receipt values, machine result,
recorder receipt, physical directory size, discovery/preparation/execute and
process timings, process peak RSS, host/toolchain/package metadata, and source
revision/dirty state. Run directories are temporary; no segment record is
decoded or retained. Physical file bytes are measured independently from
logical and encoded receipt bytes.

## Exit-gate status

1. **Recorder counters:** the regenerated `0.1.2` pilot requires recorder
   receipt schema `1.1.0` and checks per-channel segment, record, block,
   logical-byte, and encoded-byte totals against the receipt aggregates. It does
   not derive them from `EvidenceReceipt` or parse storage segments in Python.
2. **Prepared execution identity:** resolved by decision. The Python
   `PreparedRun` remains opaque; no identity accessor or reference is needed
   for these experiments. Each trial records the artifact digest, explicit
   configuration, schedule, execution controls, provider/package metadata,
   exact observation request, and prepared-observation identity. These are
   reproducibility inputs, not a replacement serialization of `PreparedRun`.

The recorder-counter validation gate is complete. The GlassVM crates/facade and
three machine wheels at `0.1.2` are published.

## Validation

- The isolated 27-cell pilot completed all recipes across all three providers
  using published GlassVM and machine packages at `0.1.2`.
- All 27 raw results identify recorder receipt schema `1.1.0`, expose channel
  statistics, and pass per-channel/aggregate consistency checks.
- Nine Python unit tests pass for typed configuration encoding, frame/event
  independence, extension event tagging, and versioned outcome separation.
- `run_pilot.py` requires receipt schema `1.1.0` and rejects missing or
  internally inconsistent per-channel/aggregate counters.
- The recorder source's `1.1.0` bounded counter tests were already run as part
  of the preceding recorder-counter change; no Rust source changed in Slice 4.

The measured full-frame files demonstrate why physical bytes are reported
separately: TIC-80's eight full frame artifacts occupied substantially more
physical encoded storage than hash-only capture. No general storage or runtime
claim is inferred from this tiny pilot.
