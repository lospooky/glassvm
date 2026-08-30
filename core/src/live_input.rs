use std::fmt;

use crate::{
    InputCoordinate, InputId, InputInstanceId, InputValueEvidence, PreparedInputInterface,
    PreparedRun, SchemaRef, TypedInputPayload,
};

/// Result metadata for one live input that the machine accepted synchronously.
///
/// The acceptance does not contain capability output and does not itself emit
/// normalized or native evidence. Those remain separately negotiated channels.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveInputAcceptance {
    pub instance_id: InputInstanceId,
    pub application_coordinate: InputCoordinate,
}

/// Synchronous, session-scoped live-input application boundary.
pub trait LiveInputController: Send {
    fn apply(
        &mut self,
        input_id: InputId,
        payload: TypedInputPayload,
    ) -> Result<LiveInputAcceptance, LiveInputError>;
}

/// Optional bundle service that constructs one live controller for one
/// prepared run. It exposes no native binding or asynchronous delivery API.
pub trait LiveInputCapability: Send + Sync {
    fn create_controller(
        &self,
        prepared: &PreparedRun,
    ) -> Result<Box<dyn LiveInputController>, String>;
}

/// Bind the optional bundle service to a prepared run before execution.
///
/// A scheduled-only prepared run does not need a live-input service. A
/// prepared run with any live-capable input must have one; failure is reported
/// before session construction rather than discovered by a controller later.
pub fn create_live_input_controller(
    capability: Option<&dyn LiveInputCapability>,
    prepared: &PreparedRun,
) -> Result<Option<Box<dyn LiveInputController>>, String> {
    prepared.validate()?;
    if prepared.input_interface.live_inputs.is_empty() {
        return Ok(None);
    }
    let capability = capability.ok_or_else(|| {
        String::from(
            "prepared run declares live inputs but the bundle exposes no live-input capability",
        )
    })?;
    capability.create_controller(prepared).map(Some)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveInputError {
    UnsupportedInput {
        input_id: InputId,
    },
    PayloadSchemaMismatch {
        input_id: InputId,
        expected: SchemaRef,
        actual: SchemaRef,
    },
    InvalidPayload {
        input_id: InputId,
        reason: String,
    },
    CoordinateMismatch {
        input_id: InputId,
        reason: String,
    },
    InstanceIdExhausted,
    ApplicationRejected {
        input_id: InputId,
        reason: String,
    },
}

impl fmt::Display for LiveInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedInput { input_id } => {
                write!(formatter, "input {input_id} is not live-capable")
            }
            Self::PayloadSchemaMismatch {
                input_id,
                expected,
                actual,
            } => write!(
                formatter,
                "live input {input_id} payload schema {} {} does not match {} {}",
                actual.id, actual.version, expected.id, expected.version
            ),
            Self::InvalidPayload { input_id, reason } => {
                write!(
                    formatter,
                    "live input {input_id} has an invalid payload: {reason}"
                )
            }
            Self::CoordinateMismatch { input_id, reason } => write!(
                formatter,
                "live input {input_id} application coordinate is invalid: {reason}"
            ),
            Self::InstanceIdExhausted => write!(formatter, "live input instance IDs are exhausted"),
            Self::ApplicationRejected { input_id, reason } => {
                write!(formatter, "live input {input_id} was rejected: {reason}")
            }
        }
    }
}

impl std::error::Error for LiveInputError {}

/// Small bundle-internal helper for implementing a synchronous controller.
///
/// It owns only the prepared live interface and the next per-run instance ID.
/// A bundle validates with `validate_before_application`, applies the value to
/// its native machine state synchronously, and then calls
/// `record_application_acceptance` with the actual boundary coordinate.
#[derive(Debug, Clone)]
pub struct LiveInputAdmission {
    input_interface: PreparedInputInterface,
    next_instance_id: InputInstanceId,
}

impl LiveInputAdmission {
    pub fn new(prepared: &PreparedRun) -> Result<Self, String> {
        prepared.validate()?;
        Ok(Self {
            input_interface: prepared.input_interface.clone(),
            next_instance_id: InputInstanceId::FIRST,
        })
    }

