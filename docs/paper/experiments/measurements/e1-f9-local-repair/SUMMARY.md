# E1 F9: controlled machine faults

Three isolated processes per machine prepared and executed a small faulting
program. CHIP-8 used an invalid opcode; PICO-8 and TIC-80 raised an explicit
Lua error in a frame callback. Each trial attached an online consumer and a
reference recorder. Program bytes and digests are in the raw records.

In all CHIP-8 trials the session returned `completed` with `InvalidOpcode`
termination and `boot_success = false`; evidence was complete and the recorder
finalized and published. In all PICO-8 and TIC-80 trials the session returned
`failed` with the injected runtime error; recorder finalization did not occur
and publication was not attempted. PICO-8 reported complete evidence, whereas
TIC-80 reported incomplete evidence; the bundles close their evidence paths
differently on this fault. Python consumer delivery completed in all nine runs.
These are distinct machine/session outcomes, not claims of equivalent faults.

Raw requests, outcomes, partial emission counts, staged file inventories,
source revisions, and locally built wheel hashes are retained alongside this
summary. Temporary recordings were removed after their file lists were captured.
