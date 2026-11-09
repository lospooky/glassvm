# E1 F2: optional capability with a missing prerequisite

Three fresh Python processes per machine requested the bundle's optional
`input_summary` capability while leaving input-value evidence unselected.
All nine preparations succeeded, executions completed, and the evidence
receipt reported the capability unavailable with the reason that the
input-value evidence channel was not selected. Recorders finalized and
published in all nine runs. Temporary recordings were removed after results
were captured.

The Python prepared-run facade is opaque, so these trials do not directly
inspect `PreparedObservation`. The separate Rust three-bundle conformance test
`required_missing_prerequisites_fail_and_optional_roots_downgrade` checks the
prospective unavailable status and reason for this prerequisite.

Raw records and manifest include requests, results, source revisions and
local-wheel hashes. Reproduce with `run_e1_f2.py` using the locally built
repair wheels and a fresh output directory.