    pub fn validate_before_application(
        &self,
        input_id: &InputId,
        payload: &TypedInputPayload,
    ) -> Result<(), LiveInputError> {
        let input = self
            .input_interface
            .live_inputs
            .get(input_id)
            .ok_or_else(|| LiveInputError::UnsupportedInput {
                input_id: input_id.clone(),
            })?;
        if payload.schema != input.payload_schema {
            return Err(LiveInputError::PayloadSchemaMismatch {
                input_id: input_id.clone(),
                expected: input.payload_schema.clone(),
                actual: payload.schema.clone(),
            });
        }
        payload
            .validate()
            .map_err(|reason| LiveInputError::InvalidPayload {
                input_id: input_id.clone(),
                reason,
            })?;
        if self.next_instance_id.next().is_none() {
            return Err(LiveInputError::InstanceIdExhausted);
        }
        Ok(())
    }

    pub fn record_application_acceptance(
        &mut self,
        input_id: &InputId,
        payload: &TypedInputPayload,
        application_coordinate: InputCoordinate,
    ) -> Result<LiveInputAcceptance, LiveInputError> {
        self.validate_before_application(input_id, payload)?;
        let input = self
            .input_interface
            .live_inputs
            .get(input_id)
            .expect("validation checked the prepared live-input interface");
        input
            .coordinate_policy
            .validate_coordinate(&application_coordinate)
            .map_err(|reason| LiveInputError::CoordinateMismatch {
                input_id: input_id.clone(),
                reason,
            })?;
        let instance_id = self.next_instance_id;
        self.next_instance_id = instance_id
            .next()
            .ok_or(LiveInputError::InstanceIdExhausted)?;
        Ok(LiveInputAcceptance {
            instance_id,
            application_coordinate,
        })
    }

    pub fn next_instance_id(&self) -> InputInstanceId {
        self.next_instance_id
    }
}

impl LiveInputAcceptance {
    /// Build the lightweight normalized fact after the bundle has accepted the
    /// input. This helper does not emit it or retain a history collection.
    pub fn input_applied(&self, input_id: InputId, input_schema: SchemaRef) -> crate::InputApplied {
        crate::InputApplied {
            input_id,
            input_schema,
            source: crate::InputSource::Live {
                instance_id: self.instance_id,
            },
            application_coordinate: self.application_coordinate.clone(),
        }
    }

