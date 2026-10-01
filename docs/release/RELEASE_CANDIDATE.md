# GlassVM release-candidate preflight

**Status:** GlassVM contract crates are published at `0.1.0`; machine crates
and Python distributions remain unpublished. The initial machine release set
is CHIP-8, PICO-8, and TIC-80. Local Rust, wheel-build, base-only, and
all-provider checks for this narrowed set passed on 2026-10-01; the isolated
CI matrix and package-index checks remain pending. Hexwell and Wyrd-16 are
preserved on the `glassvm-machines` branch `parked/hexwell-wyrd16` and are
excluded from this candidate.

This document records the release-candidate boundary for the two standalone
repositories. It is a packaging and evidence checklist, not a new runtime
contract.

## Earlier candidate source heads

The earlier clean-room gate was run from fresh clones at:

| Repository | Candidate commit |
| --- | --- |
| `glassvm` | `38c7c688f68fcee9573050087602379353b6e4de` |
| `glassvm-machines` | `88a67062d2db1d7b44016be358d91afa0d0eaa44` |

Both repositories were clean at the tested heads. That historical gate used
CPython 3.13 on Linux and built the generic facade plus five bundle wheels.
It verifies the previous broader workspace, not the current three-bundle
release set.

## Registry-only verification after crates.io publication

The five GlassVM contract crates are available on crates.io at `0.1.0`:
`glassvm_normalizer_contract`, `glassvm_core`, `glassvm_recorder`,
`glassvm_query`, and `glassvm_registry`.

The historical follow-up verification used the machine workspace at commit
`2443d08fceb2c6d19a5202cb47a39c2818bf63fe` and the generic facade after its
BSD-3-Clause metadata correction. The facade wheel was rebuilt from its source
distribution. Its wheel metadata reports `License-Expression: BSD-3-Clause`
and declared extras for all five bundles at that time. Current metadata is
narrowed to `chip8`, `pico8`, `tic80`, and `all`.

The machine lockfile resolves all five GlassVM crates from the crates.io
registry with registry checksums. `cargo test --workspace --locked
--no-fail-fast` passed. The historical installed-wheel smoke matrix passed
for base-only, each individual provider, and all five providers. These results
do not replace a fresh clean-room run for the narrowed release set. Invalid-
artifact rejection was checked for PICO-8 and TIC-80 only; absence checks
covered providers not installed in each isolated environment.

These checks validate local artifacts; they do not publish machine crates or
Python distributions.

## Artifact set

The narrowed candidate contains four Python distributions, all at version
`0.1.0`:

- `glassvm_py`;
- `glassvm-machine-chip8`;
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

for machine in chip8 pico8 tic80; do
    uv build --python 3.13 --directory "$machine" --no-sources
done
```

Place the four wheels in one temporary local index and run
`conformance/python_clean_room.py` in isolated environments for:

- base only;
- each individual bundle; and
- all three release bundles together.

Every environment must verify installed entry-point discovery, native-module
loading, preparation-time validation, separate execution/evidence/recorder
outcomes, and atomic publication. The base-only environment must reject every
machine request as unavailable.

The malformed-artifact negative check is bundle-specific. It must only be
applied to installed bundles whose contract rejects malformed bytes during
preparation; an absent provider is tested through the missing-provider check,
not the artifact check.

## Dependency and publication order

Bundle source distributions must resolve the published GlassVM contract
versions declared by their manifests. The release order is:

1. publish the GlassVM crates to crates.io in dependency order;
2. regenerate and commit the machine workspace lockfile against those registry
   releases;
3. build and verify the machine source distributions and wheels; and
4. publish the machine distributions.

The machine manifests use ordinary `0.1.0` crates.io requirements. No Git tag,
monorepo path dependency, or hidden local fallback is part of the clean-room
release path. This requirement is deliberately separate from wheel execution:
a wheel can be built from a local checkout while its sdist still fails to
reproduce the build until the declared registry dependencies exist.

## Publication boundary

This preflight does not publish to PyPI, crates.io, or GitHub; it does not
create or move tags; and it does not push either repository. Those are explicit
release operations after the candidate source coordinates and package-index
credentials have been reviewed.
