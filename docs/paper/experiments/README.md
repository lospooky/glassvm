# Paper artifact regeneration

Use the paper runner as the single supported entry point. Each invocation
creates a disposable environment from scratch: a locked Python 3.12 environment
with uv caching disabled, plus an isolated Cargo home and build target. This
prevents an existing project venv, user site package, Cargo registry cache, or
old target directory from satisfying a missing dependency accidentally. The
runner binds the paper matrix to published GlassVM `0.3.0` wheels and crates,
checks the three pinned workload artifacts, and keeps generated measurements
outside both source checkouts.

## Environment and source checkouts

The experiment environment is locked in [`../pyproject.toml`](../pyproject.toml)
and [`../uv.lock`](../uv.lock). Install `uv`, CPython 3.12+, and a Rust
toolchain. From the GlassVM repository root, invoke the runner directly:

```sh
python3 docs/paper/run_paper.py check \
  --machines-root ../glassvm-machines
```

The runner creates its Python environment, uv cache directory, Cargo home, and
Cargo target under a temporary directory and removes them on exit. It uses the
installed Rust toolchain (recorded in `paper-run.json` for a full run), while
all Rust crates resolve from a fresh crates.io registry cache. Python wheel
dependencies resolve from the locked package indexes without reusing uv cache.

The companion machine checkout must be on its `paper-publication` branch: that
branch contains the paper-only memory trial example; the release-facing
machine implementation is synchronized from the `0.3.0` source release. The
paper-run environment installs `glassvm==0.3.0` and all three provider bundles
at `0.3.0` from PyPI. The runner verifies those exact installed versions and
entry-point discovery before collecting results.

## One-command regeneration

After committing the paper runner and synchronizing both source checkouts, run
the full regeneration from clean source trees:

```sh
python3 docs/paper/run_paper.py all \
  --machines-root ../glassvm-machines \
  --results-root ../glassvm-paper-results/0.3.0-final
```

The results root must be empty or absent. It is deliberately outside the two
repositories so result writes cannot dirty the measured sources. The runner
refuses dirty checkouts for the full pass, records both full source revisions
in `paper-run.json`, and does not overwrite an existing result directory. Use
a new results-root for a rerun or changed configuration.

The ordered full pass is:

1. preflight: locked package versions, installed providers, fixture digests,
   a standalone Rust consumer linked only to published 0.3.0 crates.io crates,
   paper harness tests, and generated heterogeneity-table freshness;
2. Glass Garden SVG postcards and external-consumer clean-room installs;
3. one-repetition harness pilot and five-repetition selectable-observation
   matrix, with their reports;
4. Python/Rust fan-out semantic-equivalence matrix;
5. the three-machine memory-scaling matrix and Rust sink-cost slice;
6. the Python consumer sink-cost matrix;
7. source-linked heterogeneity tables and a top-level run manifest.

The full pass is intentionally substantial. It includes the 600,000-frame
memory endpoint, isolated package-install tests, and repeated sink matrices.
It is not run as part of the `check` command.

## Individual stages

The same entry point can run one stage into its corresponding subdirectory of
the results root:

```text
check                static/runtime/fixture/harness preflight
pilot                one-repetition harness pilot
observations         five-repetition selectable-observation matrix
memory               run-length and evidence-volume matrix
rust-sink            disabled / NullSink / recorder comparison
python-sink          Python consumer / recorder sink-cost comparison
equivalence          Python consumer vs Rust recorder projections
garden               three bounded-consumer SVG postcards
external-consumer    base-only and per-bundle clean-room installs
all                  ordered complete pass above
```

Every collection stage records its raw trial-level outcomes and keeps execution,
evidence, recorder, consumer-delivery, and publication status distinct. The
runner is orchestration only: the existing focused scripts remain the
implementation of their experiment, and the reference recorder remains Rust
owned. It does not read recorder storage to reconstruct evidence or introduce
a global cross-channel order.

## Interpretation and result handling

`run_pilot.py` is a harness/accounting pilot; the repeated observation matrix
is the observation-cost dataset. `run_memory_experiment.py` measures process
RSS and receipt/storage volume separately. `run_python_sink_cost.py` measures
the public bounded Python consumer alongside the Rust reference recorder.
`run_semantic_equivalence.py` compares explicit finite channel projections,
prepared-run and prepared-observation identities, capability outputs, and
receipts; it does not compare recorder layout or infer a global channel order.

Treat all performance values as specific to the recorded host, fixtures,
configuration, package releases, and source coordinates. The existing
measurement directories in this branch document prior runs and are immutable
historical records; the `0.3.0-final` run writes a new, independent result set.

For claim scope, release-machine roles, and limitations, see
[`../CLAIM_EVIDENCE_MAP.md`](../CLAIM_EVIDENCE_MAP.md) and
[`../PAPER_CHECKLIST.md`](../PAPER_CHECKLIST.md).
