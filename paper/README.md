# GlassVM manuscript scaffold

This tree is a neutral `article` manuscript for the GlassVM paper. It is
prose-ready but not sealed: empirical results, exact software coordinates,
and literature records remain explicit placeholders. Do not replace a
`TBA:` marker with a value from an intermediate experiment summary.

## Build

The draft requires [Tectonic](https://tectonic-typesetting.github.io/) and
uses standard LaTeX packages plus TikZ for the architecture diagram.
From this directory:

```sh
make draft
```

The draft target regenerates the qualitative heterogeneity table, lists
unresolved markers, and compiles the manuscript with placeholders visible.
Build files go under `build/`.

For a final-release build, every result and citation placeholder must first
be resolved from sealed receipts and checked bibliography records:

```sh
make final
```

`make final` fails if `TBA:` remains in any TeX or BibTeX source. To inspect
the current inventory without compiling, run `python3 check_tba.py`; use
`python3 check_tba.py --final` to apply the final-release gate.

## Structure

- `main.tex` assembles the neutral article and section files.
- `sections/` contains the abstract and manuscript sections.
- `macros.tex` centralizes evidence placeholders and source coordinates.
- `heterogeneity.json` is the source for
  `generated/heterogeneity_table.tex`; regenerate it with
  `python3 generate_heterogeneity.py`.
- `figures/architecture.tex` is the source for the conceptual architecture
  diagram. Evaluation and downstream-consumer figures currently render as
  labeled placeholders in the draft.
- `references.bib` contains no fabricated records. Add only references with
  verifiable metadata found in repository research notes or source material.

The sealed artifact-generation step is expected to supply figure data for
selective-observation cost, memory versus run length, Python sink overhead,
and Glass Garden postcards. Until that generator defines stable filenames,
the manuscript uses labeled boxes rather than invented paths. The boxes are
identified by `TBA:GENERATED_*_FIGURE` markers in the corresponding section
files.

## Evidence and coordinates

The manuscript centralizes evidence slots in `macros.tex`, including
`TBA:SEALED_MEMORY_RESULT`, `TBA:SEALED_OVERHEAD_RESULT`,
`TBA:SEALED_STORAGE_RESULT`, `TBA:SEALED_PYTHON_SINK_RESULT`,
`TBA:SEALED_EQUIVALENCE_RESULT`, and
`TBA:SEALED_PACKAGE_PROOF_COORDINATES`. Search for `TBA:` to inspect all
unresolved result, caption, citation, and generated-figure slots.

Source revisions and release versions appear only through the dedicated
macros `\GlassVMSourceSHA`, `\MachinesSourceSHA`, `\GlassVMVersion`, and
`\MachinesVersion` in `macros.tex`. Replace them only from the sealed source
and package receipts. The related-work section remains intentionally
uncited until verified references are available.

## Scope note

The prose uses qualitative claims grounded in the public GlassVM and
glassvm-machines documentation and the contract descriptions supplied for
this scaffold. The evaluation methods are proposed structure, not results.
No final quantitative synthesis has been performed. The draft does not claim
substrate-independent computation, universal bounded memory, universal
performance, population-scale throughput, or cross-host guarantees.
