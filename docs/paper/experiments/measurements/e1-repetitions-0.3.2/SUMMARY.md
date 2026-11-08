# E1 source-suite repetitions — GlassVM 0.3.2

## Result

Three independent repetitions passed. Each repetition ran the GlassVM
workspace suite, the Python-enabled recorder suite, and the full machine
workspace/conformance suite. That is 500 passing tests per repetition:

| Suite | Tests passed per repetition |
| --- | ---: |
| GlassVM workspace | 129 |
| Recorder with Python feature | 37 |
| Machine workspace, including CHIP-8/PICO-8/TIC-80 conformance | 334 |
| **Total** | **500** |

The suites include the negotiated-prerequisite cases, ordinary Rust and Python
consumer failures, external-kill staging behavior, both evidence overflow
policies, recorder create/write/finalization failures, publication collision,
execution failure, and zero-frame versus truncation accounting. Each assertion
was exercised once in each of the three separate suite invocations.

## Reproduction record

- GlassVM source: `b62b2bd`, branch `paper-publication`.
- Machine source: `a32af92`, branch `paper-publication`.
- Package metadata and external GlassVM crate pins: `0.3.2`.
- Host: WSL2, Linux `6.18.54.1-microsoft-standard-WSL2`, x86_64.
- Rust: `rustc 1.97.1`; Cargo `1.97.1`.
- Python running the harness: `3.10.12`.
- Runner: [`run_e1_repetitions.py`](../run_e1_repetitions.py).
- Raw Cargo output: `r01-*.log` through `r03-*.log`; per-repetition summaries
  and source coordinates: `manifest.json` and `r01.json` through `r03.json`.

The runner invokes `cargo test --workspace --locked --no-fail-fast` for each
repository and separately runs
`cargo test --package glassvm_recorder --features python --locked --no-fail-fast`.
The separate invocation avoids applying PyO3's facade `extension-module` link
mode to ordinary test binaries.

## Scope

This is repeated source-level verification of the fault/accounting assertions
and bundle conformance tests. It is not a new runtime fault-injection campaign
using machine programs, a performance measurement, or evidence about
population-scale behavior. The finalization-failure trigger remains
`#[cfg(test)]` only. Graceful cancellation remains unsupported; the external
process-kill test covers abrupt termination before finalization.

An initial attempt to run the entire workspace with `--all-features` could not
link the Rust test binary because the facade enables PyO3 extension-module
linking. That command was discarded and is not counted. The split test commands
above passed in all three recorded repetitions.
