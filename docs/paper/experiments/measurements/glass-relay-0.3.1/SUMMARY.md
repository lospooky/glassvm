# Glass Relay

One generic bounded Python consumer evaluates the same press/hold/release protocol on three machines.

The comparator receives input bindings and protocol coordinates, never machine/variant-specific evaluation logic.
Packages: {'glassvm': '0.3.1', 'glassvm-chip8': '0.3.1', 'glassvm-pico8': '0.3.1', 'glassvm-tic80': '0.3.1'}. CPython 3.12.13. 2 repetitions per cell.
Provider metadata inside these 0.3.1 distributions reports `bundle_version` 0.3.0; the package versions below identify the installed artifacts. Both values are retained as provenance, and this accepted metadata discrepancy does not invalidate this focused run.
This focused validation records working-tree revisions/dirty state; it is not a final sealed paper regeneration.
The same explicit current-state input is scheduled at every frame for all machines, covering both latched keys and frame-scoped gamepad masks.

| Machine | Cartridge | Protocol | Response delays | Restoration delays | Repeatable |
| --- | --- | --- | --- | --- | --- |
| chip8 | reference | pass | 0, 0, 0 | 0, 0, 0 | True |
| chip8 | delayed | fail | 3, 3, 3 | 0, 0, 0 | True |
| chip8 | stuck | fail | 0, —, — | —, —, — | True |
| pico8 | reference | pass | 0, 0, 0 | 0, 0, 0 | True |
| pico8 | delayed | fail | 3, 3, 3 | 0, 0, 0 | True |
| pico8 | stuck | fail | 0, —, — | —, —, — | True |
| tic80 | reference | pass | 0, 0, 0 | 0, 0, 0 | True |
| tic80 | delayed | fail | 3, 3, 3 | 0, 0, 0 | True |
| tic80 | stuck | fail | 0, —, — | —, —, — | True |

Controlled-variant matrix: PASS. The common protocol measurements match across all three machines for each variant.
These observations cover this task/configuration only; they are not general machine equivalence or execution-speed results.

Zero delay means a visible response at completion of the input-acceptance frame. All timings use declared semantic frames under the recorded configurations.
— means absent or unmeasurable. A stuck prior trial leaves later trials unarmed; its persistent marker is not misattributed to a new input.
Evidence/consumer failure or unexpected acceptance coordinates make the trial inconclusive. Recorder and publication outcomes remain independent.

[Comparison diagram](comparison.svg) · [Structured results and provenance](report.json)
