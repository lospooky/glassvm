use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    ArtifactEncoding, ExecutionControls, ExecutionLimitCatalog, InputCatalog, InputCoordinate,
    InputDeliveryMode, InputId, InputSchedule, MachineArtifactSpec, MachineConfigSchema,
    MachineConfiguration, PreparedConfiguration, PreparedExecutionControls, ResolvedInputSchedule,
    RunId, SchemaRef, SchemaVersion, StructuredValue, VersionStamp, canonical_cbor_bytes,
};

pub const PREPARED_RUN_IDENTITY_SCHEMA_VERSION: SchemaVersion = SchemaVersion::V1;

pub fn prepared_run_identity_schema() -> SchemaRef {
    SchemaRef::new(
        "glassvm.prepared_run_identity",
        PREPARED_RUN_IDENTITY_SCHEMA_VERSION,
    )
}

/// Direct, schema-qualified identity material for an admitted machine
/// artifact. This is not a digest or authenticity token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedArtifactIdentity {
    pub schema: SchemaRef,
    pub encoding: ArtifactEncoding,
    pub canonical_value: Vec<u8>,
}

impl PreparedArtifactIdentity {
    pub fn from_spec(spec: &MachineArtifactSpec, artifact: &[u8]) -> Result<Self, String> {
        spec.validate(artifact)?;
        Self::new(spec.schema.clone(), spec.encoding.clone(), artifact)
    }

    pub fn new(
        schema: SchemaRef,
        encoding: ArtifactEncoding,
        artifact: &[u8],
    ) -> Result<Self, String> {
        validate_schema(&schema, "artifact identity")?;
        if let ArtifactEncoding::Extension(extension) = &encoding
            && extension.trim().is_empty()
        {
            return Err("artifact encoding extension must not be empty".into());
        }
        Ok(Self {
            schema,
            encoding,
            canonical_value: artifact.to_vec(),
        })
    }

    fn validate(&self) -> Result<(), String> {
        validate_schema(&self.schema, "artifact identity")?;
        if let ArtifactEncoding::Extension(extension) = &self.encoding
            && extension.trim().is_empty()
        {
            return Err("artifact encoding extension must not be empty".into());
        }
        Ok(())
    }
}

/// Machine and implementation versions whose semantics can affect execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineIdentity {
    pub machine_id: crate::MachineId,
    pub bundle_version: VersionStamp,
    pub machine_version: VersionStamp,
    pub emulator_version: VersionStamp,
    pub contract_schema: SchemaRef,
}

