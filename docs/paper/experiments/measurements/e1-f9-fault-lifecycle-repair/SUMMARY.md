# E1 F9 after machine fault lifecycle repair

Three independent Python processes per machine ran the same faulting programs
as the earlier F9 study using locally built machine wheels. All nine returned a
completed session with a structured `fault`, complete evidence, completed
Python delivery, finalized recorder, and successful publication. Each consumer
observed `run_started`, `run_finished`, and `evidence_receipt` exactly once.

CHIP-8 reported `chip8.invalid_opcode`; PICO-8 and TIC-80 reported
`pico8.callback_error` and `tic80.callback_error`, including the injected Lua
error message. The fault remains a machine result; completion describes the
session lifecycle and is not a claim that the program succeeded.

Raw requests, exact artifact bytes and digests, complete outcomes, emission
counts, locally built wheel hashes, and source revisions are in the records and
manifest. The published 0.3.2 base facade was used; machine extensions were
locally built from the repaired commits and were not released. The original
pre-repair F9 observations remain in `../e1-f9-local-repair/`.
