# Slice 5: selectable-observation measurements

Each cell reports the median of independent process repetitions. Raw per-run
observations remain authoritative in `raw/`; this table does not replace them.
All recipes use identical artifact/configuration/empty-schedule material per
machine and the same frame limit. Work is reported as the machine's returned
frame/cycle coordinates and termination; cycle units are machine-specific.
Evidence budgets are deliberately unbounded (all configured maxima are null).
The Python providers configure hard recorder limits of 1 MiB per block, 256
records per segment, 2 MiB per segment, and 1 MiB live buffer per channel.
A measurement is complete only when execution returns, evidence and recorder
receipts are complete, and publication succeeds. Any other outcome is retained
as an anomaly rather than treated as an ordinary overhead observation.

## Runtime, work, outcome, and total storage

The recipe ID names the exact request in `OBSERVATION_RECIPES.toml`; its
workload-specific resolved event/capability selectors are in the raw trial.
Artifact keys are the first 12 hex digits of the fixture SHA-256; full
digests and full GlassVM/machine source revisions remain in each raw trial.
At collection, the GlassVM checkout was marked dirty for experiment/docs
work and the machine checkout was clean; the exact flags are in every trial.

| Machine | Recipe/request ID | Artifact key | Source commits GVM/machines | n | Frames / cycles / termination | Evidence / recorder / publish | Prepare s median | Execute+publish s median | Worker s median | Peak RSS bytes median | Evidence items / logical bytes | Recorder records / logical bytes / encoded bytes | Blocks / segments / files | Physical directory bytes |
| --- | --- | --- | --- | ---: | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| chip8 | all_normalized | 010f9ab220f5 | 28b02ec0/1aca8f62 | 5 | 600 / 7200 / Timeout | complete / complete / True | 0.000450 | 1.134140 | 1.190057 | 31969280 | 18478 / 13647889 | 18478 / 14017449 / 10556621 | 73 / 73 / 76 | 10602549 |
| chip8 | disabled | 010f9ab220f5 | 28b02ec0/1aca8f62 | 5 | 600 / 7200 / Timeout | complete / complete / True | 0.000433 | 0.042913 | 0.096713 | 31436800 | 0 / 0 | 0 / 0 / 0 | 0 / 0 / 3 | 4216 |
| chip8 | frame_full | 010f9ab220f5 | 28b02ec0/1aca8f62 | 5 | 600 / 7200 / Timeout | complete / complete / True | 0.000455 | 1.268842 | 1.323986 | 32043008 | 600 / 19878399 | 600 / 19884399 / 10015498 | 29 / 10 / 13 | 10025754 |
| chip8 | frame_hashes | 010f9ab220f5 | 28b02ec0/1aca8f62 | 5 | 600 / 7200 / Timeout | complete / complete / True | 0.000453 | 0.068654 | 0.122636 | 31457280 | 600 / 280259 | 600 / 286259 / 214044 | 3 / 3 / 6 | 220141 |
| chip8 | minimal | 010f9ab220f5 | 28b02ec0/1aca8f62 | 5 | 600 / 7200 / Timeout | complete / complete / True | 0.000443 | 0.047136 | 0.100665 | 31498240 | 2 / 717 | 2 / 757 / 583 | 1 / 1 / 4 | 5613 |
| chip8 | mixed | 010f9ab220f5 | 28b02ec0/1aca8f62 | 5 | 600 / 7200 / Timeout | complete / complete / True | 0.000472 | 0.224324 | 0.279356 | 31924224 | 3170 / 1726621 | 3170 / 1784079 / 1364350 | 16 / 16 / 19 | 1379123 |
| chip8 | selected_native | 010f9ab220f5 | 28b02ec0/1aca8f62 | 5 | 600 / 7200 / Timeout | complete / complete / True | 0.000439 | 0.047238 | 0.101300 | 31391744 | 1 / 469 | 1 / 488 / 395 | 1 / 1 / 4 | 5451 |
| chip8 | selected_normalized | 010f9ab220f5 | 28b02ec0/1aca8f62 | 5 | 600 / 7200 / Timeout | complete / complete / True | 0.000423 | 0.188645 | 0.243795 | 31744000 | 2568 / 1443988 | 2568 / 1495348 / 1148388 | 11 / 11 / 14 | 1159230 |
| chip8 | visual_capability | 010f9ab220f5 | 28b02ec0/1aca8f62 | 5 | 600 / 7200 / Timeout | complete / complete / True | 0.000459 | 0.218404 | 0.273288 | 31797248 | 3169 / 1726152 | 3169 / 1783591 / 1363955 | 15 / 15 / 18 | 1377896 |
| pico8 | all_normalized | 491888897d07 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000222 | 0.311204 | 0.365139 | 31019008 | 602 / 206636 | 602 / 218676 / 168629 | 3 / 3 / 6 | 174814 |
| pico8 | disabled | 491888897d07 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000267 | 0.288674 | 0.342849 | 31096832 | 0 / 0 | 0 / 0 / 0 | 0 / 0 / 3 | 4218 |
| pico8 | frame_full | 491888897d07 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000261 | 1.500814 | 1.556117 | 32305152 | 600 / 20179846 | 600 / 20185846 / 10010160 | 20 / 10 / 13 | 10020271 |
| pico8 | frame_hashes | 491888897d07 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000240 | 0.317039 | 0.371183 | 31019008 | 600 / 289068 | 600 / 295068 / 224184 | 3 / 3 / 6 | 230281 |
| pico8 | minimal | 491888897d07 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000247 | 0.301522 | 0.355817 | 30978048 | 2 / 672 | 2 / 712 / 545 | 1 / 1 / 4 | 5581 |
| pico8 | mixed | 491888897d07 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000250 | 0.354199 | 0.409066 | 31322112 | 1202 / 496024 | 1202 / 514111 / 393189 | 8 / 8 / 11 | 403334 |
| pico8 | selected_native | 491888897d07 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000242 | 0.302358 | 0.357726 | 30998528 | 1 / 538 | 1 / 557 / 464 | 1 / 1 / 4 | 5524 |
| pico8 | selected_normalized | 491888897d07 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000236 | 0.313776 | 0.369089 | 30883840 | 600 / 205960 | 600 / 217960 / 168080 | 3 / 3 / 6 | 174329 |
| pico8 | visual_capability | 491888897d07 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000241 | 0.344284 | 0.400163 | 31105024 | 1201 / 495483 | 1201 / 513551 / 392723 | 7 / 7 / 10 | 402039 |
| tic80 | all_normalized | f095a8ad5344 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000222 | 0.160180 | 0.214526 | 30765056 | 1202 / 465694 | 1202 / 489734 / 374773 | 5 / 5 / 8 | 382091 |
| tic80 | disabled | f095a8ad5344 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000219 | 0.122572 | 0.176518 | 30511104 | 0 / 0 | 0 / 0 / 0 | 0 / 0 / 3 | 4214 |
| tic80 | frame_full | f095a8ad5344 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000202 | 9.910512 | 9.974338 | 36610048 | 600 / 257029067 | 600 / 257035067 / 120220554 | 300 / 150 / 153 | 120309467 |
| tic80 | frame_hashes | f095a8ad5344 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000202 | 0.178240 | 0.233227 | 30633984 | 600 / 278848 | 600 / 284848 / 213385 | 3 / 3 / 6 | 219483 |
| tic80 | minimal | f095a8ad5344 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000208 | 0.123999 | 0.178964 | 30564352 | 2 / 658 | 2 / 698 / 529 | 1 / 1 / 4 | 5561 |
| tic80 | mixed | f095a8ad5344 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000213 | 0.223204 | 0.277581 | 30908416 | 1802 / 745183 | 1802 / 775270 / 588464 | 10 / 10 / 13 | 599843 |
| tic80 | selected_native | f095a8ad5344 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000200 | 0.126526 | 0.181173 | 30584832 | 1 / 409 | 1 / 428 / 346 | 1 / 1 / 4 | 5410 |
| tic80 | selected_normalized | f095a8ad5344 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000208 | 0.161802 | 0.217255 | 30846976 | 1200 / 465030 | 1200 / 489030 / 374240 | 5 / 5 / 8 | 381711 |
| tic80 | visual_capability | f095a8ad5344 | 28b02ec0/1aca8f62 | 5 | 600 / 600 / frame_limit | complete / complete / True | 0.000218 | 0.228007 | 0.283642 | 30781440 | 1801 / 744771 | 1801 / 774839 / 588116 | 9 / 9 / 12 | 598664 |

