# Paper release Slice 6: three-machine run-length memory rerun

Completed 2026-10-12. The clean, common-source three-machine run-length
matrix is now complete and validated.

## Source and environment

The trials ran from disposable clean clones outside the source worktrees, with
the exact compatible revisions required by the published GlassVM contracts:

| Repository | Revision | Dirty |
|---|---|---|
| GlassVM | `a7dd37498d681cd9c97679ac30c6a22a7acb311e` | no |
| GlassVM Machines | `3edb0ae31a4a3887a1f385d9d7b9aa50f7a9b026` | no |

The run used Rust 1.97.1, Python 3.14.3, and Linux x86_64 under WSL2
(`5.15.133.1-microsoft-standard-WSL2`). The manifest records the Cargo lock
digest, trial-binary digest, fixture digests, configurations, and recorder
limits. The GlassVM source revision is intentionally the publication-compatible
contract coordinate paired with the machine revision; newer uncommitted
paper-branch provenance plumbing was not needed for this execution-memory
measurement.

## Matrix and validation

The seeded-shuffled matrix contains 120 isolated trials:

- CHIP-8, PICO-8, and TIC-80;
- observation disabled, selected events to `NullSink`, selected events plus
  hash frames and a bounded visual normalizer to `NullSink`, and the same
  observation through the reference recorder;
- 600, 6,000, 60,000, and 600,000 frames;
- three repetitions through 60,000 frames and one at 600,000 frames.

All 120 trials completed; zero failed. Every trial reached its requested frame
limit and had a complete `EvidenceReceipt`. All 30 recorder trials finalized
and published. Across repetitions and observation modes, each machine/run
length had matching frame and cycle totals, boot outcome, and termination.
The summarizer validated the full matrix and generated the tables and figures
from the raw trial records.

## Results and claim boundary

Median peak process RSS remained approximately flat between the 600- and
600,000-frame endpoints in each mode:

| Machine | Disabled | Events + NullSink | Visual normalizer + NullSink | Recorder |
|---|---:|---:|---:|---:|
| CHIP-8 | 5.00 → 5.02 MiB | 5.05 → 5.11 MiB | 5.00 → 5.02 MiB | 5.81 → 6.04 MiB |
| PICO-8 | 5.27 → 5.41 MiB | 5.28 → 5.54 MiB | 5.27 → 5.49 MiB | 6.04 → 6.54 MiB |
| TIC-80 | 5.12 → 5.12 MiB | 5.12 → 5.27 MiB | 5.13 → 5.24 MiB | 6.16 → 6.08 MiB |

This supports a carefully bounded empirical statement: for these three pinned
workloads, modes, source revisions, and this WSL2 host, measured peak RSS did
not grow materially with run length over the tested range. It is not a proof
of a universal memory bound, allocator high-water, or cross-host behavior.
The longest point has one repetition; the shorter points have three.

Evidence volume and storage remain different quantities from process memory.
At 600,000 frames, logical evidence reached 1,717.34 MiB for CHIP-8,
498.09 MiB for PICO-8, and 749.54 MiB for TIC-80 in the visual-normalizer mode.
The recorder's encoded channel payloads grew with the emitted evidence; the
corresponding medians were 1,364.33 MiB, 398.07 MiB, and 596.83 MiB. This is
expected persistence cost, not a claim that durable output is bounded
independently of run length. Allocator high-water was unavailable; peak RSS is
Linux `/proc/self/status` `VmHWM` and includes process startup.

## Reproduction and retained artifacts

The runner is `experiments/run_memory_experiment.py`; the summarizer is
`experiments/summarize_memory_experiment.py`. The run required clean checkouts
and pinned both expected source revisions. Results were first written outside
the checkouts, then copied into
[`experiments/measurements/memory-scaling-three-machine-a7dd374-3edb0ae/`](experiments/measurements/memory-scaling-three-machine-a7dd374-3edb0ae/).
That directory contains 120 raw trial JSON files, `experiment.json`,
`summary.json`, `SUMMARY.md`, and both generated SVG figures.
Only trailing whitespace in the SVG files was stripped for clean diffs; trial
data and generated plot content are unchanged.
