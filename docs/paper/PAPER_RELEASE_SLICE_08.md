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

The interpreter environments installed only published distributions; the conformance script and smoke fixtures were supplied from the checked-out machines repository. This verifies published package installation and execution, but is distinct from the no-checkout proof below. Temporary run outputs were written under `/tmp/glassvm-index-proof.ktb4qG` and are not paper measurement data.

## External consumer proofs

Both outstanding consumer gates passed on 2026-10-12 using published `0.2.0`
artifacts, with the consumer projects copied to fresh temporary directories.
The reusable project sources and lockfiles are in
[`experiments/external-consumer-proof/`](experiments/external-consumer-proof/).

### Python, no GlassVM checkout

The proof ran with CPython 3.12.13 and uv 0.12.1 on Linux x86_64. Its isolated
`pyproject.toml` depends only on `glassvm==0.2.0`; provider distributions were
installed separately from PyPI in fresh environments. No GlassVM or machine
source checkout was on the consumer import path. Public input artifacts were
downloaded directly from their pinned upstream GitHub revisions; the PICO-8
input applies the documented title-only normalization. All derived bytes
matched the workload manifest hashes:

| Machine | SHA-256 |
| --- | --- |
| CHIP-8 | `010f9ab220f55cc59645a6d05157b33a7099bac01037816ef06b74c2f8339b8a` |
| PICO-8 | `491888897d07e5699a5e411c9cb881181f313e6cad7b0dcfec2fd333ea0b833a` |
| TIC-80 | `f095a8ad534482a3aefbe0aa84c9b8bc47b74a89dd4202acbd5fc8b517995994` |

Each external run used installed `glassvm.machine_bundles` entry points,
prepared typed machine configuration plus an empty typed input schedule,
requested normalized events and hash frames, then used the shared Python
facade to execute and publish through the Rust recorder. Every provider run
completed execution, fulfilled evidence, finalized the recorder, and
atomically published. The tested fresh-environment selections were:

| Install selection | Discovery and execution result |
| --- | --- |
| `glassvm==0.2.0` | No providers discovered; base-only caller passed |
| Base + `glassvm-chip8==0.2.0` | CHIP-8 only; published run passed |
| Base + `glassvm-pico8==0.2.0` | PICO-8 only; published run passed |
| Base + `glassvm-tic80==0.2.0` | TIC-80 only; published run passed |
| Base + all three provider wheels | Exactly the three providers; all published runs passed |

The caller source, pinned fixture downloader, exact commands, and `uv.lock`
are retained in the external-consumer proof project. These checks validate
package consumption and execution, not performance.

### Rust, crates.io only

A separate Rust 2024 consumer project used exact `=0.2.0` dependencies on all
five published crates: `glassvm_normalizer_contract`, `glassvm_core`,
`glassvm_recorder`, `glassvm_query`, and `glassvm_registry`. Its `Cargo.lock`
records each GlassVM crate with the crates.io registry source; the manifest has
no path or Git dependencies. On rustc/cargo `1.97.1`, both `cargo check
--locked` and `cargo run --locked` passed from the temporary copy. The small
executable references public types from every crate. Its source and lockfile
are retained beside the Python proof.

## Scope boundary

The Slice 7b 60,000-frame Python-consumer matrix is measured evidence and is
documented separately; this installation proof makes no performance claim.
The full manuscript-wide regeneration and final immutable artifact seal remain
in Slice 9.

The release workflows build from clean GitHub runners. Machine CI rebuilds the provider sdists into wheels and exercises base-only, individual-provider, and all-provider installs for CPython 3.12, 3.13, and 3.14.
