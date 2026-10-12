# GlassVM documentation

These guides describe the shared GlassVM source and public interfaces. Machine
specific behavior and artifact requirements belong with each bundle in the
[`glassvm-machines`](https://github.com/lospooky/glassvm-machines) repository.

## Guides

- [Architecture](architecture.md) — crate responsibilities, bundle boundary,
  and one run's lifecycle.
- [Observations and inputs](observations.md) — typed inputs, observation
  selection, evidence channels, and receipts.
- [Recording](recording.md) — reference files, segment limits, and publication.
- [Python](python.md) — provider discovery, preparation, sinks, and execution.
- [Bundle authoring](bundle-authoring.md) — implementing the Rust interfaces
  for a machine bundle.

## Where to look in the source

| Topic | Source |
| --- | --- |
| Machine and emulator contracts | `crates/core/src/bundle.rs`, `crates/core/src/lib.rs` |
| Requests and observation negotiation | `crates/core/src/observation.rs`, `crates/core/src/observation_contract.rs` |
| Normalizer and capability types | `crates/core/src/normalizer.rs`, `crates/normalizer-contract/src/lib.rs` |
| File format and run publication | `crates/recorder/src/lib.rs` |
| Event projections | `crates/query/src/lib.rs` |
| Python facade and provider protocol | `python/facade/src/lib.rs` |