    /// Build the explicitly requested full-value evidence for the same
    /// application. The caller still decides whether to emit it.
    pub fn input_value_evidence(
        &self,
        input_id: InputId,
        input_schema: SchemaRef,
        payload: TypedInputPayload,
    ) -> InputValueEvidence {
        InputValueEvidence::new(
            input_id,
            input_schema,
            crate::InputSource::Live {
                instance_id: self.instance_id,
            },
            self.application_coordinate.clone(),
            payload,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{
        ArtifactEncoding, ConfigField, ConfigFieldKind, CoordinatePolicy, ExecutionControls,
        ExecutionLimitCatalog, InputApplicationSemantics, InputCatalog, InputDeliveryMode,
        InputSchedule, InputSpec, MachineArtifactSpec, MachineConfigSchema, MachineConfiguration,
        MachineId, MachineIdentity, PreparedRun, RunId, SchemaVersion, StructuredValue,
        VersionStamp,
    };

    use super::*;

    fn schema(id: &str) -> SchemaRef {
        SchemaRef::new(id, SchemaVersion::V1)
    }

    fn prepared_run() -> PreparedRun {
        let artifact = MachineArtifactSpec {
            schema: schema("test.artifact"),
            encoding: ArtifactEncoding::RawBytes,
            min_bytes: 1,
            max_bytes: 8,
            alignment_bytes: 1,
            load_address: None,
        };
        let configuration_schema = MachineConfigSchema {
            schema: schema("test.configuration"),
            fields: vec![ConfigField {
                key: "mode".into(),
                label: "Mode".into(),
                description: "test mode".into(),
                kind: ConfigFieldKind::Enum {
                    options: vec!["a".into()],
                },
                default_value: serde_json::json!("a"),
            }],
        };
        let input_catalog = InputCatalog {
            schema: crate::input_catalog_schema(),
            inputs: vec![InputSpec {
                id: InputId::new("test.live").unwrap(),
                payload_schema: schema("test.input"),
                schema_family: None,
                coordinate_policy: CoordinatePolicy::frame_start(),
                delivery_modes: std::collections::BTreeSet::from([InputDeliveryMode::Live]),
                application: InputApplicationSemantics::new(schema("test.application")).unwrap(),
            }],
        };
        PreparedRun::prepare(
            RunId::from("run"),
            &artifact,
            &[1],
            MachineIdentity {
                machine_id: MachineId::from("test.machine"),
                bundle_version: VersionStamp::from("bundle-1"),
                machine_version: VersionStamp::from("machine-1"),
                emulator_version: VersionStamp::from("emulator-1"),
                contract_schema: schema("test.contract"),
            },
            &MachineConfiguration::new(
                schema("test.configuration"),
                StructuredValue::Map(BTreeMap::new()),
            )
            .unwrap(),
            &configuration_schema,
            &input_catalog,
            &InputSchedule {
                schema: crate::input_schedule_schema(),
                entries: Vec::new(),
            },
            &ExecutionControls::default(),
            &ExecutionLimitCatalog::empty(),
        )
        .unwrap()
    }

    fn payload(value: u64) -> TypedInputPayload {
        TypedInputPayload::new(schema("test.input"), StructuredValue::Unsigned(value)).unwrap()
    }

    #[test]
    fn admission_rejects_non_live_inputs_and_schema_mismatches_without_consuming_ids() {
        let prepared = prepared_run();
        let admission = LiveInputAdmission::new(&prepared).unwrap();
        let unknown = InputId::new("test.unknown").unwrap();
        assert!(matches!(
            admission.validate_before_application(&unknown, &payload(1)),
            Err(LiveInputError::UnsupportedInput { .. })
        ));
        let input_id = InputId::new("test.live").unwrap();
        let wrong_payload =
            TypedInputPayload::new(schema("test.other-input"), StructuredValue::Unsigned(1))
                .unwrap();
        assert!(matches!(
            admission.validate_before_application(&input_id, &wrong_payload),
            Err(LiveInputError::PayloadSchemaMismatch { .. })
        ));
        assert_eq!(admission.next_instance_id(), InputInstanceId::FIRST);
    }

    #[test]
    fn accepted_inputs_allocate_ids_at_zero_and_use_actual_coordinates() {
        let prepared = prepared_run();
        let mut admission = LiveInputAdmission::new(&prepared).unwrap();
        let input_id = InputId::new("test.live").unwrap();
        let first = admission
            .record_application_acceptance(&input_id, &payload(1), InputCoordinate::frame(3))
            .unwrap();
        let second = admission
            .record_application_acceptance(&input_id, &payload(2), InputCoordinate::frame(4))
            .unwrap();
        assert_eq!(first.instance_id, InputInstanceId::new(0));
        assert_eq!(second.instance_id, InputInstanceId::new(1));
        assert_eq!(first.application_coordinate, InputCoordinate::frame(3));
        assert_eq!(
            first
                .input_applied(input_id.clone(), schema("test.input"))
                .source,
            crate::InputSource::Live {
                instance_id: InputInstanceId::new(0)
            }
        );
        assert_eq!(
            first
                .input_value_evidence(input_id, schema("test.input"), payload(1))
                .payload
                .value,
            StructuredValue::Unsigned(1)
        );
    }

    #[test]
    fn coordinate_mismatch_is_rejected_before_instance_allocation() {
        let prepared = prepared_run();
        let mut admission = LiveInputAdmission::new(&prepared).unwrap();
        let input_id = InputId::new("test.live").unwrap();
        assert!(matches!(
            admission.record_application_acceptance(
                &input_id,
                &payload(1),
                InputCoordinate::step(1),
            ),
            Err(LiveInputError::CoordinateMismatch { .. })
        ));
        assert_eq!(admission.next_instance_id(), InputInstanceId::FIRST);
    }

    #[test]
    fn service_binding_rejects_live_runs_without_a_capability_but_allows_scheduled_only_runs() {
        let live_run = prepared_run();
        match create_live_input_controller(None, &live_run) {
            Err(error) => assert!(error.contains("no live-input capability")),
            Ok(_) => panic!("live run unexpectedly bound without a capability"),
        }

        let mut scheduled_run = prepared_run();
        scheduled_run.input_interface = PreparedInputInterface::empty();
        scheduled_run.identity = crate::PreparedRunIdentity::new(
            scheduled_run.identity.artifact_identity.clone(),
            scheduled_run.identity.machine_identity.clone(),
            scheduled_run.configuration.clone(),
            scheduled_run.input_schedule.clone(),
            scheduled_run.input_interface.clone(),
            scheduled_run.execution_controls.clone(),
        )
        .unwrap();
        assert!(
            create_live_input_controller(None, &scheduled_run)
                .unwrap()
                .is_none()
        );
    }
}
