# E1 F6: evidence budget exhaustion

Each trial selected frame-completion events, bounded the normalized-event
count at one, and ran for two frames. Three independent processes per machine
exercised each overflow policy (18 trials).

All nine `fail` trials reported execution interrupted by evidence failure,
failed evidence and normalized channel, no recorder finalization, and no
publication. All nine `allow_incomplete` trials completed execution, reported
incomplete evidence and a truncated normalized channel, finalized recording,
and published. Each normalized channel accounted for one delivered event.
Raw trial records preserve complete receipts and file inventories before
temporary recording cleanup. The manifest identifies local wheel sources.
