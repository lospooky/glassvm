# Slice 4 pilot: accounting and lifecycle check

These short, eight-frame runs validate harness wiring and receipt separation;
they are not performance or bounded-memory evidence. Each run used the
published 0.1.1 Python distributions in an isolated Python 3.12 environment.
The published recorder receipt is schema 1.0.0, so its per-channel bounded
recorder counters are unavailable; do not infer them from evidence receipts.
The local recorder source tests cover the new counter aggregation path, but
a package rebuilt against that source is still needed for an end-to-end pilot
of per-channel recorder counters.

| Machine | Recipe | Frames | Evidence | Recorder | Published | Evidence records | Evidence logical bytes | Physical directory bytes | Per-channel recorder counters |
| --- | --- | ---: | --- | --- | --- | ---: | ---: | ---: | --- |
| chip8 | all_normalized | 8 | complete | complete | True | 246 | 178051 | 142972 | unavailable (receipt 1.0.0) |
| chip8 | disabled | 8 | complete | complete | True | 0 | 0 | 4157 | unavailable (receipt 1.0.0) |
| chip8 | frame_full | 8 | complete | complete | True | 8 | 265016 | 138248 | unavailable (receipt 1.0.0) |
| chip8 | frame_hashes | 8 | complete | complete | True | 8 | 3723 | 7575 | unavailable (receipt 1.0.0) |
| chip8 | minimal | 8 | complete | complete | True | 2 | 711 | 5332 | unavailable (receipt 1.0.0) |
| chip8 | mixed | 8 | complete | complete | True | 44 | 23422 | 25693 | unavailable (receipt 1.0.0) |
| chip8 | selected_native | 8 | complete | complete | True | 1 | 461 | 5171 | unavailable (receipt 1.0.0) |
| chip8 | selected_normalized | 8 | complete | complete | True | 34 | 18877 | 19881 | unavailable (receipt 1.0.0) |
| chip8 | visual_capability | 8 | complete | complete | True | 43 | 22961 | 24685 | unavailable (receipt 1.0.0) |
| pico8 | all_normalized | 8 | complete | complete | True | 10 | 3354 | 7486 | unavailable (receipt 1.0.0) |
| pico8 | disabled | 8 | complete | complete | True | 0 | 0 | 4159 | unavailable (receipt 1.0.0) |
| pico8 | frame_full | 8 | complete | complete | True | 8 | 267504 | 138166 | unavailable (receipt 1.0.0) |
| pico8 | frame_hashes | 8 | complete | complete | True | 8 | 3808 | 7666 | unavailable (receipt 1.0.0) |
| pico8 | minimal | 8 | complete | complete | True | 2 | 666 | 5298 | unavailable (receipt 1.0.0) |
| pico8 | mixed | 8 | complete | complete | True | 18 | 7173 | 12737 | unavailable (receipt 1.0.0) |
| pico8 | selected_native | 8 | complete | complete | True | 1 | 529 | 5242 | unavailable (receipt 1.0.0) |
| pico8 | selected_normalized | 8 | complete | complete | True | 8 | 2688 | 7010 | unavailable (receipt 1.0.0) |
| pico8 | visual_capability | 8 | complete | complete | True | 17 | 6643 | 11669 | unavailable (receipt 1.0.0) |
| tic80 | all_normalized | 8 | complete | complete | True | 18 | 6740 | 10162 | unavailable (receipt 1.0.0) |
| tic80 | disabled | 8 | complete | complete | True | 0 | 0 | 4155 | unavailable (receipt 1.0.0) |
| tic80 | frame_full | 8 | complete | complete | True | 8 | 3427392 | 1608589 | unavailable (receipt 1.0.0) |
| tic80 | frame_hashes | 8 | complete | complete | True | 8 | 3679 | 7531 | unavailable (receipt 1.0.0) |
| tic80 | minimal | 8 | complete | complete | True | 2 | 652 | 5278 | unavailable (receipt 1.0.0) |
| tic80 | mixed | 8 | complete | complete | True | 26 | 10356 | 15314 | unavailable (receipt 1.0.0) |
| tic80 | selected_native | 8 | complete | complete | True | 1 | 405 | 5131 | unavailable (receipt 1.0.0) |
| tic80 | selected_normalized | 8 | complete | complete | True | 16 | 6086 | 9792 | unavailable (receipt 1.0.0) |
| tic80 | visual_capability | 8 | complete | complete | True | 25 | 9950 | 14357 | unavailable (receipt 1.0.0) |

`Evidence logical bytes` and `physical directory bytes` have different
measurement boundaries. The disabled rows still contain run envelope
metadata; they request no evidence channels. All rows preserve machine,
evidence, recorder, and publication outcomes as separate fields.
