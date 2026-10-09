//! Python-facing sink contract types.
//!
//! This module defines the owned records and outcome vocabulary shared by
//! Python bundle bindings. The borrowed core emission API remains unchanged;
//! batching and PyO3 callback delivery are implemented by a later layer.

use glassvm_core::{
    CapabilityOutput, CapabilityReceipt, CommonMetrics, ContentDigest, Emission, EvidenceReceipt,
    ExecutionControls, ExecutionEvent, ExecutionRequest, FrameArtifact, InputSchedule,
    InputValueEvidence, MachineConfiguration, MachineId, NativeEvidenceEnvelope,
    ObservationRequest, RunId, RunResult, SchemaVersion, SnapshotArtifact,
};
use serde::{Deserialize, Serialize};

/// Metadata visible at run start. Artifact bytes are intentionally not copied
/// into the consumer record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunStartedRecord {
    pub schema_version: SchemaVersion,
    pub run_id: RunId,
    pub machine_id: MachineId,
    pub artifact_length_bytes: u64,
    pub prepared_observation_id: Option<ContentDigest>,
    pub configuration: MachineConfiguration,
    pub input_schedule: InputSchedule,
    pub observation: ObservationRequest,
    pub execution_controls: ExecutionControls,
}

/// Terminal machine metrics. Capability outputs are delivered through their
/// distinct callback and are not duplicated in this record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunFinishedRecord {
    pub common: CommonMetrics,
}

impl From<&RunResult> for RunFinishedRecord {
    fn from(result: &RunResult) -> Self {
        Self {
            common: result.common.clone(),
        }
    }
}

impl From<&ExecutionRequest> for RunStartedRecord {
    fn from(request: &ExecutionRequest) -> Self {
        Self {
            schema_version: request.schema_version,
            run_id: request.run_id.clone(),
            machine_id: request.machine_id.clone(),
            artifact_length_bytes: request.artifact.len() as u64,
            prepared_observation_id: request.prepared_observation_id.clone(),
            configuration: request.configuration.clone(),
            input_schedule: request.input_schedule.clone(),
            observation: request.observation.clone(),
            execution_controls: request.execution_controls.clone(),
        }
    }
}

/// Owned form of one core emission for eventual Python conversion.
///
/// This is a transport representation, not a collected-run model. Consumers
/// receive bounded slices of these records; no code here retains run history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "record", rename_all = "snake_case")]
pub enum OwnedEmission {
    RunStarted(RunStartedRecord),
    Event(ExecutionEvent),
    NativeEvidence(NativeEvidenceEnvelope),
    InputValueEvidence(InputValueEvidence),
    Frame(FrameArtifact),
    Snapshot(SnapshotArtifact),
    EvidenceReceipt(EvidenceReceipt),
    RunFinished(RunFinishedRecord),
}

impl<'a> From<Emission<'a>> for OwnedEmission {
    fn from(emission: Emission<'a>) -> Self {
        match emission {
            Emission::RunStarted(request) => Self::RunStarted(request.into()),
            Emission::Event(event) => Self::Event(event.clone()),
            Emission::NativeEvidence(evidence) => Self::NativeEvidence(evidence.clone()),
            Emission::InputValueEvidence(evidence) => Self::InputValueEvidence(evidence.clone()),
            Emission::Frame(frame) => Self::Frame(frame.clone()),
            Emission::Snapshot(snapshot) => Self::Snapshot(snapshot.clone()),
            Emission::EvidenceReceipt(receipt) => Self::EvidenceReceipt(receipt.clone()),
            Emission::RunFinished(result) => Self::RunFinished(result.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PythonSinkCallbackKind {
    EmissionBatch,
    CapabilityOutputs,
    CapabilityReceipts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PythonSinkFailure {
    pub sink_index: usize,
    pub sink_identity: Option<String>,
    pub callback: PythonSinkCallbackKind,
    pub batch_size: usize,
    pub successfully_delivered_items: usize,
    pub exception_type: String,
    pub exception_message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PythonSinkDeliveryStatus {
    NotRequested,
    Complete,
    Failed,
}

/// Separate accounting for Python consumer delivery. This is not evidence,
/// machine-execution, or reference-recorder status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PythonSinkDeliveryOutcome {
    pub status: PythonSinkDeliveryStatus,
    pub sink_count: usize,
    pub batches_delivered: u64,
    pub items_delivered: u64,
    pub logical_bytes_delivered: u64,
    pub failure: Option<PythonSinkFailure>,
}

impl PythonSinkDeliveryOutcome {
    pub const fn not_requested() -> Self {
        Self {
            status: PythonSinkDeliveryStatus::NotRequested,
            sink_count: 0,
            batches_delivered: 0,
            items_delivered: 0,
            logical_bytes_delivered: 0,
            failure: None,
        }
    }
}

/// Rust-side shape implemented by the PyO3 bridge. The actual batching and
/// Python exception conversion are deliberately deferred to the next slice.
pub trait PythonEmissionConsumer {
    fn emit_batch(&mut self, emissions: &[OwnedEmission]) -> Result<(), PythonSinkFailure>;

    fn record_capability_outputs(
        &mut self,
        outputs: &[CapabilityOutput],
    ) -> Result<(), PythonSinkFailure>;

    fn record_capability_receipts(
        &mut self,
        receipts: &[CapabilityReceipt],
    ) -> Result<(), PythonSinkFailure>;
}
