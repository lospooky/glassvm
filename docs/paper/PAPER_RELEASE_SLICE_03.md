# Slice 3: machine heterogeneity table

Execution date: 2026-10-07. Status: **complete; changes remain local and
uncommitted for review**.

## Evaluated coordinates

| Repository | Source revision |
| --- | --- |
| GlassVM | [`ba5637a72d5484ac170069021fbcbc28ee25f61e`](https://github.com/lospooky/glassvm/tree/ba5637a72d5484ac170069021fbcbc28ee25f61e) |
| GlassVM Machines | [`7cf884861e3cc22d6f15badc7265c969af032e0a`](https://github.com/lospooky/glassvm-machines/tree/7cf884861e3cc22d6f15badc7265c969af032e0a) |

The source file pins both revisions. Every cell in the generated comparison
links to its supporting declaration or supported-subset documentation at the
corresponding immutable revision. A local validation checked all 38 referenced
source paths against those Git objects.

## Deliverables

- Machine-readable comparison: [PAPER_HETEROGENEITY.json](PAPER_HETEROGENEITY.json)
- Deterministic generator: [generate_heterogeneity.py](generate_heterogeneity.py)
- Generated table: [PAPER_HETEROGENEITY.md](PAPER_HETEROGENEITY.md)
- LaTeX-ready long table: [PAPER_HETEROGENEITY.tex](PAPER_HETEROGENEITY.tex)

The table covers all twelve dimensions in checklist section 6: artifact,
execution, configuration, inputs, timing/coordinates, frames, normalized and
native evidence, snapshots, capabilities, static analysis/verifier, and
Python-provider surface. It distinguishes the CHIP-8 baseline from the PICO-8
and TIC-80 external validation specimens while making clear that all three use
the same preparation and file-backed caller lifecycle.

## Scope qualifications

PICO-8 and TIC-80 are described as bounded Lua 5.4 compatibility runtimes over
documented subsets. The table makes no instruction-exact or complete-console
claim. In particular, it notes that upstream TIC-80 targets Lua 5.3 and that
the exposed operation/event schemas do not imply per-Lua-instruction traces.
CHIP-8's instruction-stepped semantics are not generalized to those machines.

This artifact is a source-backed contract comparison only. It does not claim
compatibility completeness, cross-platform support, performance, storage cost,
or empirical run-length-independent memory behavior. Those remain gated by
later checklist slices.

## Validation

- `python3 docs/paper/generate_heterogeneity.py` generated both table files.
- `python3 docs/paper/generate_heterogeneity.py --check` passed after generation.
- The source-data validator passed: all 12 dimensions have all three machine
  cells, each cell has source references, and both repository revisions are
  pinned.
- A local Git-object check found all 38 referenced source paths at their
  pinned revisions.
- No Rust or bundle changes were needed; the work is documentation and table
  tooling only.

## Changed files

- `docs/paper/PAPER_HETEROGENEITY.json`
- `docs/paper/generate_heterogeneity.py`
- `docs/paper/PAPER_HETEROGENEITY.md`
- `docs/paper/PAPER_HETEROGENEITY.tex`
- `docs/paper/PAPER_CHECKLIST.md`
- `docs/paper/PAPER_RELEASE_EXECUTION_PLAN.md`
- `docs/paper/PAPER_RELEASE_SLICE_03.md`

No commit was created, as requested.
