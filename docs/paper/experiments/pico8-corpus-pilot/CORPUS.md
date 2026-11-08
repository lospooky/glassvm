# PICO-8 autonomous candidate carts

This directory contains 30 reconstructed PICO-8 carts selected from the Fraser
PICO-8 Games Dataset. The manifest gives per-cart author, original BBS thread,
license, digest, and the 10-frame screening result. The dataset parquet digest
and deterministic screening rule are also recorded there.

Selection required a nonduplicate cart with `_init`, `_update`, and `_draw`, an
explicit CC BY-family license, no direct `btn`, `btnp`, or `stat` call in the
Lua source, successful completion of a 10-frame run on GlassVM's PICO-8 bundle,
and more than one distinct frame fingerprint. This is an operational screen for
autonomous visual candidates, not a representative sample of all PICO-8 carts.

Cart data was reconstructed from the dataset's Lua and graphics/audio/map
fields. These files may differ from original BBS artifacts. Each has a
separate CC BY-NC-SA 4.0 notice; see `carts/LICENSES.md`. Do not treat source
code license as proof of broader asset rights beyond the dataset's per-cart
license field.

The complete screening outcomes, including failures, are in `bbs-screen.json`.
