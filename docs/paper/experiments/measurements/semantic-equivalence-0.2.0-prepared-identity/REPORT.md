# Prepared-run provenance and sink equivalence

Result: pass for CHIP-8, PICO-8, and TIC-80.

Each provider prepared once and executed once. The same `RunStarted` emission
was fanned out to the Python consumer and Rust recorder. The Rust recorder
manifest retained the identity from that event, allowing the harness to
independently compare the Python-owned value with Rust-decoded provenance.

| Machine | Normalized events | Native evidence | Frames | Capability outputs | Capability receipts | Evidence receipt | Machine metrics | PreparedRunIdentity | PreparedObservationIdentity |
| --- | ---: | ---: | ---: | ---: | --- | --- | --- | --- | --- |
| CHIP-8 | 2,568 | 1 | 600 | 1 | match | match | match | match | match |
| PICO-8 | 600 | 1 | 600 | 1 | match | match | match | match | match |
| TIC-80 | 1,200 | 1 | 600 | 1 | match | match | match | match | match |

All runs ended with complete evidence, finalized recorder receipts, and
published-run references. Snapshots and input-value evidence were not selected
by the `mixed` recipe. Records were compared in canonical JSON form, preserving
order within each channel only. Segment/block encodings, filesystem layout,
and cross-channel ordering were excluded.

The matrix used locally rebuilt CPython 3.14 wheels, all distribution versions
`0.2.0`, linked against the modified GlassVM workspace crates. The reference
run manifest schema is now `glassvm.reference_run` 1.1.0 to carry the same
prepared identity provenance as `RunStarted`. This is metadata only; segment
records and channel encodings did not change.

## Provenance and payload note

`PreparedRunIdentity` is forwarded from the object returned by preparation;
the provider does not recompute or serialize its opaque `PreparedRun`. The
existing `PreparedArtifactIdentity.canonical_value` field contains the
submitted artifact bytes, so serializing the full identity into `RunStarted`
and the reference manifest includes those bytes as part of the identity value.
This is a consequence of the current canonical identity type, not a separate
artifact-copying implementation in the Python bridge.

The exact values used in the comparisons are retained in
`equivalence-report.json` and the per-machine Python/Rust projections. The
harness streams Python callbacks to NDJSON and asks the Rust recorder reader to
decode the durable channels; it does not parse `.gvmseg` in Python.

Harness: [`../../run_semantic_equivalence.py`](../../run_semantic_equivalence.py).
