use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest as _, Sha256};

use crate::{ExecutionEvent, RunResult, SchemaRef, SchemaVersion};

const CANONICAL_JSON_DOMAIN: &str = "glassvm.canonical_json.v1";
const STATE_DOMAIN: &str = "glassvm.final_state.v1";
const EVENT_PROJECTION_DOMAIN: &str = "glassvm.event_projection.v1";
const REPORT_DOMAIN: &str = "glassvm.run_result.v1";

pub const REPORT_SCHEMA_VERSION: SchemaVersion = SchemaVersion::V1;
pub const EVENT_PROJECTION_FINGERPRINT_SCHEMA_VERSION: SchemaVersion = SchemaVersion::V1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DigestAlgorithm {
    Sha256,
}

/// Algorithm-tagged content digest used for storage-neutral identity material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentDigest {
    pub algorithm: DigestAlgorithm,
    pub value: [u8; 32],
}

impl ContentDigest {
    /// Standard SHA-256 over caller-supplied bytes.
    pub fn sha256(bytes: &[u8]) -> Self {
        Self {
            algorithm: DigestAlgorithm::Sha256,
            value: Sha256::digest(bytes).into(),
        }
    }

    pub fn to_hex(self) -> String {
        let mut output = String::with_capacity(64);
        for byte in self.value {
            use fmt::Write as _;
            write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
        }
        output
    }

    fn from_hasher(hasher: Sha256) -> Self {
        Self {
            algorithm: DigestAlgorithm::Sha256,
            value: hasher.finalize().into(),
        }
    }
}

impl fmt::Display for ContentDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "sha256:")?;
        for byte in self.value {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Identity of caller-supplied artifact bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactIdentity {
    pub byte_len: u64,
    pub digest: ContentDigest,
}

impl ArtifactIdentity {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            byte_len: bytes.len() as u64,
            digest: ContentDigest::sha256(bytes),
        }
    }

    pub fn matches(&self, bytes: &[u8]) -> bool {
        self == &Self::from_bytes(bytes)
    }
}

/// Serialize a value using GlassVM's deterministic JSON representation.
///
/// This helper produces identity material only. It does not define a recorder
/// format and does not retain or acquire execution history.
pub fn canonical_json_bytes<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, String> {
    let value = serde_json::to_value(value)
        .map_err(|error| format!("cannot convert value to canonical JSON: {error}"))?;
    let mut output = Vec::new();
    write_canonical_value(&value, &mut output)?;
    Ok(output)
}

/// Domain-separated SHA-256 over canonical JSON identity material.
pub fn canonical_json_fingerprint<T: Serialize + ?Sized>(
    domain: &str,
    value: &T,
) -> Result<ContentDigest, String> {
    if domain.is_empty() {
        return Err("fingerprint domain must not be empty".into());
    }
    let bytes = canonical_json_bytes(value)?;
    let mut hasher = domain_hasher(CANONICAL_JSON_DOMAIN);
    update_segment(&mut hasher, domain.as_bytes());
    update_segment(&mut hasher, &bytes);
    Ok(ContentDigest::from_hasher(hasher))
}

fn write_canonical_value(value: &Value, output: &mut Vec<u8>) -> Result<(), String> {
    match value {
        Value::Null => output.extend_from_slice(b"null"),
        Value::Bool(value) => output.extend_from_slice(if *value { b"true" } else { b"false" }),
        Value::Number(value) => output.extend_from_slice(value.to_string().as_bytes()),
        Value::String(value) => {
            let encoded = serde_json::to_string(value)
                .map_err(|error| format!("cannot encode canonical JSON string: {error}"))?;
            output.extend_from_slice(encoded.as_bytes());
        }
        Value::Array(values) => {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                write_canonical_value(value, output)?;
            }
            output.push(b']');
        }
        Value::Object(values) => {
            output.push(b'{');
            let mut keys: Vec<&str> = values.keys().map(String::as_str).collect();
            keys.sort_unstable();
            for (index, key) in keys.into_iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                let encoded = serde_json::to_string(key)
                    .map_err(|error| format!("cannot encode canonical JSON key: {error}"))?;
                output.extend_from_slice(encoded.as_bytes());
                output.push(b':');
                write_canonical_value(
                    values
                        .get(key)
                        .expect("key collected from the same JSON object"),
                    output,
                )?;
            }
            output.push(b'}');
        }
    }
    Ok(())
}

fn domain_hasher(domain: &str) -> Sha256 {
    let mut hasher = Sha256::new();
    update_segment(&mut hasher, b"glassvm.sha256.domain.v1");
    update_segment(&mut hasher, domain.as_bytes());
    hasher
}

fn update_segment(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

/// Identity of one caller-selected normalized-event projection.
///
/// The projection is supplied by the caller. Core hashes each supplied event
/// while omitting only its run ID, and retains no event collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventProjectionFingerprint {
    pub schema: SchemaRef,
    pub event_count: u64,
    pub fingerprint: ContentDigest,
}

