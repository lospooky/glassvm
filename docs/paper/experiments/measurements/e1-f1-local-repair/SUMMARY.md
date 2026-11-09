# E1 F1 after event-kind negotiation repair

Using locally built bundle wheels linked against the repaired GlassVM sources,
three fresh processes per machine submitted each of three unsupported requests:
a required capability, a normalized extension kind, and a native kind. All 27
requests failed during preparation. One supported normalized/native selector
control per machine prepared successfully. No execution was invoked.

Raw records and `manifest.json` preserve the requests, outcomes, fixture
digests, source revisions, and wheel digests. The installed base facade was
the published GlassVM 0.3.2 distribution; the three locally built bundle
extensions retained their 0.3.3 package metadata but are not the published
0.3.3 wheels. Rebuild them against the recorded source revisions with local
Cargo overrides before reproducing the experiment.
