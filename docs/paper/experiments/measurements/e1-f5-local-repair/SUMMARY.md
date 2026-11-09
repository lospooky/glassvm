# E1 F5: external process interruption

Three fresh child processes per machine ran a file-backed session with an
approved synchronous consumer barrier at `RunFinished`. The parent killed
each child after observing its ready marker. All nine children exited due to
SIGKILL; staging files remained inspectable before temporary cleanup, while
no published run or returned result existed. The retained file inventories
showed no evidence or recorder receipt files at that boundary.

The parent observed filesystem state and process termination; the killed child
returned no execution or receipt outcome. Graceful cancellation is not exposed
by the current session interface and was not exercised. Raw records preserve
the requests, exit statuses, and staged file lists. See `manifest.json` for
source revisions and locally built wheel hashes.
