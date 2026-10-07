# Slice 2: release and conformance audit

Audit date: 2026-10-04. Status: **source, conformance, and CI audit complete;
registry release gates remain open**. This execution record and the companion
related paper/release audit documentation are intentionally local and
uncommitted for review.

## Evaluated coordinates and CI

| Repository | Public `main` | CI evidence |
| --- | --- | --- |
| GlassVM | [`cf7dee8444e1f5968f7206f65335663855c981e2`](https://github.com/lospooky/glassvm/commit/cf7dee8444e1f5968f7206f65335663855c981e2) | [Run 37216452075](https://github.com/lospooky/glassvm/actions/runs/37216452075): Rust checks and facade sdist-to-wheel builds on CPython 3.12, 3.13, and 3.14. |
| Machines | [`56563bd0dfb92a67d272ed4d8ab4868e448b265d`](https://github.com/lospooky/glassvm-machines/commit/56563bd0dfb92a67d272ed4d8ab4868e448b265d) | [Run 37216276015](https://github.com/lospooky/glassvm-machines/actions/runs/37216276015): Rust/conformance plus sdist-to-wheel and base-only, per-provider, and all-provider clean-room installs on CPython 3.12, 3.13, and 3.14. |

The previous CI run covered Python 3.10/3.13 while all project metadata still
claimed Python 3.9+. The user resolved the advertised floor as Python 3.12+.
The bindings now use PyO3 0.27.2, the CI matrix covers 3.12/3.13/3.14, and
`Requires-Python` is `>=3.12` for the facade and all three bundles. The machine
CI builds the generic facade from the public GlassVM source and all provider
wheels from the actual source distributions before installation.

## Cross-machine conformance

The shared tests construct a case for each publication bundle and assert the
same contract guarantees without assuming identical machine semantics. The
full property-to-evidence mapping is in
[`PAPER_CONFORMANCE_MATRIX.md`](PAPER_CONFORMANCE_MATRIX.md).

The audit found no cross-machine contract gap in artifact/config/input
admission, prepared execution and observation separation, exact prerequisite
negotiation, emission-channel separation, bounded online capability outputs,
continuation, FileRunSession, or Python provider behavior. Material differences
remain intentional: live input is advertised only by CHIP-8; PICO-8 and TIC-80
execute explicitly bounded Lua compatibility subsets. No full-console or
instruction-exact claim is supported.

The maturity inventory marks twelve implementation/conformance dimensions
verified per release machine. It retains two explicit open dimensions per
machine: packaging/provenance and paper evidence. The packaging gap now names
the actual remaining gates rather than stale source coordinates.

## Release and dependency audit

### License

- Both repository roots contain BSD-3-Clause `LICENSE` files.
- The GlassVM publishable crates, Python facade Cargo package, machine
  workspace members, and all four Python project metadata entries declare
  BSD-3-Clause; machine Cargo metadata inherits the workspace value.
- Machine Rust implementation packages are all `publish = false`.
- Third-party fixture provenance and bundled upstream license notices were
  not changed; their SPDX/license descriptions remain fixture-specific.
- No licensing blocker was found.

### Intended package targets and dependency closure

The crates.io target is only the five machine-neutral GlassVM crates:
`glassvm_normalizer_contract`, `glassvm_core`, `glassvm_recorder`,
`glassvm_query`, and `glassvm_registry`, all published at `0.1.0`. `cargo info`
resolved all five from crates.io. Machine-specific Rust crates are not release
targets. The four intended Python distributions are `glassvm`,
`glassvm-chip8`, `glassvm-pico8`, and `glassvm-tic80`.

The machine manifests use registry version dependencies for GlassVM; the
machine `Cargo.lock` records registry sources and checksums for all five
crates. The only machine-workspace path dependencies are sibling source crates
(machine core/verifier/bundle/Python binding and conformance edges). Those are
included in each complete machine sdist, not claimed as publishable crates.io
dependencies. This distinction was verified by building all three machine
sdists and rebuilding their wheels from the archives. The generic facade sdist
was also rebuilt to a wheel. An installed, local-index all-provider run from
those sdist-built wheels passed on CPython 3.14.

The source-archive builds used the existing local uv/Cargo caches. Empty-cache
release reproducibility remains unverified and stays open in the maturity
inventory. All four PyPI project pages returned 404 on 2026-10-04; the Python
distributions remain unpublished, so package-index-only installation is also
open.

### Python and platform boundary

The declared minimum is Python 3.12. CI currently proves CPython 3.12, 3.13,
and 3.14 on Ubuntu 24.04 x86_64. No Windows or macOS build/install claim is
made. `Requires-Python >=3.12` is the package metadata floor, not a claim that
every operating system or future interpreter has been tested.

## Prohibited-surface audit

Both repositories have independent source checks and both checks pass:

- `cargo test --locked --test prohibited_surface -p glassvm_core`
- `cargo test --locked --test prohibited_surface -p glassvm_machine_conformance`

The core check was found to walk one parent too high and therefore scan the
wrong root. That was repaired in `cf7dee8`; its rerun now scans the actual
publication source roots. The repaired scan exposed the old provider protocol
spelling inside the intentional negative test. The test still checks rejection
but now constructs the rejected spelling from the canonical protocol constant,
so the obsolete identifier is absent from publication source.

The exact forbidden identifiers otherwise occur only in the two enforcement
test tables and in this paper checklist's audit vocabulary. Historical
implementation notes remain quarantined under `docs/history/` or fixture
reference corpora. Current PICO-8/TIC-80 uses of “Lua compatibility runtime”
describe the limited machine semantics and are not an obsolete GlassVM
compatibility layer. No release-facing prohibited API or old provider fallback
was found.

## Claim disposition

- **C1:** shared caller and three-machine conformance are evidenced; public CI
  passes on the pinned current sources. Python index installation remains
  unverified because no GlassVM Python packages are published.
- **C2:** preparation-time required failure, optional downgrade with reason,
  exact prerequisite closure, and typed configuration/input admission are
  tested across the three bundles. Receipts and all four run outcome domains
  are kept distinct in the clean-room provider checks.
- **C3:** the selectable channels, bounded reducers, hard evidence budgets,
  recorder hard limits, and publication gate are implementation/conformance
  evidence. Performance, storage-cost, and run-length memory measurements have
  not been collected; no empirical boundedness claim is closed.

## Remaining release gates

1. Publish the four Python distributions, then verify the package-index-only
   base/provider/all install matrix from an environment with no repository
   checkout.
2. Repeat sdist and wheel builds with fresh/empty caches and record their
   exact source and package coordinates.
3. Continue to Slice 3's generated heterogeneity table.
4. Keep all benchmark and long-run memory claims open until Slice 4 onward
   produces reproducible raw results.

No paper planning/checklist document was committed as part of this slice.
