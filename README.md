# GlassVM

GlassVM is the generic execution, observation, evidence, recording, and Python
provider facade for machine bundles. The reference bundles live in the
separate [`glassvm-machines`](https://github.com/lospooky/glassvm-machines)
repository.

## Repository units

- `crates/core` — machine-neutral execution and observation contracts;
- `crates/normalizer-contract` — shared schemas for bundle-owned normalizers;
- `crates/recorder` — bounded reference recording and atomic publication;
- `crates/query` — caller-supplied normalized-event projections and queries;
- `crates/registry` — generic validated bundle registration; and
- `python/facade` — the entry-point-discovered Python runtime facade.

The repository contains no built-in machine registry or bundle source. Bundle
selection happens through installed `glassvm.machine_bundles` entry points.

## Development

```bash
cargo test --workspace --locked --no-fail-fast
```

The Python facade is built from `python/facade/pyproject.toml`. Bundle wheels
are built from the companion machine repository and installed separately.
