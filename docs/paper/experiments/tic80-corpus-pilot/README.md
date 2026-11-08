# TIC-80 autonomous cart screening pilot

This is an exploratory compatibility screen for candidate TIC-80 workloads, not
a representative corpus and not a paper result. It used the installed
`glassvm==0.3.1` and `glassvm-tic80==0.3.1` provider on this WSL2 host.

## Result

The fixed candidate pool contains 180 carts drawn from the TIC-80 entries in
the [Libretro TIC-80 catalog](https://github.com/libretro/libretro-database/blob/master/dat/TIC-80.dat),
which links to individual carts on the [TIC-80 website](https://tic80.com/).
The recorded catalog version and digest, candidate metadata, download URLs,
per-trial outcomes, and artifact digests are in `candidate-set.json` and
`results.json`.

Each cart was requested to run for ten frames with an empty input schedule,
`cycles_per_frame = 1`, Lua runtime, machine seed zero, and frame hashes only.
A candidate passed the runtime screen if preparation succeeded, execution
completed, the frame evidence receipt was complete, all ten frames arrived,
and at least two frame fingerprints differed. The source stored in each cart
was also scanned for direct `btn`, `btnp`, `key`, `keyp`, `mouse`, or `pmem`
calls after removing line comments. That scan is a heuristic, not proof that a
program is independent of every possible input path.

The candidate pool yielded 62 runtime passes, 33 with no direct input API call
matched by the scan after excluding titles that indicate music or an obvious
derivative. The first 30 in title/page order were retained. The exact selection
and per-cart results are in `manifest.json` and `results.json`.

## Rights and retention

The TIC-80 catalog provides public cart pages and direct downloads; it does not
provide a license field. Five selected carts contain a license statement in
their embedded source, three contain template/ambiguous license wording, and
22 contain no license statement found by the pilot. A source comment is not by
itself a complete rights review. Therefore the downloaded `.tic` files are in
`local-only-carts/`, which is Git-ignored. Do not commit or redistribute these
files until each cart's license and any bundled assets have been checked.
`manifest.json` preserves author, source page, download URL, MD5, SHA-256, and
the embedded license marker/status so the rights review can be completed.

The screen can be rerun using the command below. It downloads the candidate
files from their catalog URLs, checks each downloaded MD5 against the catalog,
executes each candidate in a fresh process, and writes the trial outcomes and
selected local carts.

```sh
docs/paper/.venv/bin/python docs/paper/experiments/tic80-corpus-pilot/pilot.py
```

This screen establishes only that these 30 carts ran under this bundle's
current Lua compatibility subset for ten frames and produced changing frame
hashes. It does not establish upstream TIC-80 fidelity, long-run stability,
behavioral representativeness, or a usable campaign corpus.
