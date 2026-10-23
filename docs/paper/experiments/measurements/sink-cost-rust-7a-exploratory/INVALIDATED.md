# Superseded exploratory run

Do not use these measurements for Slice 7a. The trial `run_id` included the
observation mode, so its differing string length changed per-record logical
byte accounting between otherwise equivalent NullSink and recorder requests.
The Rust trial now uses the same run ID across modes, and the corrected matrix
is stored in the sibling `sink-cost-rust-7a-corrected` directory.
