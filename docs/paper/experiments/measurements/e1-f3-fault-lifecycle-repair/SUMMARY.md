# E1 F3 after machine fault lifecycle repair

The F3 Python consumer exception matrix was repeated using the local wheels
built for the guest-fault lifecycle repair. Three independent processes per
machine exercised errors in event batches, capability outputs, and capability
receipts. All 27 reached the intended callback and reported a structured
Python delivery failure; machine execution was not reported as a machine fault.

Raw requests, outcomes, callback observations, staged file inventories, source
revisions, and local-wheel digests are retained in the trial records and
manifest. The earlier F3 results remain under `../e1-f3-local-repair/`.
