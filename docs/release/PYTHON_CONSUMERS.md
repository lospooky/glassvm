# Python emission consumers

`PreparedRun.execute` can synchronously deliver requested GlassVM emissions to
one or more caller-owned Python consumers, the Rust reference file recorder,
both, or neither:

```python
result = prepared.execute(
    sinks=[consumer_a, consumer_b],
    file_recorder=glassvm.FileRecorder("runs/example"),
)
```

A consumer follows this structural protocol:

```python
class EmissionSink(Protocol):
    def emit_batch(self, emissions: Sequence[Emission]) -> None: ...
    def record_capability_outputs(
        self, outputs: Sequence[CapabilityOutput]
    ) -> None: ...
    def record_capability_receipts(
        self, receipts: Sequence[CapabilityReceipt]
    ) -> None: ...
```

Every consumer must provide all three callable methods. Emission batches contain
owned Python values; no Rust borrow crosses the boundary. Batch order is
callback-delivery order only and does not define a cross-channel event order.
Capability outputs and capability receipts are delivered separately.

The bridge is synchronous and applies backpressure while callbacks run. There
is no worker, asynchronous queue, retry, or detach-and-continue behavior. Rust
accumulates one bounded batch, currently targeting at most 32 items and 256 KiB
of logical record bytes. Each compact JSON record encoding is reused for both
batch-size accounting and Python decoding. These transport bytes are not
canonical identity material or the reference recorder format. A valid single
record larger than the byte target is delivered alone. These initial values
are implementation defaults, not frozen tuning guarantees; benchmark them
before treating them as API commitments.

Callback exceptions are captured as the separate `python_sink_delivery`
outcome, with sink/callback context and partial delivery counts. They are not
recast as machine execution failure or recorder corruption. Machine execution,
evidence fulfillment, Python sink delivery, recorder finalization, and
publication remain separately inspectable in the returned mapping. A
`published_run` reference is present only when recorder finalization and the
atomic publication gate succeed.

The optional `FileRecorder` is only a request value for the provider's shared
`FileRunSession` lifecycle. Python does not implement or parse the GlassVM
reference file format.