impl MachineIdentity {
    pub fn validate(&self) -> Result<(), String> {
        if self.machine_id.as_str().trim().is_empty() {
            return Err("machine identity has an empty machine ID".into());
        }
        for (label, version) in [
            ("bundle", &self.bundle_version),
            ("machine", &self.machine_version),
            ("emulator", &self.emulator_version),
        ] {
            if version.as_str().trim().is_empty() {
                return Err(format!("machine identity has an empty {label} version"));
            }
        }
        validate_schema(&self.contract_schema, "machine contract identity")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedLiveInput {
    pub input_id: InputId,
    pub payload_schema: SchemaRef,
    pub coordinate_policy: crate::CoordinatePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedInputInterface {
    pub live_inputs: BTreeMap<InputId, PreparedLiveInput>,
}

impl PreparedInputInterface {
    pub fn from_catalog(catalog: &InputCatalog) -> Result<Self, String> {
        catalog.validate()?;
        let live_inputs = catalog
            .inputs
            .iter()
            .filter(|input| input.delivery_modes.contains(&InputDeliveryMode::Live))
            .map(|input| {
                (
                    input.id.clone(),
                    PreparedLiveInput {
                        input_id: input.id.clone(),
                        payload_schema: input.payload_schema.clone(),
                        coordinate_policy: input.coordinate_policy.clone(),
                    },
                )
            })
            .collect();
        let interface = Self { live_inputs };
        interface.validate()?;
        Ok(interface)
    }

    pub fn empty() -> Self {
        Self {
            live_inputs: BTreeMap::new(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        for (key, input) in &self.live_inputs {
            if key != &input.input_id {
                return Err(format!(
                    "prepared live-input map key {} does not match entry {}",
                    key, input.input_id
                ));
            }
            validate_schema(&input.payload_schema, "prepared live-input payload")?;
            input.coordinate_policy.validate()?;
        }
        Ok(())
    }
}

/// The complete validated execution-side context. Observation and persistence
/// are intentionally absent from this object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedRun {
    pub run_id: RunId,
    pub machine_id: crate::MachineId,
    pub configuration: PreparedConfiguration,
    pub input_schedule: ResolvedInputSchedule,
    pub input_interface: PreparedInputInterface,
    pub execution_controls: PreparedExecutionControls,
    pub identity: PreparedRunIdentity,
}

impl PreparedRun {
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        run_id: RunId,
        artifact_spec: &MachineArtifactSpec,
        artifact: &[u8],
        machine_identity: MachineIdentity,
        configuration: &MachineConfiguration,
        configuration_schema: &MachineConfigSchema,
        input_catalog: &InputCatalog,
        input_schedule: &InputSchedule,
        execution_controls: &ExecutionControls,
        execution_limit_catalog: &ExecutionLimitCatalog,
    ) -> Result<Self, String> {
        if run_id.as_str().trim().is_empty() {
            return Err("prepared run has an empty run ID".into());
        }
        machine_identity.validate()?;
        let artifact_identity = PreparedArtifactIdentity::from_spec(artifact_spec, artifact)?;
        let prepared_configuration = configuration.prepare(configuration_schema)?;
        let resolved_schedule = input_schedule.resolve(input_catalog)?;
        let input_interface = PreparedInputInterface::from_catalog(input_catalog)?;
        let prepared_controls = execution_controls.prepare(execution_limit_catalog)?;
        let identity = PreparedRunIdentity::new(
            artifact_identity,
            machine_identity,
            prepared_configuration.clone(),
            resolved_schedule.clone(),
            input_interface.clone(),
            prepared_controls.clone(),
        )?;
        let prepared = Self {
            machine_id: identity.machine_identity.machine_id.clone(),
            run_id,
            configuration: prepared_configuration,
            input_schedule: resolved_schedule,
            input_interface,
            execution_controls: prepared_controls,
            identity,
        };
        prepared.validate()?;
        Ok(prepared)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.run_id.as_str().trim().is_empty() {
            return Err("prepared run has an empty run ID".into());
        }
        self.configuration.validate()?;
        self.input_schedule.validate_shape()?;
        self.input_interface.validate()?;
        self.execution_controls.validate()?;
        self.identity.validate()?;
        let expected = PreparedRunIdentity::new(
            self.identity.artifact_identity.clone(),
            self.identity.machine_identity.clone(),
            self.configuration.clone(),
            self.input_schedule.clone(),
            self.input_interface.clone(),
            self.execution_controls.clone(),
        )?;
        if self.identity != expected {
            return Err("prepared run identity does not match resolved execution state".into());
        }
        if self.machine_id != self.identity.machine_identity.machine_id {
            return Err("prepared run machine ID does not match machine identity".into());
        }
        Ok(())
    }
}

/// Canonical execution identity. Its bytes are deterministic identity
/// material only; they are not a checksum, signature, or audit mechanism.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedRunIdentity {
    pub artifact_identity: PreparedArtifactIdentity,
    pub machine_identity: MachineIdentity,
    pub configuration: PreparedConfiguration,
    pub resolved_input_schedule: ResolvedInputSchedule,
    pub prepared_input_interface: PreparedInputInterface,
    pub execution_controls: PreparedExecutionControls,
    #[serde(default)]
    canonical_bytes: Vec<u8>,
}

impl PreparedRunIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        artifact_identity: PreparedArtifactIdentity,
        machine_identity: MachineIdentity,
        configuration: PreparedConfiguration,
        resolved_input_schedule: ResolvedInputSchedule,
        prepared_input_interface: PreparedInputInterface,
        execution_controls: PreparedExecutionControls,
    ) -> Result<Self, String> {
        artifact_identity.validate()?;
        machine_identity.validate()?;
        configuration.validate()?;
        resolved_input_schedule.validate_shape()?;
        prepared_input_interface.validate()?;
        execution_controls.validate()?;
        let canonical_bytes = canonical_identity_bytes(
            &artifact_identity,
            &machine_identity,
            &configuration,
            &resolved_input_schedule,
            &prepared_input_interface,
            &execution_controls,
        )?;
        Ok(Self {
            artifact_identity,
            machine_identity,
            configuration,
            resolved_input_schedule,
            prepared_input_interface,
            execution_controls,
            canonical_bytes,
        })
    }

    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    pub fn validate(&self) -> Result<(), String> {
        let expected = Self::new(
            self.artifact_identity.clone(),
            self.machine_identity.clone(),
            self.configuration.clone(),
            self.resolved_input_schedule.clone(),
            self.prepared_input_interface.clone(),
            self.execution_controls.clone(),
        )?;
        if self.canonical_bytes != expected.canonical_bytes {
            return Err("prepared-run identity canonical bytes do not match envelope".into());
        }
        Ok(())
    }
}

