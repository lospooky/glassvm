# E1 F10: zero items and truncation

Three fresh processes per machine requested frame hashes with a zero-frame
limit. All nine sessions completed; each receipt marked the requested frame
channel complete with zero items, and each recorder finalized and published.

For comparison, F6's `allow_incomplete` trials selected frame-completion
events with a one-event budget and recorded truncated normalized channels and
incomplete evidence. These are different requests and channels; the comparison
shows accounting statuses, not behavioral equivalence. Neither zero items nor
a complete receipt establishes whether a behavior would have occurred under a
different execution. No expected-count or silent-drop detection was tested.

Raw requests, outcomes, receipts, pre-cleanup file lists, source coordinates,
and local wheel hashes are retained in the records and manifest.
