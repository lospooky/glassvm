# E1 F1: unsupported required capability

Three independent Python processes per machine submitted the canonical
`glassvm.paper.unsupported-capability` as a required capability during
preparation. All nine requests were rejected with `ValueError`, naming the
capability and stating that the normalizer catalog does not declare it. The
runner did not invoke execution after rejection, so the other outcome domains
were not started or not requested. This is a preparation-boundary result, not
a directly observed `RunStarted` emission test.

The nine raw JSON records contain exact artifact digests, configuration,
input schedule, controls, request, and rejection. `manifest.json` records
the per-trial outcomes; `provenance.json` records host, distribution versions,
binary and script hashes, source coordinates, and the exact command. Run with
Python 3.14 and the pinned published packages listed there. The runner uses
the committed CE1 fixture manifests and helpers from the sibling
`glassvm-machines` checkout. Choose a new empty output directory for a rerun.

This slice covers the required-capability part of F1 only. Unsupported
normalized and native evidence requests, F2, and the remaining E1 cases still
require their own per-machine experiment records.
