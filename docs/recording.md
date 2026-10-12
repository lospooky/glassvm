# Recording reference runs

`glassvm_recorder` provides a file-backed recorder for callers that need to
retain selected evidence. It writes evidence incrementally into channel
segments and publishes a run only after finalization succeeds.

## Run directory

A published run contains a manifest, evidence and recorder receipts, and
segments for the channels that actually received records:

```text
run/
  manifest.json
  normalized/segment-000000.gvmseg
  native/segment-000000.gvmseg
  input-values/segment-000000.gvmseg
  frames/segment-000000.gvmseg
  snapshots/segment-000000.gvmseg
  capabilities/segment-000000.gvmseg
  evidence-receipt.json
  recorder-receipt.json
```

Empty or unrequested channels have no placeholder segment. The manifest stores
run identity, the observation request, selected channels, and recorder limits.
The evidence receipt records fulfillment. The recorder receipt records what
was persisted and whether the package can be published.

## Segment format

Each `.gvmseg` file has a versioned prelude, bounded data blocks, and a footer.
Records are typed channel values encoded as CBOR. Each record in a block is
length-framed. The prelude declares the channel, segment index, encoding,
compression mode, and configured limits. The footer records final counts and
sequence coordinates where the channel has them.

The current segment format supports CBOR records with no block compression.
The framing detects malformed or incomplete segments; it is not a cryptographic
integrity or anti-tamper mechanism.

## Hard limits

`RecorderLimits` sets a maximum logical block size, maximum records and logical
bytes per segment, and maximum live logical bytes per active channel. The
limits are enforced while records are admitted. A record larger than the
block limit fails; it is not split across blocks. The block limit must fit
within both the segment and active-channel buffer limits.

These limits bound recorder buffering. They do not set a retention policy or
limit the size of the completed run on disk.

## Finalization and publication

The recorder writes first to a unique staging directory next to the requested
published path. It writes the evidence receipt exactly once, closes and
validates segment files, writes the recorder receipt, and then atomically
renames the staging directory to the published path. The destination must not
already exist.

Callers should treat a run as published only when the returned publication
outcome is `published` and the result contains a published-run reference. If
execution or recording is interrupted, staging data may remain for inspection
and cleanup. The current file recorder does not resume a partial run.

Evidence fulfillment and recorder persistence have separate statuses. A
recorder can successfully publish a run whose evidence receipt reports
incomplete evidence; publication means the recorded package finalized
correctly, not that every requested channel was complete.

## Reading a published run

`PublishedFileRun::open` validates the manifest, receipts, and recorded segment
counts. Use its channel-specific segment paths and `FileSegmentReader` to
stream records. A consumer should read only the channels needed for its task
instead of building a whole-run object by default.