fn canonical_identity_bytes(
    artifact: &PreparedArtifactIdentity,
    machine: &MachineIdentity,
    configuration: &PreparedConfiguration,
    schedule: &ResolvedInputSchedule,
    interface: &PreparedInputInterface,
    controls: &PreparedExecutionControls,
) -> Result<Vec<u8>, String> {
    let fields = vec![
        named_field("artifact_identity", artifact_value(artifact)),
        named_field("machine_identity", machine_value(machine)),
        named_field("configuration", configuration_value(configuration)),
        named_field("resolved_input_schedule", schedule_value(schedule)),
        named_field("prepared_input_interface", interface_value(interface)),
        named_field("execution_controls", controls_value(controls)),
    ];
    let mut envelope = BTreeMap::new();
    envelope.insert(
        "schema".into(),
        schema_value(&prepared_run_identity_schema()),
    );
    envelope.insert("fields".into(), StructuredValue::Array(fields));
    canonical_cbor_bytes(&StructuredValue::Map(envelope))
}

fn named_field(name: &str, value: StructuredValue) -> StructuredValue {
    let mut field = BTreeMap::new();
    field.insert("name".into(), StructuredValue::Text(name.into()));
    field.insert("value".into(), value);
    StructuredValue::Map(field)
}

fn artifact_value(artifact: &PreparedArtifactIdentity) -> StructuredValue {
    let mut value = BTreeMap::new();
    value.insert("schema".into(), schema_value(&artifact.schema));
    value.insert("encoding".into(), encoding_value(&artifact.encoding));
    value.insert(
        "canonical_value".into(),
        StructuredValue::Bytes(artifact.canonical_value.clone()),
    );
    StructuredValue::Map(value)
}

fn encoding_value(encoding: &ArtifactEncoding) -> StructuredValue {
    let mut value = BTreeMap::new();
    match encoding {
        ArtifactEncoding::RawBytes => {
            value.insert("kind".into(), StructuredValue::Text("raw_bytes".into()));
        }
        ArtifactEncoding::Extension(extension) => {
            value.insert("kind".into(), StructuredValue::Text("extension".into()));
            value.insert("value".into(), StructuredValue::Text(extension.clone()));
        }
    }
    StructuredValue::Map(value)
}

fn machine_value(machine: &MachineIdentity) -> StructuredValue {
    let mut value = BTreeMap::new();
    value.insert(
        "machine_id".into(),
        StructuredValue::Text(machine.machine_id.as_str().into()),
    );
    value.insert(
        "bundle_version".into(),
        StructuredValue::Text(machine.bundle_version.as_str().into()),
    );
    value.insert(
        "machine_version".into(),
        StructuredValue::Text(machine.machine_version.as_str().into()),
    );
    value.insert(
        "emulator_version".into(),
        StructuredValue::Text(machine.emulator_version.as_str().into()),
    );
    value.insert(
        "contract_schema".into(),
        schema_value(&machine.contract_schema),
    );
    StructuredValue::Map(value)
}

