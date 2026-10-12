//! Emit a caller-selected logical projection from one finalized reference run.
//!
//! This is paper-validation tooling: decoding stays in the Rust recorder, and
//! the output is a finite projection chosen by the caller, not a run model.

use std::path::PathBuf;

use glassvm_recorder::{PublishedFileRun, RecordedRecord, ReferenceChannel};
use serde_json::{Map, Value, json};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let run_path = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: project_reference_run <published-run-directory> <output-json>")?;
    let output_path = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: project_reference_run <published-run-directory> <output-json>")?;
    if args.next().is_some() {
        return Err("expected exactly two arguments".into());
    }

    let run = PublishedFileRun::open(run_path)?;
    let channels = [
        ReferenceChannel::NormalizedEvents,
        ReferenceChannel::NativeEvidence,
        ReferenceChannel::InputValueEvidence,
        ReferenceChannel::Frames,
        ReferenceChannel::Snapshots,
        ReferenceChannel::CapabilityOutputs,
    ];
    let mut projection = Map::new();
    for channel in channels {
        let key = serde_json::to_value(&channel)?
            .as_str()
            .ok_or("channel did not serialize as a string")?
            .to_owned();
        let mut records = Vec::new();
        for segment_path in run.segment_paths(channel.clone())? {
            let mut reader = glassvm_recorder::FileSegmentReader::open(segment_path)?;
            while let Some(record) = reader.next_record()? {
                match record {
                    RecordedRecord::CapabilityOutput(output) => {
                        records.push(json!({
                            "schema": output.output_schema,
                            "value": output.payload,
                        }));
                    }
                    other => records.push(serde_json::to_value(other_record(other)?)?),
                }
            }
        }
        projection.insert(key, Value::Array(records));
    }

    let value = json!({
        "manifest": run.manifest(),
        "evidence_receipt": run.evidence_receipt(),
        "recorder_receipt": run.recorder_receipt(),
        "channels": projection,
    });
    std::fs::write(output_path, serde_json::to_vec_pretty(&value)?)?;
    Ok(())
}

fn other_record(record: RecordedRecord) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(match record {
        RecordedRecord::NormalizedEvent(value) => serde_json::to_value(value)?,
        RecordedRecord::NativeEvidence(value) => serde_json::to_value(value)?,
        RecordedRecord::InputValueEvidence(value) => serde_json::to_value(value)?,
        RecordedRecord::Frame(value) => serde_json::to_value(value)?,
        RecordedRecord::Snapshot(value) => serde_json::to_value(value)?,
        RecordedRecord::CapabilityOutput(_) => unreachable!("handled by caller"),
    })
}
