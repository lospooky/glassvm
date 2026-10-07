# GlassVM release-candidate preflight

**Status (2026-10-04):** The five machine-neutral GlassVM contract crates are
published at `0.1.0`. Machine-specific Rust crates are implementation details
and are marked `publish = false`; the three machine Python distributions have
not been published to PyPI. The release denominator is CHIP-8, PICO-8, and
TIC-80. The exact public source coordinates are GlassVM
[GlassVM `cf7dee8`](https://github.com/lospooky/glassvm/commit/cf7dee8444e1f5968f7206f65335663855c981e2)
and [Machines `56563bd`](https://github.com/lospooky/glassvm-machines/commit/56563bd0dfb92a67d272ed4d8ab4868e448b265d).
[GlassVM CI](https://github.com/lospooky/glassvm/actions/runs/37216452075)
and [Machines CI](https://github.com/lospooky/glassvm-machines/actions/runs/37216276015)
pass on those heads, including CPython 3.12, 3.13, and 3.14
source-distribution-to-wheel builds and the base/per-bundle/all-bundle
clean-room matrix. Package-index installation and empty-cache publication
checks remain open. Hexwell and Wyrd-16 are
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

## Published Rust contract crates

The only crates.io publication targets are the five machine-neutral GlassVM
contract crates, all available at `0.1.0`:
[`glassvm_normalizer_contract`](https://crates.io/crates/glassvm_normalizer_contract/0.1.0),
[`glassvm_core`](https://crates.io/crates/glassvm_core/0.1.0),
[`glassvm_recorder`](https://crates.io/crates/glassvm_recorder/0.1.0),
[`glassvm_query`](https://crates.io/crates/glassvm_query/0.1.0), and
[`glassvm_registry`](https://crates.io/crates/glassvm_registry/0.1.0).

Machine-specific Rust packages are not crates.io release targets and have
`publish = false`. Their sibling path dependencies are internal to each
machine's complete Python source distribution. The machine lockfile resolves
all five GlassVM dependencies from crates.io with registry checksums.

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

The 2026-10-04 source-archive round-trip check built all four sdists and
rebuilt their wheels from the archives under CPython 3.14. This validates the
included sibling Rust source closure; it did not use a fresh empty Cargo/uv
cache and is not package-index installation evidence.

## Artifact set

The narrowed candidate contains four Python distributions, all at version
`0.1.0`, with `Requires-Python: >=3.12`:

- `glassvm` (the generic facade and `glassvm` import module);
- `glassvm-chip8`;
- `glassvm-pico8`; and
- `glassvm-tic80`.

CI covers CPython 3.12, 3.13, and 3.14 on Ubuntu 24.04 x86_64. No Windows or
macOS wheel/build claim is made by this evidence.

The provider protocol is the exact metadata contract
`glassvm.python_bundle` at numeric version `1.0.0`. The base distribution has
no built-in machine registry. Bundle discovery comes only from installed
`glassvm.machine_bundles` entry points.

## Required pre-publication checks

From a clean clone of `glassvm`:

```bash
cargo test --workspace --locked --no-fail-fast
uv build --python 3.14 --directory python/facade --no-sources
```

From a clean clone of `glassvm-machines`:

```bash
cargo test --workspace --locked --no-fail-fast

for machine in chip8 pico8 tic80; do
    uv build --python 3.14 --directory "$machine" --no-sources
done
```

The public CI goes further: it creates each sdist, rebuilds the wheel from
that archive, then runs the clean-room install matrix for CPython 3.12, 3.13,
and 3.14. The commands above are a concise local checkout smoke, not a
substitute for the CI source-archive round trip.

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

Bundle source distributions resolve the published GlassVM contract versions
declared by their manifests. For this candidate:

1. **Complete:** publish the five GlassVM crates to crates.io in dependency
   order at `0.1.0`.
2. **Complete:** resolve and commit the machine workspace lockfile against
   those registry releases.
3. **Complete for the evaluated source:** build each source distribution and
   wheel, rebuild wheels from the sdists in CI, and pass the clean-room
   provider matrix. Fresh empty-cache reproducibility remains open.
4. **Pending:** publish the four machine Python distributions to PyPI and
   verify installation from the package index in clean environments.

The machine manifests use ordinary `0.1.0` crates.io requirements. No Git tag,
monorepo path dependency, or hidden local fallback is part of the clean-room
release path. This requirement is deliberately separate from wheel execution:
a wheel can be built from a local checkout while its sdist still fails to
reproduce the build until the declared registry dependencies exist.

## Publication boundary

This preflight does not publish machine distributions to PyPI, publish new
GlassVM crate versions, create or move tags, or push the repositories. Those
remain explicit release operations after package-index credentials and the
candidate artifact coordinates have been reviewed.