fn configuration_value(configuration: &PreparedConfiguration) -> StructuredValue {
    let mut value = BTreeMap::new();
    value.insert("schema".into(), schema_value(&configuration.schema));
    value.insert("value".into(), configuration.value.clone());
    StructuredValue::Map(value)
}

fn schedule_value(schedule: &ResolvedInputSchedule) -> StructuredValue {
    let entries = schedule
        .entries
        .iter()
        .map(|entry| {
            let mut value = BTreeMap::new();
            value.insert("ordinal".into(), StructuredValue::Unsigned(entry.ordinal));
            value.insert("coordinate".into(), coordinate_value(&entry.coordinate));
            value.insert(
                "input_id".into(),
                StructuredValue::Text(entry.input_id.as_str().into()),
            );
            value.insert(
                "payload".into(),
                payload_value(&entry.payload.schema, &entry.payload.value),
            );
            StructuredValue::Map(value)
        })
        .collect();
    let mut value = BTreeMap::new();
    value.insert("schema".into(), schema_value(&schedule.schema));
    value.insert("entries".into(), StructuredValue::Array(entries));
    StructuredValue::Map(value)
}

fn interface_value(interface: &PreparedInputInterface) -> StructuredValue {
    let live_inputs = interface
        .live_inputs
        .values()
        .map(|input| {
            let mut value = BTreeMap::new();
            value.insert(
                "input_id".into(),
                StructuredValue::Text(input.input_id.as_str().into()),
            );
            value.insert("payload_schema".into(), schema_value(&input.payload_schema));
            value.insert(
                "coordinate_policy".into(),
                policy_value(&input.coordinate_policy),
            );
            StructuredValue::Map(value)
        })
        .collect();
    let mut value = BTreeMap::new();
    value.insert("live_inputs".into(), StructuredValue::Array(live_inputs));
    StructuredValue::Map(value)
}

fn controls_value(controls: &PreparedExecutionControls) -> StructuredValue {
    let mut value = BTreeMap::new();
    value.insert("frame_limit".into(), optional_u64(controls.frame_limit));
    value.insert("step_limit".into(), optional_u64(controls.step_limit));
    value.insert(
        "bundle_limits".into(),
        StructuredValue::Array(
            controls
                .bundle_limits
                .iter()
                .map(|limit| {
                    let mut item = BTreeMap::new();
                    item.insert(
                        "limit_id".into(),
                        StructuredValue::Text(limit.limit_id.as_str().into()),
                    );
                    item.insert(
                        "value".into(),
                        payload_value(&limit.value.schema, &limit.value.value),
                    );
                    StructuredValue::Map(item)
                })
                .collect(),
        ),
    );
    StructuredValue::Map(value)
}

fn optional_u64(value: Option<u64>) -> StructuredValue {
    value.map_or(StructuredValue::Null, StructuredValue::Unsigned)
}

fn payload_value(schema: &SchemaRef, value: &StructuredValue) -> StructuredValue {
    let mut payload = BTreeMap::new();
    payload.insert("schema".into(), schema_value(schema));
    payload.insert("value".into(), value.clone());
    StructuredValue::Map(payload)
}

fn policy_value(policy: &crate::CoordinatePolicy) -> StructuredValue {
    let mut value = BTreeMap::new();
    value.insert("schema".into(), schema_value(&policy.schema));
    value.insert(
        "boundary".into(),
        StructuredValue::Text(
            match policy.boundary {
                crate::InputBoundaryKind::FrameStart => "frame_start",
                crate::InputBoundaryKind::Step => "step",
                crate::InputBoundaryKind::Cycle => "cycle",
                crate::InputBoundaryKind::Tick => "tick",
                crate::InputBoundaryKind::BundleDefined => "bundle_defined",
            }
            .into(),
        ),
    );
    StructuredValue::Map(value)
}