impl EventProjectionFingerprint {
    pub fn from_events(events: &[ExecutionEvent]) -> Result<Self, String> {
        let mut accumulator = EventProjectionFingerprintAccumulator::new();
        for event in events {
            accumulator.update(event)?;
        }
        Ok(accumulator.finish())
    }
}

/// Incremental identity helper for a caller-supplied event projection.
///
/// This is an identity accumulator, not a trace buffer: it stores only the
/// digest state and event count.
pub struct EventProjectionFingerprintAccumulator {
    hasher: Sha256,
    event_count: u64,
}

impl EventProjectionFingerprintAccumulator {
    pub fn new() -> Self {
        Self {
            hasher: domain_hasher(EVENT_PROJECTION_DOMAIN),
            event_count: 0,
        }
    }

    pub fn update(&mut self, event: &ExecutionEvent) -> Result<(), String> {
        let mut value = serde_json::to_value(event)
            .map_err(|error| format!("cannot serialize event projection record: {error}"))?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| "execution event did not serialize as an object".to_string())?;
        object.remove("run_id");
        update_segment(&mut self.hasher, &canonical_json_bytes(&value)?);
        self.event_count = self
            .event_count
            .checked_add(1)
            .ok_or_else(|| "event projection record count overflow".to_string())?;
        Ok(())
    }

    pub fn event_count(&self) -> u64 {
        self.event_count
    }

    pub fn finish(mut self) -> EventProjectionFingerprint {
        update_segment(&mut self.hasher, b"event_count");
        self.hasher.update(self.event_count.to_be_bytes());
        EventProjectionFingerprint {
            schema: SchemaRef::new(
                "glassvm.event_projection_fingerprint",
                EVENT_PROJECTION_FINGERPRINT_SCHEMA_VERSION,
            ),
            event_count: self.event_count,
            fingerprint: ContentDigest::from_hasher(self.hasher),
        }
    }
}

impl Default for EventProjectionFingerprintAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

pub fn event_projection_fingerprint(
    events: &[ExecutionEvent],
) -> Result<EventProjectionFingerprint, String> {
    EventProjectionFingerprint::from_events(events)
}

/// Strong fingerprint over a machine-owned state codec and its exact bytes.
pub fn state_fingerprint(state_schema: &SchemaRef, bytes: &[u8]) -> ContentDigest {
    let mut hasher = domain_hasher(STATE_DOMAIN);
    update_segment(&mut hasher, state_schema.id.as_bytes());
    hasher.update(state_schema.version.major.to_be_bytes());
    hasher.update(state_schema.version.minor.to_be_bytes());
    hasher.update(state_schema.version.patch.to_be_bytes());
    update_segment(&mut hasher, bytes);
    ContentDigest::from_hasher(hasher)
}

/// Canonical semantic fingerprint for a supplied run-result projection.
pub fn report_fingerprint(result: &RunResult) -> Result<ContentDigest, String> {
    let mut capabilities = Map::new();
    for capability in &result.capabilities {
        if capabilities
            .insert(
                capability.schema.id.as_str().to_owned(),
                capability.value.clone(),
            )
            .is_some()
        {
            return Err(format!(
                "run result contains duplicate capability ID {:?}",
                capability.schema.id
            ));
        }
    }
    let normalized = json!({
        "schema": SchemaRef::new("glassvm.run_result", REPORT_SCHEMA_VERSION),
        "common": result.common,
        "capabilities": capabilities,
    });
    canonical_json_fingerprint(REPORT_DOMAIN, &normalized)
}

impl fmt::Display for EventProjectionFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} ({})", self.fingerprint, self.event_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventContext, EventKind, MachineId, RunId, VersionStamp};

    fn event(run_id: &str, sequence: u64) -> ExecutionEvent {
        ExecutionEvent::from_context(
            &EventContext {
                arch: MachineId::from("test"),
                machine_version: VersionStamp::from("test.v1"),
                run_id: RunId::from(run_id),
                sequence,
                step: sequence,
                cycle_or_tick: Some(sequence),
                frame: Some(0),
                pc: None,
                instruction: None,
            },
            EventKind::StepStarted,
        )
    }

    #[test]
    fn event_projection_fingerprint_is_run_neutral_and_incremental() {
        let first = [event("run-a", 0), event("run-a", 1)];
        let second = [event("run-b", 0), event("run-b", 1)];
        assert_eq!(
            event_projection_fingerprint(&first).unwrap(),
            event_projection_fingerprint(&second).unwrap()
        );

        let mut accumulator = EventProjectionFingerprintAccumulator::new();
        for value in &first {
            accumulator.update(value).unwrap();
        }
        assert_eq!(
            accumulator.finish(),
            event_projection_fingerprint(&first).unwrap()
        );
    }
}