## Per-channel accounting

Values are `items/evidence-logical-bytes/recorder-encoded-bytes`; recorder encoded bytes are physical-channel accounting, not the evidence receipt's logical size.

| Machine | Recipe | Normalized | Native | Frames | Capabilities | Snapshots | Input values |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| chip8 | all_normalized | 18478/13647889/10556621 | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| chip8 | disabled | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| chip8 | frame_full | 0/0/n/a | 0/0/n/a | 600/19878399/10015498 | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| chip8 | frame_hashes | 0/0/n/a | 0/0/n/a | 600/280259/214044 | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| chip8 | minimal | 2/717/583 | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| chip8 | mixed | 2568/1443988/1148388 | 1/469/395 | 600/281801/215178 | 1/363/389 | 0/0/n/a | 0/0/n/a |
| chip8 | selected_native | 0/0/n/a | 1/469/395 | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| chip8 | selected_normalized | 2568/1443988/1148388 | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| chip8 | visual_capability | 2568/1443988/1148388 | 0/0/n/a | 600/281801/215178 | 1/363/389 | 0/0/n/a | 0/0/n/a |
| pico8 | all_normalized | 602/206636/168629 | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| pico8 | disabled | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| pico8 | frame_full | 0/0/n/a | 0/0/n/a | 600/20179846/10010160 | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| pico8 | frame_hashes | 0/0/n/a | 0/0/n/a | 600/289068/224184 | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| pico8 | minimal | 2/672/545 | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| pico8 | mixed | 600/206115/168220 | 1/541/466 | 600/289223/224324 | 1/145/179 | 0/0/n/a | 0/0/n/a |
| pico8 | selected_native | 0/0/n/a | 1/538/464 | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| pico8 | selected_normalized | 600/205960/168080 | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| pico8 | visual_capability | 600/206115/168220 | 0/0/n/a | 600/289223/224324 | 1/145/179 | 0/0/n/a | 0/0/n/a |
| tic80 | all_normalized | 1202/465694/374773 | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| tic80 | disabled | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| tic80 | frame_full | 0/0/n/a | 0/0/n/a | 600/257029067/120220554 | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| tic80 | frame_hashes | 0/0/n/a | 0/0/n/a | 600/278848/213385 | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| tic80 | minimal | 2/658/529 | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| tic80 | mixed | 1200/465399/374333 | 1/412/348 | 600/279189/213572 | 1/183/211 | 0/0/n/a | 0/0/n/a |
| tic80 | selected_native | 0/0/n/a | 1/409/346 | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| tic80 | selected_normalized | 1200/465030/374240 | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a | 0/0/n/a |
| tic80 | visual_capability | 1200/465399/374333 | 0/0/n/a | 600/279189/213572 | 1/183/211 | 0/0/n/a | 0/0/n/a |

## Anomalies and interpretation

- None detected by the mechanical checks.

Runtime medians are descriptive for this host and these finite workloads;
the isolated-process high-water RSS includes interpreter/provider startup,
and is not an allocation-only or bounded-memory claim. Physical directory
bytes include recorder framing and metadata; evidence logical bytes do not.
