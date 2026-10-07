# Slice 1: implementation and workload freeze

Review date: 2026-10-04. Status: **complete at the published source coordinate;
all Slice 1 acceptance gates passed**. This is an execution record for
[PAPER_RELEASE_EXECUTION_PLAN.md](PAPER_RELEASE_EXECUTION_PLAN.md), not a claim
that the source or empirical denominator is already frozen.

## Source inventory

| Repository | Public main | Local committed main | Pending work |
| --- | --- | --- | --- |
| GlassVM | `5aa7d2a60210f173b344958e1338e6f8500c2a81` | Same | Untracked paper checklist, execution plan, slice record, workload manifest, and historical `standalone_glassvm` directory; unrelated modified `.gitignore`. |
| Machines | `66860cad9db978d6399f4f1a7a0e016e61c82b92` | Same; clean and synchronized with public main | Reviewed source and workload changes are published. |

Both public heads were checked after pushing. The four machine commits are now
published: `eba2be1` (PICO-8 hidden Lua state continuation), `27b8a15` (bounded
TIC-80 trace/input retention), `8486fc7` (TIC-80 graphics reference fixes),
and `66860ca` (pinned paper workloads and consistent fixture layout). GlassVM
paper planning documents remain uncommitted for later review; unrelated
`.gitignore` and `standalone_glassvm/` work was preserved.

## Reference review and repairs

The TIC-80 review uses official source revision
`4aba09c98f1e5028b82765be1647677b08d35942`, matching the bundle's recorded
reference revision. The locally vendored source-doc corpus does not include
the implementation files below; they were read directly from the pinned
official repository.

| Finding | Disposition | Regression evidence in `tic80/core/tests/emulation.rs` |
| --- | --- | --- |
| Affine image texture sampling treated tile RAM as a linear image. Native sheet pixels resolve through individual 8x8 tiles. | Fixed tile index and intra-tile address. Corrected the original test's matching wrong stride. | `textured_triangles_sample_image_map_and_other_vbank_with_clip_and_colorkey`, now also comparing sprite and triangle reads of the same tile. |
| `map()` clamped negative coordinates; upstream wraps them and passes wrapped coordinates to remap callbacks. | Fixed lookup and callback coordinates together. | `map_wraps_lookup_and_callback_coordinates_at_native_boundaries`. |
| `cls()` ignored clipping; negative clip origins also widened the effective rectangle. | Fixed intersected clip bounds and clipped clears, preserving single palette-map application and the full-screen fast path. | `clipped_clear_maps_color_once_and_negative_colorkey_keeps_color_fifteen`. |
| Explicit `-1` sprite/map colorkeys incorrectly became index 15. | Negative colorkey no longer selects a transparent palette color. | Same clipped-clear/colorkey regression. |
| BDR implementation retains RGB palettes per row, not intermediate screen/overlay/offset state. | Documented palette-only raster subset; no full raster-effect claim. No extra raster machinery added. | Existing BDR row-order and palette test; arbitrary BDR drawing remains outside the validated subset. |
| Affine texture support is not depth/perspective or selectable blit-segment/BPP support. | Documented default 4-bpp subset and preserved explicit nonzero-depth rejection. | Existing depth rejection and affine sampling tests. |

Primary references:

