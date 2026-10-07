pub mod bundle;
pub mod canonical;
pub mod configuration;
pub mod emission;
pub mod event;
pub mod identity;
pub mod input;
pub mod live_input;
pub mod normalizer;
pub mod observation;
pub mod observation_contract;
pub mod preparation;
pub mod replay;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use bundle::{
    AnalyzeResult, ArtifactEncoding, MachineArtifactSpec, MachineBundle, MachineContract,
    MachineDescriptor, MachineSemantics, StaticAnalyzerBackend,
};
pub use canonical::{StructuredValue, canonical_cbor_bytes, canonical_schema_ref_bytes};
pub use configuration::{
    BundleExecutionLimit, EXECUTION_LIMIT_CATALOG_SCHEMA_VERSION, ExecutionControls,
    ExecutionLimitCatalog, ExecutionLimitDescriptor, MachineConfiguration, PreparedConfiguration,
    PreparedExecutionControls, TypedStructuredValue, execution_limit_catalog_schema,
};
pub use emission::{
    BudgetedSink, Emission, EmissionSink, FRAME_ARTIFACT_SCHEMA_VERSION, FrameArtifact,
    FrameEvidence, NullSink, SinkError, SnapshotArtifact,
};
pub use event::{
    Address, CausalLink, CausalRelation, ControlFlow, ControlFlowKind, EVENT_SCHEMA_VERSION,
    EventContext, EventKind, ExecutionEvent, InstructionRef, IoChannel, IoDirection, IoObservation,
    NativeEvent, StateLocation, StateRead, StateSpace, StateWrite, TrapInfo,
};
pub use glassvm_normalizer_contract::{
    CAPABILITY_SCHEMA_VERSION, CapabilityDependency, CapabilityDescriptor, CapabilityId,
    CapabilityOutput, CapabilityReceipt, CapabilityRequest, CapabilitySchema, CapabilityStatus,
    CostClass, FrameCaptureRequirement, InputValueSelector, NORMALIZER_CONTRACT_SCHEMA_VERSION,
    NormalizedAccessRequirement, NormalizerCatalog, SchemaVersion as NormalizerSchemaVersion,
    SnapshotRequirement, standard_capabilities,
};
pub use identity::{
    ExecutionLimitId, InputId, InputInstanceId, MachineId, RunId, SchemaFamilyId, SchemaRef,
    SchemaVersion, VersionStamp,
};
pub use input::{
    CoordinatePolicy, ExecutionCoordinateCatalog, FrameBoundaryKind, FrameCoordinateDescriptor,
    InputApplicationSemantics, InputApplied, InputBoundaryKind, InputCatalog, InputCoordinate,
    InputDeliveryMode, InputSchedule, InputSource, InputSpec, InputValueEvidence,
    ResolvedInputSchedule, ScheduledInput, StepCoordinateDescriptor, StepUnit, TypedInputPayload,
    cycle_coordinate_schema, frame_coordinate_schema, input_catalog_schema, input_schedule_schema,
    step_coordinate_schema, tick_coordinate_schema,
};
pub use live_input::{
    LiveInputAcceptance, LiveInputAdmission, LiveInputCapability, LiveInputController,
    LiveInputError, create_live_input_controller,
};
pub use normalizer::{
    Normalizer, NormalizerDriver, NormalizerError, NormalizerOutput, NormalizerRun,
};
pub use observation::{
    AccessDetail, EventSelection, ExecutionRequest, FrameCapture, SnapshotCapture,
};
pub use observation_contract::{
    BudgetKind, BudgetReceipt, ChannelReceipt, ChannelStatus, EMISSION_SCHEMA_VERSION,
    EmissionBudgets, EmissionChannel, EmissionFailure, EvidenceGuarantee, EvidenceReceipt,
    EvidenceStatus, FrameRequest, InputValueEvidenceRequest,
    NATIVE_EVIDENCE_ENVELOPE_SCHEMA_VERSION, NativeEvidenceEnvelope, NativeEvidenceRequest,
    NormalizedEventRequest, ObservationRequest, OverflowPolicy,
    PREPARED_OBSERVATION_SCHEMA_VERSION, PreparedCapability, PreparedCapabilityStatus,
    PreparedNormalizer, PreparedObservation, SnapshotRequest,
};
pub use preparation::{
    MachineIdentity, PREPARED_RUN_IDENTITY_SCHEMA_VERSION, PreparedArtifactIdentity,
    PreparedInputInterface, PreparedLiveInput, PreparedRun, PreparedRunIdentity,
    prepared_run_identity_schema,
};
pub use replay::{
    ArtifactIdentity, ContentDigest, DigestAlgorithm, EVENT_PROJECTION_FINGERPRINT_SCHEMA_VERSION,
    EventProjectionFingerprint, EventProjectionFingerprintAccumulator, REPORT_SCHEMA_VERSION,
    canonical_json_bytes, canonical_json_fingerprint, event_projection_fingerprint,
    report_fingerprint, state_fingerprint,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigField {
    pub key: String,
    pub label: String,
    pub description: String,
    pub kind: ConfigFieldKind,
    pub default_value: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConfigFieldKind {
    Bool,
    Int { min: i64, max: i64, step: i64 },
    Float { min: f64, max: f64, step: f64 },
    Enum { options: Vec<String> },
    String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineConfigSchema {
    pub schema: SchemaRef,
    pub fields: Vec<ConfigField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommonMetrics {
    pub cycles: u64,
    pub frames: u64,
    pub termination: String,
    pub boot_success: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunResult {
    pub common: CommonMetrics,
    pub capabilities: Vec<CapabilityOutput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyResult {
    pub severity_max: String,
    pub diagnostics: Vec<Value>,
    pub capabilities: Vec<CapabilityOutput>,
}

pub trait EmulatorSession: Send {
    fn request(&self) -> &ExecutionRequest;
    fn execute(&mut self, sink: &mut dyn EmissionSink) -> Result<RunResult, String>;

    fn step_frame(&mut self) -> Result<(), String>;
    fn reset(&mut self) -> Result<(), String>;
    fn snapshot(&self) -> Result<Vec<u8>, String>;
    fn restore_snapshot(&mut self, bytes: &[u8]) -> Result<(), String>;
}

pub trait EmulatorBackend: Send + Sync {
    fn machine_id(&self) -> MachineId;
    fn display_name(&self) -> String;
    fn config_schema(&self) -> MachineConfigSchema;

    /// Declare caller-imposed bundle limits independently from machine
    /// configuration and timing parameters. The empty catalog is valid.
    fn execution_limit_catalog(&self) -> ExecutionLimitCatalog {
        ExecutionLimitCatalog::empty()
    }

    fn prepare_configuration(
        &self,
        configuration: &MachineConfiguration,
    ) -> Result<PreparedConfiguration, String> {
        configuration.prepare(&self.config_schema())
    }

    fn prepare_execution_controls(
        &self,
        controls: &ExecutionControls,
    ) -> Result<PreparedExecutionControls, String> {
        let prepared = controls.prepare(&self.execution_limit_catalog())?;
        self.validate_execution_limit_combination(&prepared.bundle_limits)?;
        Ok(prepared)
    }

    /// Validate bundle-specific combinations after common limit admission.
    /// V1 intentionally leaves the combination language bundle-owned.
    fn validate_execution_limit_combination(
        &self,
        _limits: &[BundleExecutionLimit],
    ) -> Result<(), String> {
        Ok(())
    }
    /// Construct an execution from the fully validated execution-side and
    /// observation contexts. Prepared execution is the only construction path
    /// exposed by the publication contract.
    fn create_execution_with_prepared_run(
        &self,
        artifact: &[u8],
        request: ExecutionRequest,
        prepared_run: PreparedRun,
        prepared_observation: PreparedObservation,
    ) -> Result<Box<dyn EmulatorSession>, String>;
}

pub trait VerifierBackend: Send + Sync {
    fn machine_id(&self) -> MachineId;
    fn verify_bytes(&self, rom_bytes: &[u8]) -> Result<VerifyResult, String>;
}
