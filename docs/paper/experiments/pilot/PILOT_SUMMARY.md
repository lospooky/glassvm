# Slice 4 pilot: accounting and lifecycle check

These short, eight-frame runs validate harness wiring and receipt separation;
they are not performance or bounded-memory evidence. Each run used the
published 0.1.2 Python distributions in an isolated Python 3.14 environment.
Recorder receipt schema 1.1.0 exposes per-channel persisted segment, record,
block, logical-byte, and encoded-byte counters; the harness validates these
against aggregate receipt totals without reading stored records.

| Machine | Recipe | Frames | Evidence | Recorder | Published | Evidence records | Evidence logical bytes | Physical directory bytes | Per-channel recorder counters |
| --- | --- | ---: | --- | --- | --- | ---: | ---: | ---: | --- |
| chip8 | all_normalized | 8 | complete | complete | True | 246 | 178051 | 143259 | available |
| chip8 | disabled | 8 | complete | complete | True | 0 | 0 | 4216 | available |
| chip8 | frame_full | 8 | complete | complete | True | 8 | 265016 | 138520 | available |
| chip8 | frame_hashes | 8 | complete | complete | True | 8 | 3723 | 7843 | available |
| chip8 | minimal | 8 | complete | complete | True | 2 | 711 | 5609 | available |
| chip8 | mixed | 8 | complete | complete | True | 44 | 23422 | 26614 | available |
| chip8 | selected_native | 8 | complete | complete | True | 1 | 461 | 5446 | available |
| chip8 | selected_normalized | 8 | complete | complete | True | 34 | 18877 | 20164 | available |
| chip8 | visual_capability | 8 | complete | complete | True | 43 | 22961 | 25392 | available |
| pico8 | all_normalized | 8 | complete | complete | True | 10 | 3354 | 7767 | available |
| pico8 | disabled | 8 | complete | complete | True | 0 | 0 | 4218 | available |
| pico8 | frame_full | 8 | complete | complete | True | 8 | 267504 | 138438 | available |
| pico8 | frame_hashes | 8 | complete | complete | True | 8 | 3808 | 7934 | available |
| pico8 | minimal | 8 | complete | complete | True | 2 | 666 | 5575 | available |
| pico8 | mixed | 8 | complete | complete | True | 18 | 7173 | 13655 | available |
| pico8 | selected_native | 8 | complete | complete | True | 1 | 529 | 5517 | available |
| pico8 | selected_normalized | 8 | complete | complete | True | 8 | 2688 | 7289 | available |
| pico8 | visual_capability | 8 | complete | complete | True | 17 | 6643 | 12373 | available |
| tic80 | all_normalized | 8 | complete | complete | True | 18 | 6740 | 10443 | available |
| tic80 | disabled | 8 | complete | complete | True | 0 | 0 | 4214 | available |
| tic80 | frame_full | 8 | complete | complete | True | 8 | 3427392 | 1608863 | available |
| tic80 | frame_hashes | 8 | complete | complete | True | 8 | 3679 | 7799 | available |
| tic80 | minimal | 8 | complete | complete | True | 2 | 652 | 5555 | available |
| tic80 | mixed | 8 | complete | complete | True | 26 | 10356 | 16233 | available |
| tic80 | selected_native | 8 | complete | complete | True | 1 | 405 | 5406 | available |
| tic80 | selected_normalized | 8 | complete | complete | True | 16 | 6086 | 10073 | available |
| tic80 | visual_capability | 8 | complete | complete | True | 25 | 9950 | 15062 | available |

`Evidence logical bytes` and `physical directory bytes` have different
measurement boundaries. The disabled rows still contain run envelope
metadata; they request no evidence channels. All rows preserve machine,
evidence, recorder, and publication outcomes as separate fields.