- [Upstream draw.c](https://github.com/nesbox/TIC-80/blob/4aba09c98f1e5028b82765be1647677b08d35942/src/core/draw.c): `drawMap`, `tic_api_clip`, `tic_api_cls`, palette/colorkey handling.
- [Upstream tilesheet.h](https://github.com/nesbox/TIC-80/blob/4aba09c98f1e5028b82765be1647677b08d35942/src/tilesheet.h): sheet coordinates, tile indices, and intra-tile pixel addressing.

These targeted checks establish the named behavior only. They are not a
pixel-exact full-console differential evaluation. In particular, transformed
multi-tile sprites are not established by the existing single-tile transform
and untransformed composite tests. Do not silently extend those test claims.

## Frozen workloads and supported subset

The user selected suitable publicly available, zero-input cartridges for all
three machines and retained the reviewed TIC-80 palette/BDR/affine subset. The
canonical workload manifest is
[`PAPER_WORKLOADS.toml`](PAPER_WORKLOADS.toml). All three paper artifacts live
under their bundle's `fixtures/paper/` directory with native extensions and
exact upstream basenames; original upstream inputs remain under
`fixtures/source/public/`. The manifest pins artifact hashes, source paths,
upstream revisions, machine configuration, empty schedules, normalized event
selection, requested visual capability, frame-capture mode, and run length.

| Machine | Existing fixture | What it establishes / limitation |
| --- | --- | --- |
| CHIP-8 | `chip8/fixtures/paper/octojam2title.ch8`; CC0-1.0 Octojam 2 title animation from `JohnEarnest/chip8Archive`, pinned to commit `761e3ffc63f43e6a849e80714122feff2afca208`, blob `6e19074790fe94742e8594b6b109d4946c38d9dc`. | Public, zero-keypad animated specimen. Its exact source basename is also retained under `source/public/`. Run for 600 frames; request `chip8.visual_trajectory_motifs`, frame completion/display/input-sample event kinds, and hash frames. The generated four-byte loop ROM remains the small conformance smoke fixture. |
| PICO-8 | `pico8/fixtures/paper/fireintro.p8`; MIT-licensed fire-effect cartridge from `thxrsxm/fire-intro-effect`, pinned to commit `428e67017a8951d4ec32a7adadc0907aa9a36b6e`, blob `33329fd679fcdbff1a2a22ef49fd3cce2bebd6a2`. | Public zero-input animated workload, with a deliberately documented title-only P8SCII-to-ASCII normalization because the current parser rejects those title escapes. Fire update/render logic is unchanged. Retains the upstream filename and `.p8` extension. Not an unmodified-upstream result or full PICO-8 compatibility claim. |
| TIC-80 | `tic80/fixtures/paper/game.tic`; MIT-licensed Gecko Lua cartridge from `lincerely/gecko`, pinned to commit `f958fd19218510468a36f93b77449ca26a2d13cf`. | Public zero-gamepad-input game workload. The source is preserved as `source/public/game.tic`, retaining the upstream filename; the paper artifact retains `.tic`. Run for 600 frames; request `tic80.visual.motifs`, frame/input-sample/trace event kinds, and hash frames. The legacy `smoke.rom` copy remains only for existing tests. TIC-80 claims remain limited to the implemented Lua/API subset. |

For each specimen, machine defaults are made explicit in the manifest, the
input schedule is empty, and the caller imposes a 600-frame limit. The selected
observation captures frame fingerprints and asks for the bundle's visual
capability. The conformance gate checks that all 600 frame artifacts have
distinct temporal coordinates, machine step advances, hashes change, and one
canonical visual output plus a fulfilled nonempty receipt is produced whose
frame/change counts reflect the run. This is a finite conformance workload,
not the later performance or bounded-memory experiment.

The retained TIC-80 graphics scope is precisely: palette operations, palette
selection by row through BDR, and affine textured triangles with default 4-bpp
texture addressing. Nonzero depth, perspective, arbitrary raster-state BDR,
audio synthesis, keyboard, and mouse remain outside the supported claim. This
scope decision does not establish every such feature through the selected Gecko
workload; targeted reference tests separately cover the named graphics rules.

## Checks performed

All checks below passed on the local, uncommitted machine worktree:

```sh
cargo test --workspace --locked --no-fail-fast
cargo test --locked -p tic80_core --test emulation
cargo clippy --locked -p tic80_core -p tic80_bundle -p tic80_verifier \
  --all-targets --no-deps -- -D warnings
cargo fmt --all -- --check
git diff --check
```

The combined local worktree now passes:

```sh
cargo test --workspace --locked --no-fail-fast
cargo clippy -p glassvm_machine_conformance -p tic80_core -p tic80_bundle \
  --all-targets --locked --no-deps -- -D warnings
(cd chip8/fixtures && sha256sum -c SHA256SUMS)
(cd pico8/fixtures && sha256sum -c SHA256SUMS)
(cd tic80/fixtures && sha256sum -c SHA256SUMS)
cargo fmt --all -- --check
git diff --check
```

All workspace tests passed, including 14 TIC-80 emulation tests and the new
three-machine paper workload gate. All three fixture checksum lists verified.
Changed-package Clippy passed. A broader workspace Clippy attempt is currently
blocked by a `collapsible_if` warning in unchanged
`chip8/verifier/src/analysis.rs:194` under the local Clippy 1.97; no unrelated
verifier edit was made for this slice.
The texture-addressing,
map-wrap, and clipped-clear regression cases were observed failing against the
pre-fix local implementation before their corresponding repairs.

Local green tests were followed by public clean-room CI on the exact published
source coordinate; details are below.

Additional targeted gate, passed after the workload scope was frozen:

```sh
cargo test --locked -p glassvm_machine_conformance --test paper_workloads -- --nocapture
```

It runs all three selected zero-input cartridges for exactly 600 frames, checks
their selected normalized-event prerequisites and first-class hash-frame
emissions, and requires each visual capability output/receipt to be fulfilled
with frame and changing-frame counts matching the execution. The test is part
of the published machine commit.

## Published-source CI

[Machine CI run 37214867564](https://github.com/lospooky/glassvm-machines/actions/runs/37214867564)
passed on machine SHA `66860cad9db978d6399f4f1a7a0e016e61c82b92`, using GlassVM
facade SHA `5aa7d2a60210f173b344958e1338e6f8500c2a81`. It passed:

- Rust formatting, workspace tests, and machine conformance.
- CPython 3.10 facade/provider builds and base-only, per-bundle, and all-bundle installs.
- CPython 3.13 facade/provider builds and base-only, per-bundle, and all-bundle installs.

## Next actions after decisions

1. Review the still-uncommitted paper planning/checklist documents separately.
2. Begin Slice 2: audit release/conformance evidence and reconcile the claim map.
