use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use glassvm_normalizer_contract::{
    CapabilityDependency, CapabilityDescriptor, CapabilityId, CapabilityReceipt, CapabilityRequest,
    CapabilityStatus, FrameCaptureRequirement, InputValueSelector, NormalizedAccessRequirement,
    NormalizerCatalog, SnapshotRequirement,
};

use crate::{
    AccessDetail, ContentDigest, EventKind, EventSelection, FrameCapture, InputCatalog, InputId,
    MachineId, NativeEvent, RunId, SchemaRef, SchemaVersion, SnapshotCapture, VersionStamp,
    canonical_json_fingerprint,
};

pub const EMISSION_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(2, 1, 0);
pub const NATIVE_EVIDENCE_ENVELOPE_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEvidenceEnvelope {
    pub schema: SchemaRef,
    pub run_id: RunId,
    pub machine_id: MachineId,
    pub machine_version: VersionStamp,
    pub sequence: u64,
    pub step: u64,
    pub native_event: NativeEvent,
}

impl NativeEvidenceEnvelope {
    pub fn new(
        run_id: RunId,
        machine_id: MachineId,
        machine_version: VersionStamp,
        sequence: u64,
        step: u64,
        native_event: NativeEvent,
    ) -> Self {
        Self {
            schema: SchemaRef::new(
                "glassvm.native_evidence",
                NATIVE_EVIDENCE_ENVELOPE_SCHEMA_VERSION,
            ),
            run_id,
            machine_id,
            machine_version,
            sequence,
            step,
            native_event,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let expected = SchemaRef::new(
            "glassvm.native_evidence",
            NATIVE_EVIDENCE_ENVELOPE_SCHEMA_VERSION,
        );
        if self.schema != expected {
            return Err(format!(
                "unsupported native-evidence envelope schema {} {}",
                self.schema.id, self.schema.version
            ));
        }
        if self.native_event.kind.trim().is_empty() {
            return Err("native-evidence event kind cannot be empty".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceGuarantee {
    Summary,
    CompleteNormalized,
    OracleReady,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedEventRequest {
    pub events: EventSelection,
    pub access_detail: AccessDetail,
    pub state_diffs: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEvidenceRequest {
    pub enabled: bool,
    pub all: bool,
    pub kinds: Vec<String>,
}

impl NativeEvidenceRequest {
    pub fn includes(&self, kind: &str) -> bool {
        self.enabled && (self.all || self.kinds.iter().any(|candidate| candidate == kind))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameRequest {
    pub capture: FrameCapture,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotRequest {
    pub capture: SnapshotCapture,
}

/// Select the first-class input-value evidence channel. Which concrete input
/// values are emitted is negotiated by capability prerequisites; there is no
/// implicit all-input selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputValueEvidenceRequest {
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverflowPolicy {
    Fail,
    AllowIncomplete,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmissionBudgets {
    pub max_normalized_events: Option<u64>,
    pub max_normalized_bytes: Option<u64>,
    pub max_capability_bytes: Option<u64>,
    pub max_native_events: Option<u64>,
    pub max_native_bytes: Option<u64>,
    pub max_frame_bytes: Option<u64>,
    pub max_snapshot_bytes: Option<u64>,
    pub max_input_value_events: Option<u64>,
    pub max_input_value_bytes: Option<u64>,
    pub max_total_logical_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationRequest {
    pub schema_version: SchemaVersion,
    pub guarantee: EvidenceGuarantee,
    pub normalized_events: NormalizedEventRequest,
    pub capabilities: Vec<CapabilityRequest>,
    pub native_evidence: NativeEvidenceRequest,
    pub frames: FrameRequest,
    pub snapshots: SnapshotRequest,
    pub input_value_evidence: InputValueEvidenceRequest,
    pub budgets: EmissionBudgets,
    pub overflow: OverflowPolicy,
}

pub const PREPARED_OBSERVATION_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparedCapabilityStatus {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedCapability {
    pub id: CapabilityId,
    pub requested: bool,
    pub required: bool,
    pub status: PreparedCapabilityStatus,
    pub dependencies: Vec<CapabilityDependency>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedNormalizer {
    pub id: String,
    pub version: String,
    /// Requests passed to the per-run normalizer. Dependency capabilities are
    /// part of `PreparedObservation.capabilities` but are not emitted unless
    /// they were roots in the caller request.
    pub requests: Vec<CapabilityRequest>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedObservation {
    pub schema_version: SchemaVersion,
    pub identity: ContentDigest,
    pub request: ObservationRequest,
    /// The complete capability dependency closure, including unavailable
    /// optional nodes. This is prospective preparation state, not an output
    /// or evidence receipt.
    pub capabilities: Vec<PreparedCapability>,
    /// Native observation schemas the bundle must make available internally
    /// to satisfy the selected capability closure.
    pub native_observation_schemas: Vec<String>,
    /// Concrete input IDs selected by available capability prerequisites.
    /// Selectors are never carried beyond preparation.
    pub input_value_ids: Vec<InputId>,
    pub normalizer: PreparedNormalizer,
}

#[derive(Debug, Clone)]
struct PreparedCandidate {
    requested: bool,
    required: bool,
    descriptor: Option<CapabilityDescriptor>,
}

#[derive(Debug, Clone)]
struct DependencyAvailability {
    available: bool,
    reason: Option<String>,
}

impl PreparedObservation {
    pub fn prepare(
        catalog: &NormalizerCatalog,
        request: &ObservationRequest,
    ) -> Result<Self, Vec<String>> {
        Self::prepare_with_input_catalog(catalog, &InputCatalog::empty(), request)
    }

    pub fn prepare_with_input_catalog(
        catalog: &NormalizerCatalog,
        input_catalog: &InputCatalog,
        request: &ObservationRequest,
    ) -> Result<Self, Vec<String>> {
        let mut errors = Vec::new();
        if let Err(error) = request.validate() {
            errors.push(error);
        }
        if let Err(error) = catalog.validate() {
            errors.push(error);
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        let roots = request.capabilities.clone();
        let mut root_seen = BTreeSet::new();
        for root in &roots {
            if !root_seen.insert(root.id.clone()) {
                errors.push(format!(
                    "observation request selects capability {} more than once",
                    root.id.as_str()
                ));
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        let mut candidates = BTreeMap::new();
        let mut queue = roots
            .iter()
            .map(|root| (root.id.clone(), root.required, true))
            .collect::<Vec<_>>();
        while let Some((id, required, requested)) = queue.pop() {
            let descriptor = catalog.find(&id).cloned();
            let entry = candidates.entry(id.clone()).or_insert(PreparedCandidate {
                requested: false,
                required: false,
                descriptor: descriptor.clone(),
            });
            let first_visit = !entry.requested && !entry.required;
            let required_upgrade = required && !entry.required;
            entry.requested |= requested;
            entry.required |= required;
            if entry.descriptor.is_none() {
                entry.descriptor = descriptor.clone();
            }
            if (first_visit || required_upgrade)
                && let Some(descriptor) = descriptor
            {
                for dependency in &descriptor.dependencies {
                    if let CapabilityDependency::Capability(dependency_id) = dependency {
                        queue.push((dependency_id.clone(), required, false));
                    }
                }
            }
        }

        let mut availability = BTreeMap::new();
        for id in candidates.keys() {
            resolve_capability(
                id,
                &candidates,
                input_catalog,
                request,
                &mut availability,
                &mut BTreeSet::new(),
            );
        }

        for (id, candidate) in &candidates {
            if candidate.required
                && !availability
                    .get(id)
                    .is_some_and(|availability| availability.available)
            {
                let reason = availability
                    .get(id)
                    .and_then(|availability| availability.reason.as_deref())
                    .unwrap_or("capability dependency or emission prerequisite is unavailable");
                errors.push(format!(
                    "required capability {} is unavailable: {reason}",
                    id.as_str(),
                ));
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        let mut native_observation_schemas = BTreeSet::new();
        let mut input_value_ids = BTreeSet::new();
        let capabilities = candidates
            .iter()
            .map(|(id, candidate)| {
                let resolved = availability.get(id);
                let available = resolved.is_some_and(|availability| availability.available);
                let dependencies = candidate
                    .descriptor
                    .as_ref()
                    .map(|descriptor| descriptor.dependencies.clone())
                    .unwrap_or_default();
                if available {
                    for dependency in &dependencies {
                        if let CapabilityDependency::NativeEvidence { schema_id } = dependency {
                            native_observation_schemas.insert(schema_id.clone());
                        }
                        if let CapabilityDependency::InputValueEvidence { selectors } = dependency {
                            for input_id in resolve_input_value_selectors(input_catalog, selectors)
                            {
                                input_value_ids.insert(input_id);
                            }
                        }
                    }
                }
                PreparedCapability {
                    id: id.clone(),
                    requested: candidate.requested,
                    required: candidate.required,
                    status: if available {
                        PreparedCapabilityStatus::Available
                    } else {
                        PreparedCapabilityStatus::Unavailable
                    },
                    dependencies,
                    message: (!available).then(|| {
                        if candidate.descriptor.is_none() {
                            "capability is not declared by the normalizer catalog".into()
                        } else {
                            resolved
                                .and_then(|availability| availability.reason.clone())
                                .unwrap_or_else(|| {
                                    "capability dependency or emission prerequisite is unavailable"
                                        .into()
                                })
                        }
                    }),
                }
            })
            .collect::<Vec<_>>();

        let normalizer_requests = roots
            .iter()
            .filter(|root| {
                availability
                    .get(&root.id)
                    .is_some_and(|availability| availability.available)
            })
            .cloned()
            .collect::<Vec<_>>();
        let prepared = Self {
            schema_version: PREPARED_OBSERVATION_SCHEMA_VERSION,
            identity: ContentDigest::sha256(&[]),
            request: request.clone(),
            capabilities,
            native_observation_schemas: native_observation_schemas.into_iter().collect(),
            input_value_ids: input_value_ids.into_iter().collect(),
            normalizer: PreparedNormalizer {
                id: catalog.normalizer_id.clone(),
                version: catalog.normalizer_version.clone(),
                requests: normalizer_requests,
            },
        };
        let identity = prepared.compute_identity().map_err(|error| vec![error])?;
        Ok(Self {
            identity,
            ..prepared
        })
    }

    pub fn normalizer_requests(&self) -> Vec<CapabilityRequest> {
        self.normalizer.requests.clone()
    }

    /// Return retrospective unavailable receipts for optional capability roots
    /// that preparation explicitly downgraded. Dependency-only nodes are not
    /// included because callers requested only the roots.
    pub fn unavailable_capability_receipts(&self) -> Vec<CapabilityReceipt> {
        self.capabilities
            .iter()
            .filter(|capability| {
                capability.requested && capability.status == PreparedCapabilityStatus::Unavailable
            })
            .map(|capability| CapabilityReceipt {
                id: capability.id.clone(),
                status: CapabilityStatus::Unavailable,
                output_schema: None,
                logical_bytes: 0,
                message: capability.message.clone(),
            })
            .collect()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != PREPARED_OBSERVATION_SCHEMA_VERSION {
            return Err(format!(
                "unsupported prepared-observation schema {}; expected {}",
                self.schema_version, PREPARED_OBSERVATION_SCHEMA_VERSION
            ));
        }
        self.request.validate()?;
        if self
            .input_value_ids
            .windows(2)
            .any(|window| window[0] >= window[1])
        {
            return Err("prepared input-value IDs must be sorted and unique".into());
        }
        if self.identity != self.compute_identity()? {
            return Err("prepared observation identity does not match its contents".into());
        }
        Ok(())
    }

    fn compute_identity(&self) -> Result<ContentDigest, String> {
        #[derive(Serialize)]
        struct IdentityInput<'a> {
            schema_version: SchemaVersion,
            request: &'a ObservationRequest,
            capabilities: &'a [PreparedCapability],
            native_observation_schemas: &'a [String],
            input_value_ids: &'a [InputId],
            normalizer: &'a PreparedNormalizer,
        }
        canonical_json_fingerprint(
            "glassvm.prepared_observation.v1",
            &IdentityInput {
                schema_version: self.schema_version,
                request: &self.request,
                capabilities: &self.capabilities,
                native_observation_schemas: &self.native_observation_schemas,
                input_value_ids: &self.input_value_ids,
                normalizer: &self.normalizer,
            },
        )
    }
}

fn resolve_capability(
    id: &CapabilityId,
    candidates: &BTreeMap<CapabilityId, PreparedCandidate>,
    input_catalog: &InputCatalog,
    request: &ObservationRequest,
    availability: &mut BTreeMap<CapabilityId, DependencyAvailability>,
    visiting: &mut BTreeSet<CapabilityId>,
) -> DependencyAvailability {
    if let Some(resolved) = availability.get(id) {
        return resolved.clone();
    }
    if !visiting.insert(id.clone()) {
        return DependencyAvailability {
            available: false,
            reason: Some(format!(
                "capability dependency cycle reaches {}",
                id.as_str()
            )),
        };
    }
    let resolved = match candidates
        .get(id)
        .and_then(|candidate| candidate.descriptor.as_ref())
    {
        None => DependencyAvailability {
            available: false,
            reason: Some("capability is not declared by the normalizer catalog".into()),
        },
        Some(descriptor) => descriptor
            .dependencies
            .iter()
            .find_map(|dependency| {
                let result = resolve_dependency(
                    dependency,
                    candidates,
                    input_catalog,
                    request,
                    availability,
                    visiting,
                );
                (!result.available).then(|| DependencyAvailability {
                    available: false,
                    reason: Some(format_dependency_failure(dependency, &result)),
                })
            })
            .unwrap_or(DependencyAvailability {
                available: true,
                reason: None,
            }),
    };
    visiting.remove(id);
    availability.insert(id.clone(), resolved.clone());
    resolved
}

fn resolve_dependency(
    dependency: &CapabilityDependency,
    candidates: &BTreeMap<CapabilityId, PreparedCandidate>,
    input_catalog: &InputCatalog,
    request: &ObservationRequest,
    availability: &mut BTreeMap<CapabilityId, DependencyAvailability>,
    visiting: &mut BTreeSet<CapabilityId>,
) -> DependencyAvailability {
    match dependency {
        CapabilityDependency::NormalizedEvents => {
            if request.normalized_events.events == EventSelection::None {
                unavailable("normalized event channel is not selected")
            } else {
                available()
            }
        }
        CapabilityDependency::NormalizedEventKinds { kinds } => {
            for required in kinds {
                let Some(kind) = parse_event_kind_key(required) else {
                    return unavailable(format!("unknown normalized event kind {required:?}"));
                };
                if !request.normalized_events.events.includes(&kind) {
                    return unavailable(format!(
                        "normalized event kind {required:?} is not selected"
                    ));
                }
            }
            available()
        }
        CapabilityDependency::NormalizedState {
            reads,
            writes,
            state_diffs,
            access,
        } => {
            if request.normalized_events.events == EventSelection::None {
                return unavailable("normalized event channel is not selected");
            }
            if (*reads || *writes) && request.normalized_events.access_detail == AccessDetail::None
            {
                return unavailable("normalized state reads/writes require access detail");
            }
            if *state_diffs && !request.normalized_events.state_diffs {
                return unavailable("normalized state diffs are not selected");
            }
            if !access_satisfies(request.normalized_events.access_detail, *access) {
                return unavailable(format!(
                    "normalized access detail {:?} does not satisfy {:?}",
                    request.normalized_events.access_detail, access
                ));
            }
            available()
        }
        CapabilityDependency::NativeEvidence { .. } => available(),
        CapabilityDependency::NativeEventKinds { kinds } => {
            if !request.native_evidence.enabled {
                return unavailable("native evidence channel is not selected");
            }
            if !request.native_evidence.all {
                for required in kinds {
                    if !request
                        .native_evidence
                        .kinds
                        .iter()
                        .any(|kind| kind == required)
                    {
                        return unavailable(format!(
                            "native event kind {required:?} is not selected"
                        ));
                    }
                }
            }
            available()
        }
        CapabilityDependency::Frames { capture } => {
            if frame_capture_satisfies(request.frames.capture, *capture) {
                available()
            } else {
                unavailable(format!(
                    "frame capture {:?} does not satisfy {:?}",
                    request.frames.capture, capture
                ))
            }
        }
        CapabilityDependency::Snapshots { capture } => {
            if snapshot_capture_satisfies(request.snapshots.capture, *capture) {
                available()
            } else {
                unavailable(format!(
                    "snapshot capture {:?} does not satisfy {:?}",
                    request.snapshots.capture, capture
                ))
            }
        }
        CapabilityDependency::FinalState => {
            if request.snapshots.capture == SnapshotCapture::Final {
                available()
            } else {
                unavailable("final-state access requires an explicit final snapshot")
            }
        }
        CapabilityDependency::InputValueEvidence { selectors } => {
            if !request.input_value_evidence.enabled {
                unavailable("input-value evidence channel is not selected")
            } else if let Some(reason) = input_value_selector_failure(input_catalog, selectors) {
                unavailable(reason)
            } else {
                available()
            }
        }
        CapabilityDependency::Capability(dependency_id) => resolve_capability(
            dependency_id,
            candidates,
            input_catalog,
            request,
            availability,
            visiting,
        ),
    }
}

fn resolve_input_value_selectors(
    input_catalog: &InputCatalog,
    selectors: &[InputValueSelector],
) -> Vec<InputId> {
    let mut ids = BTreeSet::new();
    for selector in selectors {
        match selector {
            InputValueSelector::InputId(id) => {
                if input_catalog.find(id).is_some() {
                    ids.insert(id.clone());
                }
            }
            InputValueSelector::SchemaFamily(family) => {
                ids.extend(input_catalog.family_ids(family));
            }
        }
    }
    ids.into_iter().collect()
}

fn input_value_selector_failure(
    input_catalog: &InputCatalog,
    selectors: &[InputValueSelector],
) -> Option<String> {
    for selector in selectors {
        let matches = match selector {
            InputValueSelector::InputId(id) => input_catalog
                .find(id)
                .map(|_| vec![id.clone()])
                .unwrap_or_default(),
            InputValueSelector::SchemaFamily(family) => input_catalog.family_ids(family),
        };
        if matches.is_empty() {
            return Some(format!(
                "input-value selector {:?} matches no declared input",
                selector
            ));
        }
    }
    None
}

fn available() -> DependencyAvailability {
    DependencyAvailability {
        available: true,
        reason: None,
    }
}

fn unavailable(reason: impl Into<String>) -> DependencyAvailability {
    DependencyAvailability {
        available: false,
        reason: Some(reason.into()),
    }
}

fn format_dependency_failure(
    dependency: &CapabilityDependency,
    result: &DependencyAvailability,
) -> String {
    let reason = result
        .reason
        .as_deref()
        .unwrap_or("dependency is unavailable");
    match dependency {
        CapabilityDependency::Capability(id) => {
            format!(
                "dependent capability {} is unavailable: {reason}",
                id.as_str()
            )
        }
        _ => reason.into(),
    }
}

fn access_satisfies(actual: AccessDetail, required: NormalizedAccessRequirement) -> bool {
    match required {
        NormalizedAccessRequirement::AfterOnly => {
            matches!(
                actual,
                AccessDetail::AfterOnly | AccessDetail::BeforeAndAfter
            )
        }
        NormalizedAccessRequirement::BeforeAndAfter => actual == AccessDetail::BeforeAndAfter,
    }
}

fn frame_capture_satisfies(actual: FrameCapture, required: FrameCaptureRequirement) -> bool {
    match required {
        FrameCaptureRequirement::Hashes => {
            matches!(actual, FrameCapture::Hashes | FrameCapture::Full)
        }
        FrameCaptureRequirement::Full => actual == FrameCapture::Full,
    }
}

fn snapshot_capture_satisfies(actual: SnapshotCapture, required: SnapshotRequirement) -> bool {
    match required {
        SnapshotRequirement::Final => actual == SnapshotCapture::Final,
        SnapshotRequirement::EverySteps => matches!(actual, SnapshotCapture::EverySteps(_)),
    }
}

fn parse_event_kind_key(key: &str) -> Option<EventKind> {
    Some(match key {
        "run_started" => EventKind::RunStarted,
        "step_started" => EventKind::StepStarted,
        "instruction_decoded" => EventKind::InstructionDecoded,
        "register_read" => EventKind::RegisterRead,
        "register_write" => EventKind::RegisterWrite,
        "memory_read" => EventKind::MemoryRead,
        "memory_write" => EventKind::MemoryWrite,
        "stack_changed" => EventKind::StackChanged,
        "branch_taken" => EventKind::BranchTaken,
        "call" => EventKind::Call,
        "return" => EventKind::Return,
        "interrupt" => EventKind::Interrupt,
        "trap" => EventKind::Trap,
        "display_write" => EventKind::DisplayWrite,
        "input_applied" => EventKind::InputApplied,
        "input_sampled" => EventKind::InputSampled,
        "sound_emitted" => EventKind::SoundEmitted,
        "timer_changed" => EventKind::TimerChanged,
        "frame_completed" => EventKind::FrameCompleted,
        "snapshot_captured" => EventKind::SnapshotCaptured,
        "run_halted" => EventKind::RunHalted,
        "run_crashed" => EventKind::RunCrashed,
        extension if extension.strip_prefix("extension:").is_some() => {
            EventKind::Extension(extension.strip_prefix("extension:").unwrap().into())
        }
        _ => return None,
    })
}

impl ObservationRequest {
    pub fn summary() -> Self {
        Self {
            schema_version: EMISSION_SCHEMA_VERSION,
            guarantee: EvidenceGuarantee::Summary,
            normalized_events: NormalizedEventRequest {
                events: EventSelection::None,
                access_detail: AccessDetail::None,
                state_diffs: false,
            },
            capabilities: Vec::new(),
            native_evidence: NativeEvidenceRequest {
                enabled: false,
                all: false,
                kinds: Vec::new(),
            },
            frames: FrameRequest {
                capture: FrameCapture::None,
            },
            snapshots: SnapshotRequest {
                capture: SnapshotCapture::None,
            },
            input_value_evidence: InputValueEvidenceRequest { enabled: false },
            budgets: EmissionBudgets::default(),
            overflow: OverflowPolicy::Fail,
        }
    }

    pub fn fitness() -> Self {
        Self {
            normalized_events: NormalizedEventRequest {
                events: EventSelection::Lifecycle,
                access_detail: AccessDetail::None,
                state_diffs: false,
            },
            ..Self::summary()
        }
    }

    pub fn debug() -> Self {
        Self {
            normalized_events: NormalizedEventRequest {
                events: EventSelection::Instructions,
                access_detail: AccessDetail::AfterOnly,
                state_diffs: true,
            },
            native_evidence: NativeEvidenceRequest {
                enabled: true,
                all: true,
                kinds: Vec::new(),
            },
            frames: FrameRequest {
                capture: FrameCapture::Hashes,
            },
            snapshots: SnapshotRequest {
                capture: SnapshotCapture::Final,
            },
            ..Self::summary()
        }
    }

    pub fn forensics() -> Self {
        Self {
            normalized_events: NormalizedEventRequest {
                events: EventSelection::All,
                access_detail: AccessDetail::BeforeAndAfter,
                state_diffs: true,
            },
            native_evidence: NativeEvidenceRequest {
                enabled: true,
                all: true,
                kinds: Vec::new(),
            },
            frames: FrameRequest {
                capture: FrameCapture::Hashes,
            },
            snapshots: SnapshotRequest {
                capture: SnapshotCapture::Final,
            },
            ..Self::summary()
        }
    }

    pub fn ui() -> Self {
        Self {
            normalized_events: NormalizedEventRequest {
                events: EventSelection::Instructions,
                access_detail: AccessDetail::AfterOnly,
                state_diffs: true,
            },
            native_evidence: NativeEvidenceRequest {
                enabled: true,
                all: true,
                kinds: Vec::new(),
            },
            frames: FrameRequest {
                capture: FrameCapture::Full,
            },
            snapshots: SnapshotRequest {
                capture: SnapshotCapture::None,
            },
            ..Self::summary()
        }
    }

    pub fn research() -> Self {
        Self::forensics()
    }

    pub fn full_archaeology() -> Self {
        Self {
            frames: FrameRequest {
                capture: FrameCapture::Full,
            },
            snapshots: SnapshotRequest {
                capture: SnapshotCapture::EverySteps(4096),
            },
            ..Self::research()
        }
    }

    pub fn embedding() -> Self {
        Self {
            normalized_events: NormalizedEventRequest {
                events: EventSelection::Effects,
                access_detail: AccessDetail::AfterOnly,
                state_diffs: true,
            },
            native_evidence: NativeEvidenceRequest {
                enabled: false,
                all: false,
                kinds: Vec::new(),
            },
            frames: FrameRequest {
                capture: FrameCapture::Hashes,
            },
            snapshots: SnapshotRequest {
                capture: SnapshotCapture::None,
            },
            ..Self::summary()
        }
    }

    /// Construct the explicit complete-normalized guarantee profile.
    pub fn complete_normalized() -> Self {
        let mut request = Self::forensics();
        request.guarantee = EvidenceGuarantee::CompleteNormalized;
        request
            .validate()
            .expect("built-in complete-normalized profile is valid");
        request
    }

    /// Construct the explicit oracle-ready evidence profile.
    ///
    /// This requests the complete normalized stream, all bundle-native
    /// observations, frame hashes, and a final snapshot. It does not invent a
    /// bundle-independent oracle capability; callers still select the
    /// executable capability IDs declared by the target bundle.
    pub fn oracle_ready() -> Self {
        let mut request = Self::forensics();
        request.guarantee = EvidenceGuarantee::OracleReady;
        request
            .validate()
            .expect("built-in oracle-ready profile is valid");
        request
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != EMISSION_SCHEMA_VERSION {
            return Err(format!(
                "unsupported emission request schema {}; expected {}",
                self.schema_version, EMISSION_SCHEMA_VERSION
            ));
        }
        if self.normalized_events.state_diffs
            && self.normalized_events.access_detail == AccessDetail::None
        {
            return Err("state diffs require normalized access detail".into());
        }
        if !self.native_evidence.enabled
            && (self.native_evidence.all || !self.native_evidence.kinds.is_empty())
        {
            return Err("native evidence selection requires the native channel".into());
        }
        if self.native_evidence.all && !self.native_evidence.kinds.is_empty() {
            return Err("native evidence cannot select all kinds and named kinds".into());
        }
        if self.guarantee == EvidenceGuarantee::CompleteNormalized {
            if self.normalized_events.events != EventSelection::All {
                return Err("complete_normalized requires all normalized event kinds".into());
            }
            if self.normalized_events.access_detail == AccessDetail::None
                || !self.normalized_events.state_diffs
            {
                return Err("complete_normalized requires access detail and state diffs".into());
            }
        }
        if self.guarantee == EvidenceGuarantee::OracleReady {
            if !self.native_evidence.enabled {
                return Err("oracle_ready requires native evidence selection".into());
            }
            if self.snapshots.capture == SnapshotCapture::None {
                return Err("oracle_ready requires snapshot capture".into());
            }
        }
        Ok(())
    }

    /// Whether the request contains the complete normalized ontology needed
    /// by consumers that reason over unfiltered instruction/read/write order.
    pub fn produces_complete_trace(&self) -> bool {
        self.normalized_events.events == EventSelection::All
            && self.normalized_events.state_diffs
            && self.normalized_events.access_detail != AccessDetail::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Complete,
    Incomplete,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmissionChannel {
    NormalizedEvents,
    Capabilities,
    NativeEvidence,
    Frames,
    Snapshots,
    InputValueEvidence,
    Envelope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelStatus {
    Complete,
    Omitted,
    Truncated,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelReceipt {
    pub channel: EmissionChannel,
    pub requested: bool,
    pub status: ChannelStatus,
    pub item_count: u64,
    pub logical_bytes: u64,
    pub encoded_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetKind {
    NormalizedEvents,
    NormalizedBytes,
    CapabilityBytes,
    NativeEvents,
    NativeBytes,
    FrameBytes,
    SnapshotBytes,
    InputValueEvents,
    InputValueBytes,
    TotalLogicalBytes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BudgetReceipt {
    pub configured: EmissionBudgets,
    pub exceeded: Option<BudgetKind>,
    pub observed_logical_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmissionFailure {
    pub channel: Option<EmissionChannel>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceReceipt {
    pub schema_version: SchemaVersion,
    pub run_id: RunId,
    pub machine_id: MachineId,
    pub machine_version: VersionStamp,
    pub requested_guarantee: EvidenceGuarantee,
    pub fulfilled_guarantee: Option<EvidenceGuarantee>,
    pub status: EvidenceStatus,
    pub channels: Vec<ChannelReceipt>,
    pub capabilities: Vec<CapabilityReceipt>,
    pub budgets: BudgetReceipt,
    pub failure: Option<EmissionFailure>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_capability(id: &str, dependencies: Vec<CapabilityDependency>) -> CapabilityDescriptor {
        CapabilityDescriptor {
            schema: CapabilityId::new(id).unwrap().schema(),
            output_type: "json.object".into(),
            dependencies,
            cost_class: glassvm_normalizer_contract::CostClass::Bounded,
        }
    }

    fn test_catalog(capabilities: Vec<CapabilityDescriptor>) -> NormalizerCatalog {
        NormalizerCatalog {
            schema_version: glassvm_normalizer_contract::NORMALIZER_CONTRACT_SCHEMA_VERSION,
            normalizer_id: "test.prepared.normalizer".into(),
            normalizer_version: "test.v1".into(),
            capabilities,
        }
    }

    #[test]
    fn summary_request_is_minimal_and_round_trips() {
        let request = ObservationRequest::summary();
        request.validate().unwrap();
        let encoded = serde_json::to_vec(&request).unwrap();
        let decoded: ObservationRequest = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, request);
    }

    #[test]
    fn named_profiles_expand_to_valid_requests() {
        for request in [
            ObservationRequest::fitness(),
            ObservationRequest::debug(),
            ObservationRequest::forensics(),
            ObservationRequest::ui(),
            ObservationRequest::research(),
            ObservationRequest::full_archaeology(),
            ObservationRequest::embedding(),
            ObservationRequest::complete_normalized(),
            ObservationRequest::oracle_ready(),
        ] {
            request.validate().unwrap();
        }
        assert_eq!(
            ObservationRequest::complete_normalized().guarantee,
            EvidenceGuarantee::CompleteNormalized
        );
    }

    #[test]
    fn oracle_ready_profile_requests_explicit_validation_inputs() {
        let request = ObservationRequest::oracle_ready();
        request.validate().unwrap();
        assert_eq!(request.guarantee, EvidenceGuarantee::OracleReady);
        assert_eq!(request.normalized_events.events, EventSelection::All);
        assert!(request.native_evidence.enabled);
        assert!(request.native_evidence.all);
        assert_eq!(request.frames.capture, FrameCapture::Hashes);
        assert_eq!(request.snapshots.capture, SnapshotCapture::Final);

        let mut missing_native = request.clone();
        missing_native.native_evidence.enabled = false;
        assert!(missing_native.validate().is_err());

        let mut missing_snapshot = request;
        missing_snapshot.snapshots.capture = SnapshotCapture::None;
        assert!(missing_snapshot.validate().is_err());
    }

    #[test]
    fn complete_normalized_requires_all_events() {
        let mut request = ObservationRequest::summary();
        request.guarantee = EvidenceGuarantee::CompleteNormalized;
        assert!(request.validate().is_err());
        request.normalized_events.events = EventSelection::All;
        request.normalized_events.access_detail = AccessDetail::BeforeAndAfter;
        request.normalized_events.state_diffs = true;
        request.validate().unwrap();
    }

    #[test]
    fn native_kinds_cannot_be_requested_when_native_channel_is_disabled() {
        let mut request = ObservationRequest::summary();
        request.native_evidence.kinds.push("registers".into());
        assert!(request.validate().is_err());
    }

    #[test]
    fn prepared_observation_rejects_missing_required_capability_before_execution() {
        let request = ObservationRequest {
            capabilities: vec![CapabilityRequest::required(
                CapabilityId::new("test.required.v1").unwrap(),
            )],
            ..ObservationRequest::summary()
        };
        let errors = PreparedObservation::prepare(&test_catalog(Vec::new()), &request)
            .expect_err("missing required capability must fail preparation");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("test.required.v1"))
        );
    }

    #[test]
    fn prepared_observation_keeps_missing_optional_capability_explicitly_unavailable() {
        let id = CapabilityId::new("test.optional.v1").unwrap();
        let request = ObservationRequest {
            capabilities: vec![CapabilityRequest::optional(id.clone())],
            ..ObservationRequest::summary()
        };
        let prepared = PreparedObservation::prepare(&test_catalog(Vec::new()), &request)
            .expect("missing optional capability must downgrade");
        let capability = prepared
            .capabilities
            .iter()
            .find(|capability| capability.id == id)
            .unwrap();
        assert_eq!(capability.status, PreparedCapabilityStatus::Unavailable);
        assert!(prepared.normalizer.requests.is_empty());
    }

    #[test]
    fn prepared_observation_resolves_capability_and_native_dependency_closure() {
        let leaf = CapabilityId::new("test.leaf.v1").unwrap();
        let root = CapabilityId::new("test.root.v1").unwrap();
        let request = ObservationRequest {
            capabilities: vec![CapabilityRequest::required(root.clone())],
            ..ObservationRequest::summary()
        };
        let prepared = PreparedObservation::prepare(
            &test_catalog(vec![
                test_capability(
                    root.as_str(),
                    vec![
                        CapabilityDependency::Capability(leaf.clone()),
                        CapabilityDependency::NativeEvidence {
                            schema_id: "test.native.v1".into(),
                        },
                    ],
                ),
                test_capability(leaf.as_str(), Vec::new()),
            ]),
            &request,
        )
        .expect("dependency closure must prepare");
        assert_eq!(prepared.normalizer.requests, request.capabilities);
        assert!(
            prepared
                .native_observation_schemas
                .contains(&"test.native.v1".to_string())
        );
        assert!(
            prepared
                .capabilities
                .iter()
                .any(|capability| capability.id == leaf && !capability.requested)
        );
        prepared.validate().unwrap();
    }

    #[test]
    fn prepared_observation_merges_shared_dependency_requirement_strength() {
        let required_root = CapabilityId::new("test.required_root.v1").unwrap();
        let optional_root = CapabilityId::new("test.optional_root.v1").unwrap();
        let shared = CapabilityId::new("test.shared.v1").unwrap();
        let request = ObservationRequest {
            capabilities: vec![
                CapabilityRequest::required(required_root.clone()),
                CapabilityRequest::optional(optional_root.clone()),
            ],
            ..ObservationRequest::summary()
        };
        let prepared = PreparedObservation::prepare(
            &test_catalog(vec![
                test_capability(
                    required_root.as_str(),
                    vec![CapabilityDependency::Capability(shared.clone())],
                ),
                test_capability(
                    optional_root.as_str(),
                    vec![CapabilityDependency::Capability(shared.clone())],
                ),
                test_capability(shared.as_str(), Vec::new()),
            ]),
            &request,
        )
        .expect("shared dependency must prepare");
        let shared_prepared = prepared
            .capabilities
            .iter()
            .find(|capability| capability.id == shared)
            .unwrap();
        assert!(shared_prepared.required);
        assert!(!shared_prepared.requested);
    }

    #[test]
    fn prepared_observation_downgrades_optional_capability_with_unsatisfied_frame_dependency() {
        let id = CapabilityId::new("test.frame_dependent.v1").unwrap();
        let request = ObservationRequest {
            capabilities: vec![CapabilityRequest::optional(id.clone())],
            ..ObservationRequest::summary()
        };
        let prepared = PreparedObservation::prepare(
            &test_catalog(vec![test_capability(
                id.as_str(),
                vec![CapabilityDependency::Frames {
                    capture: FrameCaptureRequirement::Hashes,
                }],
            )]),
            &request,
        )
        .expect("optional frame dependency must downgrade");
        assert_eq!(
            prepared
                .capabilities
                .iter()
                .find(|capability| capability.id == id)
                .unwrap()
                .status,
            PreparedCapabilityStatus::Unavailable
        );
        assert!(prepared.normalizer.requests.is_empty());
    }

    #[test]
    fn prepared_observation_rejects_missing_required_normalized_event_kind() {
        let id = CapabilityId::new("test.event_kind.v1").unwrap();
        let mut request = ObservationRequest::summary();
        request.normalized_events.events =
            EventSelection::Kinds(BTreeSet::from([EventKind::InstructionDecoded]));
        request.capabilities = vec![CapabilityRequest::required(id.clone())];
        let errors = PreparedObservation::prepare(
            &test_catalog(vec![test_capability(
                id.as_str(),
                vec![CapabilityDependency::NormalizedEventKinds {
                    kinds: vec!["frame_completed".into()],
                }],
            )]),
            &request,
        )
        .expect_err("missing required normalized event kind must fail preparation");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("frame_completed") && error.contains("not selected"))
        );
    }

    #[test]
    fn prepared_observation_marks_optional_missing_event_kind_with_reason() {
        let id = CapabilityId::new("test.optional_event_kind.v1").unwrap();
        let mut request = ObservationRequest::summary();
        request.normalized_events.events =
            EventSelection::Kinds(BTreeSet::from([EventKind::InstructionDecoded]));
        request.capabilities = vec![CapabilityRequest::optional(id.clone())];
        let prepared = PreparedObservation::prepare(
            &test_catalog(vec![test_capability(
                id.as_str(),
                vec![CapabilityDependency::NormalizedEventKinds {
                    kinds: vec!["frame_completed".into()],
                }],
            )]),
            &request,
        )
        .expect("optional missing event kind must downgrade");
        let capability = prepared
            .capabilities
            .iter()
            .find(|capability| capability.id == id)
            .unwrap();
        assert_eq!(capability.status, PreparedCapabilityStatus::Unavailable);
        assert!(
            capability
                .message
                .as_deref()
                .is_some_and(|message| message.contains("frame_completed"))
        );
        assert!(prepared.normalizer.requests.is_empty());
    }

    #[test]
    fn prepared_observation_resolves_exact_native_frame_snapshot_and_final_state_prerequisites() {
        let id = CapabilityId::new("test.complete_prerequisites.v1").unwrap();
        let mut request = ObservationRequest::summary();
        request.normalized_events.events =
            EventSelection::Kinds(BTreeSet::from([EventKind::FrameCompleted]));
        request.native_evidence.enabled = true;
        request.native_evidence.kinds = vec!["sweep".into()];
        request.frames.capture = FrameCapture::Full;
        request.snapshots.capture = SnapshotCapture::Final;
        request.capabilities = vec![CapabilityRequest::required(id.clone())];
        let prepared = PreparedObservation::prepare(
            &test_catalog(vec![test_capability(
                id.as_str(),
                vec![
                    CapabilityDependency::NormalizedEventKinds {
                        kinds: vec!["frame_completed".into()],
                    },
                    CapabilityDependency::NativeEventKinds {
                        kinds: vec!["sweep".into()],
                    },
                    CapabilityDependency::Frames {
                        capture: FrameCaptureRequirement::Full,
                    },
                    CapabilityDependency::Snapshots {
                        capture: SnapshotRequirement::Final,
                    },
                    CapabilityDependency::FinalState,
                ],
            )]),
            &request,
        )
        .expect("complete prerequisite set must prepare");
        let capability = prepared
            .capabilities
            .iter()
            .find(|capability| capability.id == id)
            .unwrap();
        assert_eq!(capability.status, PreparedCapabilityStatus::Available);
        assert_eq!(prepared.normalizer.requests, request.capabilities);
    }

    #[test]
    fn prepared_observation_rejects_missing_required_native_event_kind() {
        let id = CapabilityId::new("test.native_kind.v1").unwrap();
        let mut request = ObservationRequest::summary();
        request.native_evidence.enabled = true;
        request.native_evidence.kinds = vec!["other".into()];
        request.capabilities = vec![CapabilityRequest::required(id.clone())];
        let errors = PreparedObservation::prepare(
            &test_catalog(vec![test_capability(
                id.as_str(),
                vec![CapabilityDependency::NativeEventKinds {
                    kinds: vec!["sweep".into()],
                }],
            )]),
            &request,
        )
        .expect_err("missing required native event kind must fail preparation");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("sweep") && error.contains("not selected"))
        );
    }

    #[test]
    fn prepared_observation_rejects_missing_required_snapshot_and_final_state_access() {
        let id = CapabilityId::new("test.snapshot_state.v1").unwrap();
        let mut request = ObservationRequest::summary();
        request.capabilities = vec![CapabilityRequest::required(id.clone())];
        let errors = PreparedObservation::prepare(
            &test_catalog(vec![test_capability(
                id.as_str(),
                vec![
                    CapabilityDependency::Snapshots {
                        capture: SnapshotRequirement::Final,
                    },
                    CapabilityDependency::FinalState,
                ],
            )]),
            &request,
        )
        .expect_err("missing snapshot/final-state prerequisites must fail preparation");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("snapshot capture") || error.contains("final-state"))
        );
    }

    #[test]
    fn prepared_observation_does_not_treat_periodic_snapshots_as_final_state_access() {
        let id = CapabilityId::new("test.periodic_state.v1").unwrap();
        let mut request = ObservationRequest::summary();
        request.snapshots.capture = SnapshotCapture::EverySteps(4);
        request.capabilities = vec![CapabilityRequest::required(id.clone())];
        let errors = PreparedObservation::prepare(
            &test_catalog(vec![test_capability(
                id.as_str(),
                vec![CapabilityDependency::FinalState],
            )]),
            &request,
        )
        .expect_err("periodic snapshots must not promise final-state access");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("explicit final snapshot"))
        );
    }

    #[test]
    fn prepared_observation_propagates_optional_transitive_dependency_failure() {
        let root = CapabilityId::new("test.optional_root.v1").unwrap();
        let middle = CapabilityId::new("test.optional_middle.v1").unwrap();
        let leaf = CapabilityId::new("test.optional_leaf.v1").unwrap();
        let mut request = ObservationRequest::summary();
        request.capabilities = vec![CapabilityRequest::optional(root.clone())];
        let prepared = PreparedObservation::prepare(
            &test_catalog(vec![
                test_capability(
                    root.as_str(),
                    vec![CapabilityDependency::Capability(middle.clone())],
                ),
                test_capability(
                    middle.as_str(),
                    vec![CapabilityDependency::Capability(leaf.clone())],
                ),
                test_capability(
                    leaf.as_str(),
                    vec![CapabilityDependency::NativeEventKinds {
                        kinds: vec!["sweep".into()],
                    }],
                ),
            ]),
            &request,
        )
        .expect("optional transitive failure must not fail preparation");
        for id in [root, middle, leaf] {
            let capability = prepared
                .capabilities
                .iter()
                .find(|capability| capability.id == id)
                .unwrap();
            assert_eq!(capability.status, PreparedCapabilityStatus::Unavailable);
            assert!(capability.message.is_some());
        }
        assert!(prepared.normalizer.requests.is_empty());
    }

    #[test]
    fn prepared_observation_merges_shared_prerequisite_strength() {
        let required_root = CapabilityId::new("test.required_exact_root.v1").unwrap();
        let optional_root = CapabilityId::new("test.optional_exact_root.v1").unwrap();
        let shared = CapabilityId::new("test.shared_exact.v1").unwrap();
        let mut request = ObservationRequest::summary();
        request.normalized_events.events =
            EventSelection::Kinds(BTreeSet::from([EventKind::FrameCompleted]));
        request.capabilities = vec![
            CapabilityRequest::required(required_root.clone()),
            CapabilityRequest::optional(optional_root.clone()),
        ];
        let prepared = PreparedObservation::prepare(
            &test_catalog(vec![
                test_capability(
                    required_root.as_str(),
                    vec![CapabilityDependency::Capability(shared.clone())],
                ),
                test_capability(
                    optional_root.as_str(),
                    vec![CapabilityDependency::Capability(shared.clone())],
                ),
                test_capability(
                    shared.as_str(),
                    vec![CapabilityDependency::NormalizedEventKinds {
                        kinds: vec!["frame_completed".into()],
                    }],
                ),
            ]),
            &request,
        )
        .expect("shared exact prerequisite must prepare");
        let shared_prepared = prepared
            .capabilities
            .iter()
            .find(|capability| capability.id == shared)
            .unwrap();
        assert!(shared_prepared.required);
        assert!(!shared_prepared.requested);
        assert_eq!(shared_prepared.status, PreparedCapabilityStatus::Available);
    }

    fn input_catalog_for_tests() -> InputCatalog {
        let family = crate::SchemaFamilyId::new("glassvm.input.digital-key").unwrap();
        let payload_schema = SchemaRef::new("test.input.payload", SchemaVersion::V1);
        let application = crate::InputApplicationSemantics::new(SchemaRef::new(
            "test.input.application",
            SchemaVersion::V1,
        ))
        .unwrap();
        let input = |id: &str| crate::InputSpec {
            id: InputId::new(id).unwrap(),
            payload_schema: payload_schema.clone(),
            schema_family: Some(family.clone()),
            coordinate_policy: crate::CoordinatePolicy::frame_start(),
            delivery_modes: BTreeSet::from([crate::InputDeliveryMode::Scheduled]),
            application: application.clone(),
        };
        InputCatalog {
            schema: crate::input_catalog_schema(),
            inputs: vec![input("chip8.key.1"), input("chip8.key.0")],
        }
    }

    #[test]
    fn prepared_observation_rejects_required_input_value_channel_without_selection() {
        let id = CapabilityId::new("test.input_required.v1").unwrap();
        let request = ObservationRequest {
            capabilities: vec![CapabilityRequest::required(id.clone())],
            ..ObservationRequest::summary()
        };
        let family = crate::SchemaFamilyId::new("glassvm.input.digital-key").unwrap();
        let errors = PreparedObservation::prepare_with_input_catalog(
            &test_catalog(vec![test_capability(
                id.as_str(),
                vec![CapabilityDependency::InputValueEvidence {
                    selectors: vec![InputValueSelector::SchemaFamily(family)],
                }],
            )]),
            &input_catalog_for_tests(),
            &request,
        )
        .expect_err("required input-value evidence must fail when not selected");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("input-value evidence"))
        );
    }

    #[test]
    fn prepared_observation_marks_optional_missing_input_selector_unavailable() {
        let id = CapabilityId::new("test.input_optional.v1").unwrap();
        let mut request = ObservationRequest::summary();
        request.input_value_evidence.enabled = true;
        request.capabilities = vec![CapabilityRequest::optional(id.clone())];
        let missing = InputId::new("chip8.key.missing").unwrap();
        let prepared = PreparedObservation::prepare_with_input_catalog(
            &test_catalog(vec![test_capability(
                id.as_str(),
                vec![CapabilityDependency::InputValueEvidence {
                    selectors: vec![InputValueSelector::InputId(missing)],
                }],
            )]),
            &input_catalog_for_tests(),
            &request,
        )
        .expect("optional missing input selector must downgrade");
        let capability = prepared
            .capabilities
            .iter()
            .find(|capability| capability.id == id)
            .unwrap();
        assert_eq!(capability.status, PreparedCapabilityStatus::Unavailable);
        assert!(
            capability
                .message
                .as_deref()
                .is_some_and(|message| message.contains("matches no declared input"))
        );
        assert!(prepared.input_value_ids.is_empty());
    }

    #[test]
    fn prepared_observation_expands_input_families_to_sorted_concrete_ids() {
        let id = CapabilityId::new("test.input_family.v1").unwrap();
        let family = crate::SchemaFamilyId::new("glassvm.input.digital-key").unwrap();
        let mut request = ObservationRequest::summary();
        request.input_value_evidence.enabled = true;
        request.capabilities = vec![CapabilityRequest::required(id.clone())];
        let prepared = PreparedObservation::prepare_with_input_catalog(
            &test_catalog(vec![test_capability(
                id.as_str(),
                vec![CapabilityDependency::InputValueEvidence {
                    selectors: vec![InputValueSelector::SchemaFamily(family)],
                }],
            )]),
            &input_catalog_for_tests(),
            &request,
        )
        .expect("declared input family must prepare");
        assert_eq!(
            prepared.input_value_ids,
            vec![
                InputId::new("chip8.key.0").unwrap(),
                InputId::new("chip8.key.1").unwrap()
            ]
        );
    }
}
