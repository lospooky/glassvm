# Glass Relay

Glass Relay demonstrates a downstream behavioral regression tester that uses
the same protocol and measurement code for CHIP-8, PICO-8, and TIC-80. The
experiment compares acceptance-to-visible-response delay, stable output while
held, and return to an established baseline after release.

The artifacts are small programs written for this experiment, not arbitrary
commercial games. Each machine draws its own marker in its own framebuffer
representation. The consumer compares fingerprints only within that run and
under one unchanged fingerprint schema. It does not compare digest values,
pixels, instruction counts, or wall-clock speed across machines.

## Trial protocol

The authoritative request data lives in [`experiment.json`](experiment.json).
There are 120 completed frames and three press/release windows: `[12,20)`,
`[44,57)`, and `[80,100)`. Frames before the first press establish a stable
baseline. The response and restoration deadlines are one semantic frame.

For every frame, the caller schedules the desired input value at frame start.
This common explicit input timeline works with both CHIP-8/PICO-8 latched
individual inputs and TIC-80's complete gamepad mask for one frame. A schedule
entry's ordinal equals its frame coordinate because this experiment constructs
one entry per frame; this is a local construction rule, not a GlassVM identity
or ordering rule. The manifest supplies concrete schema-qualified input IDs,
values, and construction configurations. The comparator contains no machine
or variant branches and never interprets native input bindings.

The frame in `InputApplied.application_coordinate` must match the requested
frame. Delay is the first changed completed frame minus the accepted press
frame. Zero means a response by completion of the acceptance frame. Restoration
uses the corresponding accepted release frame. These are trial-defined frame
measurements under the recorded machine configurations; CHIP-8 hardware timer
frames and the Lua callback frame descriptors remain distinct contracts.

The normalizer's `InputApplied` fact reports acceptance. `InputSampled` is not
used because its existing per-machine meanings differ. Correlation uses each
channel's coordinates and scheduled ordinal references, never cross-channel
callback order or `sequence` values. All input facts may arrive after the frame
channel without changing the result.

## Controls and interpretation

Every machine has three cartridges:

| Variant | Deliberate program behavior | Expected measurement |
| --- | --- | --- |
| reference | Show marker while held; clear on release | response 0, restoration 0, protocol passes |
| delayed | Wait three frame ticks before showing marker | response 3, restoration 0, response deadline fails |
| stuck | Keep marker after release | restoration absent; subsequent presses lack the baseline precondition |

The consumer does not receive the variant label. Only the runner uses labels
after evaluation to check the known controls. Later stuck trials are reported
as unarmed; a marker left over from an earlier trial is never credited as an
immediate response to a new press.

Missing/incomplete evidence, callback failure, changed fingerprint schemas,
frame gaps, or unexpected input acceptance prevent a supported protocol
judgment. Evidence uncertainty is reported as inconclusive, with machine,
evidence, Python delivery, recorder, and publication outcomes preserved.
No reference recorder is requested for this demo.

Two independent preparations/executions per cell compare trial measurements
and both prepared identities within each machine/variant. A separate comparison
checks the shared trial measurements across the three machines for each
variant. Preparation identities and fingerprint bytes are not expected to
match across machines. Identical results here concern only this common task,
the chosen configurations, and the recorded package builds.

## Bounded downstream state

The consumer retains a baseline and previous fingerprint, a bounded active
fingerprint per trial, at most eight trial accumulators, at most sixteen input
boundary facts, counters, bounded diagnostics, provenance, and one receipt.
It validates all per-frame input applications using a scalar cursor; it does
not retain the complete application stream. It retains no frame/event history
and never reads recorder files or execution internals. The plot uses the fixed
trial summaries: spans connect first response and restoration boundaries;
they do not reconstruct per-frame output history. Hold instability and missing
baseline preconditions are hatched.

The diagnostic prepared identity is supplied by `RunStarted` and retained once
for provenance. Its size follows the admitted request (including the explicit
input schedule), rather than the amount of emitted evidence. The observation
accumulators do not grow as additional frames/events arrive. Reports preserve
the supplied identities exactly using compact JSON; no identity is recomputed
from Python request syntax.

## Reproduce

From the Machines paper branch, rebuild the nine artifacts:

```sh
python3 conformance/paper/glass_relay/build_cartridges.py
```

From the GlassVM paper branch, run the focused demo using the locked package
environment:

```sh
uv run --locked --project docs/paper python docs/paper/experiments/glass_relay.py \
  --machines-root ../glassvm-machines --output-dir /tmp/glass-relay
```

The integrated regeneration stage creates the isolated paper environment and
enforces the existing clean-source/release gates:

```sh
python3 docs/paper/run_paper.py relay --machines-root ../glassvm-machines \
  --results-root /chosen/results
```

The `all` stage also includes Relay. The focused command writes `report.json`,
`SUMMARY.md`, and `comparison.svg`. It checks authored cartridge controls and
returns nonzero if their expected discrimination, repeatability, or common
protocol measurements do not hold. Detailed logical outcomes remain in the
report. Unit checks for attribution and uncertainty can run separately:

```sh
python3 -m unittest discover -s docs/paper/experiments -p test_glass_relay.py
```

The initial focused result, collected with the 0.3.0 packages, is in
[`measurements/glass-relay-0.3.0`](../measurements/glass-relay-0.3.0/SUMMARY.md).
It records a development run against those released wheels. The current
`0.3.1` package run is recorded separately under
[`measurements/glass-relay-0.3.1`](../measurements/glass-relay-0.3.1/SUMMARY.md).
