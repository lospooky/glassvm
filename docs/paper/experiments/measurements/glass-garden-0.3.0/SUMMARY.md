# Glass Garden

A small downstream consumer attached directly to each run's Python emission stream.
It requests frame hashes and normalized events only; it does not read recorder files or retain event/frame history.

Run environment: CPython 3.12.13; glassvm 0.3.0, glassvm-chip8 0.3.0, glassvm-pico8 0.3.0, glassvm-tic80 0.3.0.
Each bundle supplies its own declared default configuration. The caller supplies an empty input schedule and the shared 600-frame limit.

Each firefly summarizes changed adjacent frame fingerprints within its frame-coordinate bucket. The event meadow counts normalized events by their own frame coordinate; events without one are reported separately. No cross-channel ordering is inferred.

| Machine | Frame buckets | Changed pairs (bucketed) | Top normalized event kinds | Evidence status | Postcard |
| --- | --- | --- | --- | --- | --- |
| chip8 | 20/20 (600 frames) | 599/599 (20 buckets changed) | `instruction_decoded` × 7200; `step_started` × 7200; `display_write` × 1968; `branch_taken` × 1014; `frame_completed` × 600 | complete | [`SVG`](glass-garden-chip8.svg) |
| pico8 | 20/20 (600 frames) | 599/599 (20 buckets changed) | `frame_completed` × 600; `run_halted` × 1; `run_started` × 1 | complete | [`SVG`](glass-garden-pico8.svg) |
| tic80 | 20/20 (600 frames) | 599/599 (20 buckets changed) | `frame_completed` × 600; `input_sampled` × 600; `run_halted` × 1; `run_started` × 1 | complete | [`SVG`](glass-garden-tic80.svg) |

These postcards are descriptive summaries of the selected evidence, not performance scores, behavioral-equivalence claims, or measures of program complexity.
