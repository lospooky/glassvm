# GlassVM release-candidate preflight

**Status:** local candidate gate passed; package-index publication is not yet
authorized or complete.

This document records the release-candidate boundary for the two standalone
repositories. It is a packaging and evidence checklist, not a new runtime
contract.

## Candidate source heads

The clean-room gate was run from fresh clones at:

| Repository | Candidate commit |
| --- | --- |
| `glassvm` | `38c7c688f68fcee9573050087602379353b6e4de` |
| `glassvm-machines` | `88a67062d2db1d7b44016be358d91afa0d0eaa44` |

Both repositories were clean at the tested heads. The gate used CPython 3.13
on Linux and built the generic facade plus all five bundle wheels from source
distributions.

## Artifact set

The candidate contains six Python distributions, all at version `0.1.0`:

- `glassvm_py`;
- `glassvm-machine-chip8`;
- `glassvm-machine-hexwell`;
- `glassvm-machine-wyrd16`;
- `glassvm-machine-pico8`; and
- `glassvm-machine-tic80`.

The provider protocol is the exact metadata contract
`glassvm.python_bundle` at numeric version `1.0.0`. The base distribution has
no built-in machine registry. Bundle discovery comes only from installed
`glassvm.machine_bundles` entry points.

## Required pre-publication checks

From a clean clone of `glassvm`:

```bash
cargo test --workspace --locked --no-fail-fast
uv build --python 3.13 --directory python/facade --no-sources
```

From a clean clone of `glassvm-machines`:

```bash
cargo test --workspace --locked --no-fail-fast

for machine in chip8 hexwell wyrd16 pico8 tic80; do
    uv build --python 3.13 --directory "$machine" --no-sources
done
```

Place the six wheels in one temporary local index and run
`conformance/python_clean_room.py` in isolated environments for:

- base only;
- each individual bundle; and
- all five bundles together.

Every environment must verify installed entry-point discovery, native-module
loading, preparation-time validation, separate execution/evidence/recorder
outcomes, and atomic publication. The base-only environment must reject every
machine request as unavailable.

The malformed-artifact negative check is bundle-specific. It must only be
applied to installed bundles whose contract rejects malformed bytes during
preparation; an absent provider is tested through the missing-provider check,
not the artifact check.

## Coordinate requirement before source publication

Bundle source distributions must resolve the same GlassVM contract source that
was tested. Before publishing machine sdists, every machine Cargo manifest
must point to one exact, remotely available GlassVM release revision rather
than the historical scaffold tag. A local commit that has not been pushed is
not a valid sdist dependency coordinate.

This requirement is deliberately separate from wheel execution: a wheel can
be built from a local checkout while its sdist still fails to reproduce the
build if its Git dependency points at the wrong remote revision.

## Publication boundary

This preflight does not publish to PyPI, crates.io, or GitHub; it does not
create or move tags; and it does not push either repository. Those are explicit
release operations after the candidate source coordinates and package-index
credentials have been reviewed.