fn coordinate_value(coordinate: &InputCoordinate) -> StructuredValue {
    let mut value = BTreeMap::new();
    match coordinate {
        InputCoordinate::Frame { frame } => {
            value.insert("kind".into(), StructuredValue::Text("frame".into()));
            value.insert("value".into(), StructuredValue::Unsigned(*frame));
        }
        InputCoordinate::Step { step } => {
            value.insert("kind".into(), StructuredValue::Text("step".into()));
            value.insert("value".into(), StructuredValue::Unsigned(*step));
        }
        InputCoordinate::Cycle { cycle } => {
            value.insert("kind".into(), StructuredValue::Text("cycle".into()));
            value.insert("value".into(), StructuredValue::Unsigned(*cycle));
        }
        InputCoordinate::Tick { tick } => {
            value.insert("kind".into(), StructuredValue::Text("tick".into()));
            value.insert("value".into(), StructuredValue::Unsigned(*tick));
        }
        InputCoordinate::BundleDefined {
            schema,
            value: item,
        } => {
            value.insert(
                "kind".into(),
                StructuredValue::Text("bundle_defined".into()),
            );
            value.insert("schema".into(), schema_value(schema));
            value.insert("value".into(), item.clone());
        }
    }
    StructuredValue::Map(value)
}

fn schema_value(schema: &SchemaRef) -> StructuredValue {
    let mut version = BTreeMap::new();
    version.insert(
        "major".into(),
        StructuredValue::Unsigned(schema.version.major as u64),
    );
    version.insert(
        "minor".into(),
        StructuredValue::Unsigned(schema.version.minor as u64),
    );
    version.insert(
        "patch".into(),
        StructuredValue::Unsigned(schema.version.patch as u64),
    );
    let mut value = BTreeMap::new();
    value.insert("id".into(), StructuredValue::Text(schema.id.clone()));
    value.insert("version".into(), StructuredValue::Map(version));
    StructuredValue::Map(value)
}

