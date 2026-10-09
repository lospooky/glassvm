# File-backed run API

`glassvm_recorder::FileRunSession` is the caller-facing orchestration helper
for publishing one prepared execution. It consolidates lifecycle wiring; it
does not add a queue, worker, retry policy, retention policy, or evidence
semantics.

## Lifecycle

```text
FileRunSession::run_with_sinks
  -> optional FileEmissionSink + PythonBatchingSink
  -> BudgetedSink
  -> EmulatorSession::execute
  -> emit EvidenceReceipt
  -> optional recorder finalization
  -> atomic publication when a recorder was requested
```

The helper owns sink construction, budget accounting, execution, evidence
receipt emission, recorder finalization, and the publication gate. Bundle
providers own machine preparation and session construction. They do not
rebuild this lifecycle.

## Result separation

The result exposes independent execution, evidence-delivery, and publication
domains, including partial outcomes:

```rust
pub struct FileRunResult {
    pub machine_execution: MachineExecutionOutcome,
    pub evidence_fulfillment: EvidenceReceipt,
    pub python_sink_delivery: PythonSinkDeliveryOutcome,
    pub recorder_finalization: RecorderFinalizationOutcome,
    pub publication: PublicationOutcome,
    pub published: Option<PublishedFileRun>,
}
```

- `machine_execution` distinguishes completion, machine failure, and
  interruption caused by evidence delivery;
- `evidence_fulfillment` is the retrospective observation receipt;
- `python_sink_delivery` describes only Python consumer delivery, or is
  `not_requested`;
- `recorder_finalization` is `not_requested` when no file recorder is selected,
  otherwise it reports whether physical persistence finalized; and
- `publication` reports the atomic publication outcome.

The published handle is present only after the finalized recorder receipt
passes the publication gate. Partial execution results preserve whatever
evidence receipt was accumulated, report recorder finalization as not
attempted (or `not_requested` when no recorder was selected), and have no
published handle. Evidence failure does not redefine
machine execution, Python delivery failure is not recorder failure, and
recorder failure does not become successful publication.

The recorder receipt includes fixed-set, per-channel counters for persisted
segments, records, blocks, logical bytes, and encoded block-payload bytes. Its
channel list is in canonical channel order and is bounded by the recorder's six
declared channels. `encoded_bytes` excludes segment and block framing. The
receipt schema is `glassvm.recorder_receipt@1.1.0`; aggregate segment, record,
block, logical-byte, and encoded-byte totals are checked against the per-channel
counters before publication.

## Boundedness and recovery

The sink path emits records incrementally. Recorder accounting aggregates each
closed segment into fixed per-channel counters; it does not retain a growing
list of segment footers. Publication validation counts segment files while
streaming directory entries. Block and segment limits are hard buffering
limits, not rotation suggestions. A record larger than the block
limit is rejected; valid records satisfy:

```text
record size <= block hard limit <= segment hard limit
```

The initial file format is crash-inspectable but non-resumable. An interrupted
staging run can be inspected, cleaned, and abandoned; a caller restarts the
run rather than attempting to resume partial codec or receipt state.

## Ownership boundary

The recorder persists typed channel records and operational metadata. It does
not define Refinery scoring, reconstruct a whole-run query object, or turn the
execution emission contract into a replay package. Query consumers select and
decode finite projections explicitly when retrospective analysis is desired.
