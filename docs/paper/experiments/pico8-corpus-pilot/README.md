# PICO-8 autonomous-cart screening pilot

Status: exploratory compatibility screen, not the final corpus or an experiment
result for the manuscript. The affected tweetcart screening path is paused for
review; no production changes have been made.

## Candidate sources

- [Sean S. LeBlanc, Tweetcarts 2017](https://seans.site/project/Tweetcarts%20-%202017/): the author's archive contains 124 numbered `.p8` animation carts. The author makes source available, but no explicit redistribution license was identified on the project or itch.io pages. The pilot downloads and hashes the archive; it does not redistribute the cartridges. Candidate selection: consecutive IDs 51–80, specified before testing. A direct textual scan found no `btn`, `btnp`, or `stat` calls in these 30; this is not a proof of autonomous behavior.
- [koipondx86, 1k Demo Collection](https://koipondx86.itch.io/democollection1k): five named demos and downloadable cart archives. License and individual autonomy not yet confirmed; not downloaded or tested.
- [Tobias V. I. Langhoff, Postcart Gallery](https://tvil.itch.io/postcart-gallery): includes three apparent visual demonstrations and two explicitly playable entries. The page lists MIT for code; other rights, individual autonomy, download and compatibility remain unverified. Not tested.
- Existing reference [fireintro.p8](../../../../../glassvm-machines/pico8/fixtures/paper/fireintro.p8), upstream MIT provenance recorded by the machine bundle: known paper workload used as pilot control, not a new candidate from the archive.

The sources are not representative samples of all PICO-8 programs. Do not choose a final corpus solely by excluding compatibility failures without recording the selection and its limits.

## Pilot

On 2026-11-08, each of `tweetcart51.p8` through `tweetcart80.p8` ran in its own process, up to 10 frames and four seconds wall time per process, with three concurrent workers. Configuration: one cycle per frame, instruction budget 500,000, machine seed 0; empty input schedule, frame hashes requested. Python distribution versions `glassvm==0.3.1` and `glassvm-pico8==0.3.1` were installed. This is a compatibility screen, not a timing benchmark or a comparison to the official PICO-8 runtime.

All 30 preparations succeeded, then execution returned Python `ValueError`s during Lua cartridge loading; zero reached a completed run. The first reported errors group as follows:

| First reported error | Carts |
| --- | ---: |
| Lua instruction budget exhausted | 13 |
| `circ` unavailable | 6 |
| `sspr` argument conversion | 5 |
| Numeric conversion out of range | 4 |
| Syntax error | 1 |
| `circfill` unavailable | 1 |

The cases are leads, not diagnoses: loops, unsupported APIs, compatibility limitations, errors in the carts, or implementation defects may play different roles. In particular, an instruction budget is not a wall-clock timeout, and we have not established whether a different valid configuration would alter an outcome. Source review found top-level looping and `flip()` in many candidates, so callback-centric workloads are another source to investigate separately. Per-cart exceptions and artifact digests are in [results.json](results.json).

The existing Fire Intro paper cart passed the same 10-frame request: 10 frame artifacts, 10 distinct fingerprints, completed machine execution and complete evidence receipt; see [control.json](control.json). This is a positive control for the pilot path, not proof of exact upstream fidelity.

## Reproduction

From the GlassVM repository root with the stated published Python distributions installed:

```sh
docs/paper/.venv/bin/python docs/paper/experiments/pico8-corpus-pilot/pilot.py

docs/paper/.venv/bin/python docs/paper/experiments/pico8-corpus-pilot/pilot.py --child \
  < ../glassvm-machines/pico8/fixtures/paper/fireintro.p8
```

The driver verifies the downloaded archive's SHA-256 before running, captures each cart digest and error, and writes `results.json`. The source archive is not retained by the driver. Confirm rights and source stability before any publication or redistribution. `control.json` is the separately captured positive-control result; use the second command to reproduce it.
