use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    ContentDigest, EventKind, ExecutionControls, InputSchedule, MachineConfiguration, MachineId,
    ObservationRequest, RunId, SchemaVersion,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSelection {
    None,
    Lifecycle,
    Effects,
    Instructions,
    All,
    Kinds(BTreeSet<EventKind>),
}

impl EventSelection {
    pub fn includes(&self, kind: &EventKind) -> bool {
        match self {
            Self::None => false,
            Self::Lifecycle => matches!(
                kind,
                EventKind::RunStarted | EventKind::RunHalted | EventKind::RunCrashed
            ),
            Self::Effects => !matches!(
                kind,
                EventKind::StepStarted
                    | EventKind::InstructionDecoded
                    | EventKind::RegisterRead
                    | EventKind::MemoryRead
            ),
            Self::Instructions => !matches!(kind, EventKind::RegisterRead | EventKind::MemoryRead),
            Self::All => true,
            Self::Kinds(kinds) => kinds.contains(kind),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessDetail {
    None,
    AfterOnly,
    BeforeAndAfter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameCapture {
    None,
    Hashes,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotCapture {
    None,
    Final,
    EverySteps(u64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionRequest {
    pub schema_version: SchemaVersion,
    pub run_id: RunId,
    pub machine_id: MachineId,
    pub artifact: Vec<u8>,
    pub configuration: MachineConfiguration,
    pub input_schedule: InputSchedule,
    pub observation: ObservationRequest,
    pub execution_controls: ExecutionControls,
    /// Stable identity of the prepared observation contract used by the
    /// execution session. The negotiated object itself remains a preparation
    /// result; `RunStarted` carries this reference alongside the canonical
    /// request.
    #[serde(default)]
    pub prepared_observation_id: Option<ContentDigest>,
}

impl ExecutionRequest {
    pub fn new(
        run_id: impl Into<RunId>,
        machine_id: impl Into<MachineId>,
        artifact: Vec<u8>,
        configuration: MachineConfiguration,
        input_schedule: InputSchedule,
        observation: ObservationRequest,
        execution_controls: ExecutionControls,
    ) -> Result<Self, String> {
        observation.validate()?;
        configuration.validate_shape()?;
        input_schedule.validate_shape()?;
        Ok(Self {
            schema_version: SchemaVersion::V1,
            run_id: run_id.into(),
            machine_id: machine_id.into(),
            artifact,
            configuration,
            input_schedule,
            observation,
            execution_controls,
            prepared_observation_id: None,
        })
    }

    /// Validate the machine-independent execution envelope after any ingress,
    /// including direct deserialization that bypasses the constructor.
    pub fn validate_envelope(&self) -> Result<(), String> {
        if self.schema_version != SchemaVersion::V1 {
            return Err(format!(
                "unsupported execution-request schema {}; expected {}",
                self.schema_version,
                SchemaVersion::V1
            ));
        }
        if self.run_id.as_str().trim().is_empty() {
            return Err("execution request has an empty run ID".into());
        }
        if self.machine_id.as_str().trim().is_empty() {
            return Err("execution request has an empty machine ID".into());
        }
        self.configuration.validate_shape()?;
        self.input_schedule.validate_shape()?;
        self.observation.validate()?;
        let mut seen = BTreeSet::new();
        for limit in &self.execution_controls.bundle_limits {
            if !seen.insert(limit.limit_id.clone()) {
                return Err(format!(
                    "execution request repeats bundle limit {}",
                    limit.limit_id
                ));
            }
            limit.value.validate()?;
        }
        Ok(())
    }

    pub fn with_prepared_observation_id(mut self, identity: ContentDigest) -> Self {
        self.prepared_observation_id = Some(identity);
        self
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn configuration() -> MachineConfiguration {
        MachineConfiguration::new(
            crate::SchemaRef::new("test.configuration", SchemaVersion::V1),
            crate::StructuredValue::Map(BTreeMap::new()),
        )
        .unwrap()
    }

    fn request(observation: ObservationRequest) -> ExecutionRequest {
        ExecutionRequest::new(
            "run",
            "test",
            vec![0],
            configuration(),
            InputSchedule::empty(),
            observation,
            ExecutionControls::default(),
        )
        .unwrap()
    }

    #[test]
    fn canonical_observation_request_round_trips_inside_execution_request() {
        let request = request(ObservationRequest::debug());
        request.validate_envelope().unwrap();
        let encoded = serde_json::to_vec(&request).unwrap();
        let decoded: ExecutionRequest = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, request);
        assert!(decoded.observation.produces_complete_trace() == false);
    }

    #[test]
    fn execution_request_rejects_unknown_observation_fields() {
        let request = request(ObservationRequest::summary());
        let mut encoded = serde_json::to_value(&request).unwrap();
        encoded["unexpected_observation_field"] = serde_json::json!({"unexpected": true});
        assert!(serde_json::from_value::<ExecutionRequest>(encoded).is_err());
    }

    #[test]
    fn envelope_validation_remains_fail_closed() {
        let valid = request(ObservationRequest::debug());
        valid.validate_envelope().unwrap();

        let mut request = valid.clone();
        request.schema_version = SchemaVersion::new(2, 0, 0);
        assert!(request.validate_envelope().is_err());

        let mut request = valid;
        request.run_id = RunId::from("  ");
        assert!(request.validate_envelope().is_err());
    }
}
