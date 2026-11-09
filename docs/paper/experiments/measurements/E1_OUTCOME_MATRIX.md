# E1 outcome matrix

This is a factual index to the per-case trials on the repaired source. Each
applicable machine was exercised in three independent processes. The F7 row
reuses F6's nine `allow_incomplete` trials; F2-D exercises a synthetic
capability diamond at preparation and does not execute a synthetic normalizer.
The earlier F1 accepted-selector anomaly is retained separately.

| Case | Machine execution | Evidence fulfillment | Python delivery | Recorder | Publication |
| --- | --- | --- | --- | --- | --- |
| F1 unsupported capability / normalized / native kind | Not started: preparation rejected (27 trials) | Not started | Not requested | Not started | Not attempted |
| F2 optional missing prerequisite | Completed | Complete; optional capability unavailable with reason | Not requested | Finalized | Published |
| F2-D dependency diamond | Not started: preparation only | Not started | Not requested | Not requested | Not requested |
| F3 Python event/output/receipt callback error | Interrupted by evidence failure | Failed | Failed | Not finalized | Not attempted |
| F4 rejecting Rust event consumer | Session returned the sink error; no `FileRunResult` was constructed | Failed receipt | Not requested | Not requested | Not requested |
| F5 process kill at `RunFinished` | No returned outcome | No returned receipt | No returned outcome | No returned receipt; staging present | No published run |
| F6 `fail` budget | Interrupted by evidence failure | Failed | Not requested | Not finalized | Not attempted |
| F6 `allow_incomplete` / F7 | Completed | Incomplete; normalized channel truncated | Not requested | Finalized; recorder receipt complete | Published |
| F8-C recorder creation | Completed | Complete | Not requested | Failed | Not attempted |
| F8-W recorder block limit | Completed | Complete | Not requested | Failed | Not attempted |
| F8-F receipt-path collision | Completed | Complete | Complete | Failed | Not attempted |
| F8-P publication destination collision | Completed | Complete | Complete | Finalized | Not published |
| F9 recognized guest faults, after repair | Completed with a structured fault | Complete | Complete | Finalized | Published |
| F10 zero-frame request | Completed | Complete; requested frame channel has zero items | Not requested | Finalized | Published |

## Receipt and scope notes

- F2's [raw receipts](e1-f2-local-repair/SUMMARY.md) include each unavailable
  capability's reason. Python prepared state is opaque; Rust conformance checks
  prospective unavailability.
- F6's [raw receipts](e1-f6-local-repair/SUMMARY.md) preserve configured
  budgets, affected channel, counters, and overflow status. F7's
  [check](e1-f7-local-repair.json) verifies recorder/evidence receipt status and
  both receipt files before cleanup; recordings were not retained for later
  decoding.
- F2-D uses test catalog entries over real bundle input catalogs. F4 exercises
  the public Rust sink/session boundary without a file recorder. F5 reports
  only what its supervising process could observe after killing the child.
- F8-F uses an isolated filesystem collision during recorder finalization; the
  previously approved private test switch remains separately covered by source
  tests. F8-P tests a rename collision, not a post-rename sync warning.
- F10 compares a zero-item frame channel with F6's separately truncated
  normalized channel. It does not test silent omission or expected-count
  detection, and a zero count does not prove a behavior did not occur.
- F9 uses deliberately faulting programs; its machine-specific termination
  and evidence statuses do not establish equivalent machine behavior.
- Before the fault-lifecycle repair, CHIP-8 returned a completed session with
  `InvalidOpcode`, while PICO-8 and TIC-80 returned failed sessions; their
  evidence statuses were respectively complete and incomplete. Those original
  observations remain in `e1-f9-local-repair/`. The rerun in
  `e1-f9-fault-lifecycle-repair/` uses locally built repaired wheels.
- Exploratory single-run pilots were performed before the F2, F3, F8-C,
  F8-F, F9, and F10 matrices; their console output was not retained as
  structured trial artifacts. F9's machine-specific acceptance checks were
  finalized after inspecting code and pilot outcomes, so they are not an
  independently preregistered confirmation of those differences.
- The F9 pilot exposed differing machine/evidence outcomes. The implementation
  was inspected and the measured cases were then run, but the requested pause
  for author review did not occur at that point. Treat those differences as
  observations requiring review, not as a preregistered equivalence result.
- These locally built bundle wheels share 0.3.3 metadata with the published
  packages but are different artifacts; source and wheel digests are retained
  in each manifest. Full paper-suite regeneration remains pending.
