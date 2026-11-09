# E1 F8-P: publication failure after recorder finalization

Three fresh processes per machine used a Python consumer to create a file at
the publication destination when `RunFinished` was delivered. In all nine
trials execution, evidence, and Python delivery completed; the recorder
finalized successfully, while publication reported `not_published` and no
published-run handle was returned. The destination was the collision file,
and both finalized receipt files were present in staging before cleanup.

Raw records preserve the requests, outcomes, callback trigger, and staged file
inventories. Temporary data was removed after inspection. Source revisions and
local-wheel hashes are in the manifest.
