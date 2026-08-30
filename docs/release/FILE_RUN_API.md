# File-backed run API

`glassvm_recorder::FileRunSession` is the caller-facing orchestration helper
for publishing one prepared execution. It consolidates lifecycle wiring; it
does not add a queue, worker, retry policy, retention policy, or evidence
semantics.

## Lifecycle

```text
FileRunSession::run
  -> FileEmissionSink
  -> BudgetedSink
  -> EmulatorSession::execute
  -> emit EvidenceReceipt
  -> finalize recorder
  -> atomic publication
```

The helper owns sink construction, budget accounting, execution, evidence
receipt emission, recorder finalization, and the publication gate. Bundle
providers own machine preparation and session construction. They do not
rebuild this lifecycle.

## Result separation

The result exposes four independent domains:

```rust
pub struct FileRunResult {
    pub execution: RunResult,
    pub evidence: EvidenceReceipt,
    pub recorder: RecorderReceipt,
    pub published: PublishedFileRun,
}
```

- `execution` describes machine execution;
- `evidence` describes fulfillment of the requested observation;
- `recorder` describes physical persistence and finalization; and
- `published` identifies the atomically published run.

The published reference exists only when the finalized recorder receipt passes
the publication gate. Evidence failure does not redefine machine execution,
and recorder failure does not become a successful publication.

## Boundedness and recovery

The sink path emits records incrementally. Block and segment limits are hard
buffering limits, not rotation suggestions. A record larger than the block
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