fn validate_schema(schema: &SchemaRef, kind: &str) -> Result<(), String> {
    if schema.id.trim().is_empty() {
        return Err(format!("{kind} has an empty schema ID"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::json;

    use super::*;
    use crate::{
        ConfigField, ConfigFieldKind, CoordinatePolicy, InputApplicationSemantics, InputSpec,
        ScheduledInput, TypedInputPayload,
    };

    fn schema(id: &str) -> SchemaRef {
        SchemaRef::new(id, SchemaVersion::V1)
    }

    fn artifact_spec() -> MachineArtifactSpec {
        MachineArtifactSpec {
            schema: schema("test.artifact"),
            encoding: ArtifactEncoding::RawBytes,
            min_bytes: 1,
            max_bytes: 8,
            alignment_bytes: 1,
            load_address: None,
        }
    }

    fn config_schema() -> MachineConfigSchema {
        MachineConfigSchema {
            schema: schema("test.configuration"),
            fields: vec![ConfigField {
                key: "mode".into(),
                label: "Mode".into(),
                description: "test mode".into(),
                kind: ConfigFieldKind::Enum {
                    options: vec!["a".into(), "b".into()],
                },
                default_value: json!("a"),
            }],
        }
    }

    fn input_catalog() -> InputCatalog {
        InputCatalog {
            schema: crate::input_catalog_schema(),
            inputs: vec![InputSpec {
                id: InputId::new("test.live").unwrap(),
                payload_schema: schema("test.input"),
                schema_family: None,
                coordinate_policy: CoordinatePolicy::frame_start(),
                delivery_modes: BTreeSet::from([InputDeliveryMode::Live]),
                application: InputApplicationSemantics::new(schema("test.application")).unwrap(),
            }],
        }
    }

    fn machine_identity() -> MachineIdentity {
        MachineIdentity {
            machine_id: crate::MachineId::from("test.machine"),
            bundle_version: VersionStamp::from("bundle-1"),
            machine_version: VersionStamp::from("machine-1"),
            emulator_version: VersionStamp::from("emulator-1"),
            contract_schema: schema("test.contract"),
        }
    }

    #[test]
    fn preparation_resolves_defaults_schedule_interface_and_empty_limits() {
        let configuration = MachineConfiguration::new(
            schema("test.configuration"),
            StructuredValue::Map(BTreeMap::new()),
        )
        .unwrap();
        let schedule = InputSchedule {
            schema: crate::input_schedule_schema(),
            entries: vec![],
        };
        let prepared = PreparedRun::prepare(
            RunId::from("run"),
            &artifact_spec(),
            &[1, 2],
            machine_identity(),
            &configuration,
            &config_schema(),
            &input_catalog(),
            &schedule,
            &ExecutionControls::default(),
            &ExecutionLimitCatalog::empty(),
        )
        .unwrap();

        assert_eq!(
            prepared.configuration.value,
            StructuredValue::Map(BTreeMap::from([(
                "mode".into(),
                StructuredValue::Text("a".into())
            ),]))
        );
        assert!(prepared.input_schedule.entries.is_empty());
        assert!(
            prepared
                .input_interface
                .live_inputs
                .contains_key(&InputId::new("test.live").unwrap())
        );
        assert!(prepared.execution_controls.bundle_limits.is_empty());
        assert!(!prepared.identity.canonical_bytes().is_empty());
    }

    #[test]
    fn identity_excludes_observation_and_changes_for_execution_inputs() {
        let configuration = MachineConfiguration::new(
            schema("test.configuration"),
            StructuredValue::Map(BTreeMap::new()),
        )
        .unwrap();
        let schedule = InputSchedule {
            schema: crate::input_schedule_schema(),
            entries: vec![],
        };
        let first = PreparedRun::prepare(
            RunId::from("run-a"),
            &artifact_spec(),
            &[1, 2],
            machine_identity(),
            &configuration,
            &config_schema(),
            &input_catalog(),
            &schedule,
            &ExecutionControls::default(),
            &ExecutionLimitCatalog::empty(),
        )
        .unwrap();
        let second = PreparedRun::prepare(
            RunId::from("run-b"),
            &artifact_spec(),
            &[1, 3],
            machine_identity(),
            &configuration,
            &config_schema(),
            &input_catalog(),
            &schedule,
            &ExecutionControls::default(),
            &ExecutionLimitCatalog::empty(),
        )
        .unwrap();
        assert_eq!(
            first.identity,
            PreparedRun::prepare(
                RunId::from("different-run"),
                &artifact_spec(),
                &[1, 2],
                machine_identity(),
                &configuration,
                &config_schema(),
                &input_catalog(),
                &schedule,
                &ExecutionControls::default(),
                &ExecutionLimitCatalog::empty(),
            )
            .unwrap()
            .identity
        );
        assert_ne!(
            first.identity.canonical_bytes(),
            second.identity.canonical_bytes()
        );
    }

    #[test]
    fn invalid_artifact_and_configuration_fail_before_preparation() {
        let configuration = MachineConfiguration::new(
            schema("wrong.configuration"),
            StructuredValue::Map(BTreeMap::new()),
        )
        .unwrap();
        let error = PreparedRun::prepare(
            RunId::from("run"),
            &artifact_spec(),
            &[],
            machine_identity(),
            &configuration,
            &config_schema(),
            &input_catalog(),
            &InputSchedule {
                schema: crate::input_schedule_schema(),
                entries: vec![],
            },
            &ExecutionControls::default(),
            &ExecutionLimitCatalog::empty(),
        )
        .unwrap_err();
        assert!(error.contains("minimum") || error.contains("schema"));
    }

    #[test]
    fn preparation_rejects_unknown_inputs_invalid_coordinates_and_bad_payloads() {
        let configuration = MachineConfiguration::new(
            schema("test.configuration"),
            StructuredValue::Map(BTreeMap::new()),
        )
        .unwrap();
        let scheduled_id = InputId::new("test.scheduled").unwrap();
        let catalog = InputCatalog {
            schema: crate::input_catalog_schema(),
            inputs: vec![InputSpec {
                id: scheduled_id.clone(),
                payload_schema: schema("test.input"),
                schema_family: None,
                coordinate_policy: CoordinatePolicy::frame_start(),
                delivery_modes: BTreeSet::from([InputDeliveryMode::Scheduled]),
                application: InputApplicationSemantics::new(schema("test.application")).unwrap(),
            }],
        };
        let payload =
            || TypedInputPayload::new(schema("test.input"), StructuredValue::Unsigned(1)).unwrap();
        let prepare = |schedule: InputSchedule| {
            PreparedRun::prepare(
                RunId::from("run"),
                &artifact_spec(),
                &[1],
                machine_identity(),
                &configuration,
                &config_schema(),
                &catalog,
                &schedule,
                &ExecutionControls::default(),
                &ExecutionLimitCatalog::empty(),
            )
        };

        let unknown = InputSchedule {
            schema: crate::input_schedule_schema(),
            entries: vec![ScheduledInput {
                ordinal: 0,
                coordinate: InputCoordinate::frame(0),
                input_id: InputId::new("test.unknown").unwrap(),
                payload: payload(),
            }],
        };
        assert!(prepare(unknown).unwrap_err().contains("undeclared input"));

        let invalid_coordinate = InputSchedule {
            schema: crate::input_schedule_schema(),
            entries: vec![ScheduledInput {
                ordinal: 0,
                coordinate: InputCoordinate::step(0),
                input_id: scheduled_id,
                payload: payload(),
            }],
        };
        assert!(
            prepare(invalid_coordinate)
                .unwrap_err()
                .contains("does not match")
        );

        let malformed_payload = InputSchedule {
            schema: crate::input_schedule_schema(),
            entries: vec![ScheduledInput {
                ordinal: 0,
                coordinate: InputCoordinate::frame(0),
                input_id: InputId::new("test.scheduled").unwrap(),
                payload: TypedInputPayload {
                    schema: SchemaRef::new("", SchemaVersion::V1),
                    value: StructuredValue::Unsigned(1),
                },
            }],
        };
        assert!(
            prepare(malformed_payload)
                .unwrap_err()
                .contains("empty schema ID")
        );
    }

    #[test]
    fn schedule_entries_are_identity_ordered_by_ordinal() {
        let input_id = InputId::new("test.scheduled").unwrap();
        let catalog = InputCatalog {
            schema: crate::input_catalog_schema(),
            inputs: vec![InputSpec {
                id: input_id.clone(),
                payload_schema: schema("test.input"),
                schema_family: None,
                coordinate_policy: CoordinatePolicy::frame_start(),
                delivery_modes: BTreeSet::from([InputDeliveryMode::Scheduled]),
                application: InputApplicationSemantics::new(schema("test.application")).unwrap(),
            }],
        };
        let payload =
            || TypedInputPayload::new(schema("test.input"), StructuredValue::Unsigned(1)).unwrap();
        let schedule_a = InputSchedule {
            schema: crate::input_schedule_schema(),
            entries: vec![
                ScheduledInput {
                    ordinal: 1,
                    coordinate: InputCoordinate::frame(1),
                    input_id: input_id.clone(),
                    payload: payload(),
                },
                ScheduledInput {
                    ordinal: 0,
                    coordinate: InputCoordinate::frame(0),
                    input_id: input_id.clone(),
                    payload: payload(),
                },
            ],
        };
        let schedule_b = InputSchedule {
            schema: crate::input_schedule_schema(),
            entries: vec![
                ScheduledInput {
                    ordinal: 0,
                    coordinate: InputCoordinate::frame(0),
                    input_id: input_id.clone(),
                    payload: payload(),
                },
                ScheduledInput {
                    ordinal: 1,
                    coordinate: InputCoordinate::frame(1),
                    input_id,
                    payload: payload(),
                },
            ],
        };
        let first = schedule_a.resolve(&catalog).unwrap();
        let second = schedule_b.resolve(&catalog).unwrap();
        assert_eq!(first, second);
    }
}
