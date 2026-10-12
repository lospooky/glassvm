# Paper release slice 8: packages and independent installation

Execution date: 2026-10-12.

## Published coordinates

GlassVM `v0.2.0` completed successfully in [release workflow 38247046904](https://github.com/lospooky/glassvm/actions/runs/38247046904), at source commit `33161d1e7ed9617ee46727a9c88d2054193bc2e1`. It published the five machine-neutral crates (`glassvm_normalizer_contract`, `glassvm_core`, `glassvm_recorder`, `glassvm_query`, and `glassvm_registry`) and the generic `glassvm` Python facade.

Machine `v0.2.0` completed successfully in [release workflow 38250729717](https://github.com/lospooky/glassvm-machines/actions/runs/38250729717), at source commit `0aefbb950c451b4d54de85e8c13c25345065ae92`. Tag CI [38250729742](https://github.com/lospooky/glassvm-machines/actions/runs/38250729742) also passed. Published Python distributions are:

- `glassvm-chip8==0.2.0`
- `glassvm-pico8==0.2.0`
- `glassvm-tic80==0.2.0`
- `glassvm-machines==0.2.0`

Each provider has an sdist and CPython 3.12, 3.13, and 3.14 wheels. The metadata-only selector has an sdist and universal wheel. The machine lockfile resolves GlassVM crates from crates.io; machine-specific Rust crates remain unpublished.

After publication, released product changes were synchronized into both
`paper-publication` branches without moving paper artifacts to `main`. GlassVM
branch commit `a7dd37498d681cd9c97679ac30c6a22a7acb311e` passed
[CI 38252407335](https://github.com/lospooky/glassvm/actions/runs/38252407335);
machines branch commit `3edb0ae31a4a3887a1f385d9d7b9aa50f7a9b026` passed
[CI 38252506914](https://github.com/lospooky/glassvm-machines/actions/runs/38252506914).
The experiment results/generators and `conformance/examples/paper_memory_trial.rs`
remain tracked on their respective paper branches.

The first tag attempt exposed a workflow issue: publisher jobs did not check out the repository containing the retry-safe PyPI file-selection helper. All upload steps failed before publishing. After confirming that all four machine distributions still returned 404 for `0.2.0`, commit `0aefbb9` added source checkout to both publisher jobs. The `v0.2.0` tag was then moved to that fix commit and the full release succeeded. No partial files from the failed attempt needed recovery.

## Package-index smoke

Fresh Python 3.12.13 virtual environments installed directly from PyPI and passed `conformance/python_clean_room.py` for:

| Install selection | Result |
| --- | --- |
| `glassvm==0.2.0` only | Passed; no providers discovered |
| `glassvm-chip8==0.2.0` | Passed; CHIP-8 only |
| `glassvm-pico8==0.2.0` | Passed; PICO-8 only; invalid artifact rejected during preparation |
| `glassvm-tic80==0.2.0` | Passed; TIC-80 only; invalid artifact rejected during preparation |
| `glassvm-machines[all]==0.2.0` | Passed; all three discovered and executed |

The installed providers completed the same `Runtime.discover -> prepare -> execute` lifecycle, delivered Python emissions, finalized the Rust file recorder, and atomically published the run with separate machine, evidence, Python sink, recorder, and publication outcomes. CHIP-8 was not included in the invalid-artifact rejection assertion because its declared admission contract accepts arbitrary artifact bytes.

The interpreter environments installed only published distributions; the conformance script and smoke fixtures were supplied from the checked-out machines repository. Thus this verifies published package installation and execution, but it does not close the separate zero-checkout consumer-project proof. Temporary run outputs were written under `/tmp/glassvm-index-proof.ktb4qG` and are not paper measurement data.

## Remaining packaging gates

- Build a minimal external Rust consumer against published crate versions without local paths.
- Run a Python consumer project with no GlassVM repository checkout, supplying fixtures independently.
- Keep the Slice 7b 60,000-frame Python-consumer performance matrix open; this smoke is packaging validation, not performance evidence.

The release workflows build from clean GitHub runners. Machine CI rebuilds the provider sdists into wheels and exercises base-only, individual-provider, and all-provider installs for CPython 3.12, 3.13, and 3.14.
