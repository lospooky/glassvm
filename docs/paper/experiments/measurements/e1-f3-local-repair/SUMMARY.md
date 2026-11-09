# E1 F3: Python consumer callback failures

For each of three machines, three independent processes injected a Python
`RuntimeError` at each of three callback boundaries: an event-containing batch,
capability outputs, and capability receipts. All 27 trials reached the intended
callback and recorded the Python delivery failure with the exception type and
message. Every trial reported machine execution interrupted by evidence failure,
evidence fulfillment failed, recorder not finalized, and publication not
attempted. The machine result was not classified as a machine fault.

Requests, outcomes, callback observations, and staging paths before cleanup
are retained in the per-trial JSON records. Recordings were removed after the
outcome and directory observations were captured. The Python bundles were
locally built against the repair revision; see `manifest.json` for source and
wheel identifiers. The observer runs synchronously and batching can delay the
callback until after machine work has occurred.
