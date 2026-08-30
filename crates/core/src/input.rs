use std::collections::{BTreeSet, HashSet};

use serde::{Deserialize, Serialize};

use crate::{
    InputId, InputInstanceId, SchemaFamilyId, SchemaRef, SchemaVersion, StructuredValue,
    canonical_cbor_bytes,
};

pub const INPUT_CONTRACT_SCHEMA_VERSION: SchemaVersion = SchemaVersion::V1;

pub fn input_catalog_schema() -> SchemaRef {
    SchemaRef::new("glassvm.input_catalog", INPUT_CONTRACT_SCHEMA_VERSION)
}

pub fn frame_coordinate_schema() -> SchemaRef {
    SchemaRef::new("glassvm.coordinate.frame", INPUT_CONTRACT_SCHEMA_VERSION)
}

pub fn step_coordinate_schema() -> SchemaRef {
    SchemaRef::new("glassvm.coordinate.step", INPUT_CONTRACT_SCHEMA_VERSION)
}

pub fn cycle_coordinate_schema() -> SchemaRef {
    SchemaRef::new("glassvm.coordinate.cycle", INPUT_CONTRACT_SCHEMA_VERSION)
}

pub fn tick_coordinate_schema() -> SchemaRef {
    SchemaRef::new("glassvm.coordinate.tick", INPUT_CONTRACT_SCHEMA_VERSION)
}

pub fn input_schedule_schema() -> SchemaRef {
    SchemaRef::new("glassvm.input_schedule", INPUT_CONTRACT_SCHEMA_VERSION)
}

/// A schema-qualified input value. The value is intentionally narrower than
/// arbitrary JSON so its canonical CBOR representation is stable for replay
/// and continuation identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedInputPayload {
    pub schema: SchemaRef,
    pub value: StructuredValue,
}

impl TypedInputPayload {
    pub fn new(schema: SchemaRef, value: StructuredValue) -> Result<Self, String> {
        let payload = Self { schema, value };
        payload.validate()?;
        Ok(payload)
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_schema(&self.schema, "input payload")?;
        self.canonical_cbor_bytes().map(|_| ())
    }

    /// Encode the schema and value as one explicit canonical CBOR map.
    pub fn canonical_cbor_bytes(&self) -> Result<Vec<u8>, String> {
        let mut fields = std::collections::BTreeMap::new();
        fields.insert("schema".to_owned(), schema_as_value(&self.schema));
        fields.insert("value".to_owned(), self.value.clone());
        canonical_cbor_bytes(&StructuredValue::Map(fields))
    }
}

fn schema_as_value(schema: &SchemaRef) -> StructuredValue {
    let mut version = std::collections::BTreeMap::new();
    version.insert(
        "major".to_owned(),
        StructuredValue::Unsigned(schema.version.major as u64),
    );
    version.insert(
        "minor".to_owned(),
        StructuredValue::Unsigned(schema.version.minor as u64),
    );
    version.insert(
        "patch".to_owned(),
        StructuredValue::Unsigned(schema.version.patch as u64),
    );
    let mut fields = std::collections::BTreeMap::new();
    fields.insert("id".to_owned(), StructuredValue::Text(schema.id.clone()));
    fields.insert("version".to_owned(), StructuredValue::Map(version));
    StructuredValue::Map(fields)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputSchedule {
    pub schema: SchemaRef,
    pub entries: Vec<ScheduledInput>,
}

impl InputSchedule {
    /// The explicit no-input schedule used when a caller supplies no
    /// scheduled inputs. Keeping this as a real contract value avoids a
    /// second implicit input policy in preparation.
    pub fn empty() -> Self {
        Self {
            schema: input_schedule_schema(),
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduledInput {
    pub ordinal: u64,
    pub coordinate: InputCoordinate,
    pub input_id: InputId,
    pub payload: TypedInputPayload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedInputSchedule {
    pub schema: SchemaRef,
    pub entries: Vec<ScheduledInput>,
}

impl ResolvedInputSchedule {
    /// Validate the shape of a schedule that has already been resolved
    /// against an input catalog. Catalog membership and delivery-mode checks
    /// belong to `InputSchedule::resolve`; this method verifies the immutable
    /// prepared representation itself.
    pub fn validate_shape(&self) -> Result<(), String> {
        if self.schema != input_schedule_schema() {
            return Err(format!(
                "unsupported resolved input schedule schema {} {}; expected {} {}",
                self.schema.id,
                self.schema.version,
                input_schedule_schema().id,
                input_schedule_schema().version
            ));
        }
        for window in self.entries.windows(2) {
            if window[0].ordinal >= window[1].ordinal {
                return Err("resolved input schedule must be sorted by unique ordinal".into());
            }
        }
        for entry in &self.entries {
            entry.payload.validate()?;
        }
        Ok(())
    }
}

impl InputSchedule {
    /// Validate schedule-local structure before catalog-dependent resolution.
    /// Input IDs and coordinate policies are checked by `resolve`.
    pub fn validate_shape(&self) -> Result<(), String> {
        if self.schema != input_schedule_schema() {
            return Err(format!(
                "unsupported input schedule schema {} {}; expected {} {}",
                self.schema.id,
                self.schema.version,
                input_schedule_schema().id,
                input_schedule_schema().version
            ));
        }
        let mut ordinals = BTreeSet::new();
        for entry in &self.entries {
            if !ordinals.insert(entry.ordinal) {
                return Err(format!("input schedule repeats ordinal {}", entry.ordinal));
            }
            entry.payload.validate()?;
            if let InputCoordinate::BundleDefined { schema, value } = &entry.coordinate {
                validate_schema(schema, "bundle-defined input coordinate")?;
                canonical_cbor_bytes(value)?;
            }
        }
        Ok(())
    }

    /// Validate and resolve entries into authoritative ordinal order. Caller
    /// insertion order is deliberately not part of the resolved schedule.
    pub fn resolve(&self, catalog: &InputCatalog) -> Result<ResolvedInputSchedule, String> {
        self.validate_shape()?;
        catalog.validate()?;
        let mut entries = self.entries.clone();
        for entry in &entries {
            let input = catalog.find(&entry.input_id).ok_or_else(|| {
                format!(
                    "input schedule references undeclared input {}",
                    entry.input_id
                )
            })?;
            if !input.delivery_modes.contains(&InputDeliveryMode::Scheduled) {
                return Err(format!(
                    "input {} does not advertise scheduled delivery",
                    entry.input_id
                ));
            }
            input
                .coordinate_policy
                .validate_coordinate(&entry.coordinate)?;
            if entry.payload.schema != input.payload_schema {
                return Err(format!(
                    "input {} payload schema {} {} does not match declared schema {} {}",
                    entry.input_id,
                    entry.payload.schema.id,
                    entry.payload.schema.version,
                    input.payload_schema.id,
                    input.payload_schema.version
                ));
            }
            entry.payload.validate()?;
        }
        entries.sort_by_key(|entry| entry.ordinal);
        Ok(ResolvedInputSchedule {
            schema: self.schema.clone(),
            entries,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputApplied {
    pub input_id: InputId,
    pub input_schema: SchemaRef,
    pub source: InputSource,
    pub application_coordinate: InputCoordinate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputValueEvidence {
    pub input_id: InputId,
    pub input_schema: SchemaRef,
    pub source: InputSource,
    pub application_coordinate: InputCoordinate,
    pub payload: TypedInputPayload,
}

impl InputValueEvidence {
    pub fn new(
        input_id: InputId,
        input_schema: SchemaRef,
        source: InputSource,
        application_coordinate: InputCoordinate,
        payload: TypedInputPayload,
    ) -> Self {
        Self {
            input_id,
            input_schema,
            source,
            application_coordinate,
            payload,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        self.payload.validate()?;
        if self.payload.schema != self.input_schema {
            return Err("input-value payload schema does not match input schema".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum InputSource {
    Scheduled { ordinal: u64 },
    Live { instance_id: InputInstanceId },
}

/// A semantic machine coordinate used by scheduled and live input.
///
/// These variants are intentionally not aliases. A bundle may map two
/// coordinates onto the same work unit internally, but the public contract
/// preserves their distinct meanings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum InputCoordinate {
    Frame {
        frame: u64,
    },
    Step {
        step: u64,
    },
    Cycle {
        cycle: u64,
    },
    Tick {
        tick: u64,
    },
    BundleDefined {
        schema: SchemaRef,
        value: StructuredValue,
    },
}

impl InputCoordinate {
    pub const fn frame(frame: u64) -> Self {
        Self::Frame { frame }
    }

    pub const fn step(step: u64) -> Self {
        Self::Step { step }
    }

    pub const fn cycle(cycle: u64) -> Self {
        Self::Cycle { cycle }
    }

    pub const fn tick(tick: u64) -> Self {
        Self::Tick { tick }
    }

    pub fn bundle_defined(schema: SchemaRef, value: StructuredValue) -> Self {
        Self::BundleDefined { schema, value }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputBoundaryKind {
    FrameStart,
    Step,
    Cycle,
    Tick,
    BundleDefined,
}

/// Versioned declaration of the boundary at which an input is eligible for
/// application. The policy describes boundary semantics, not work quantum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoordinatePolicy {
    pub schema: SchemaRef,
    pub boundary: InputBoundaryKind,
}

impl CoordinatePolicy {
    pub fn new(schema: SchemaRef, boundary: InputBoundaryKind) -> Result<Self, String> {
        let policy = Self { schema, boundary };
        policy.validate()?;
        Ok(policy)
    }

    pub fn frame_start() -> Self {
        Self {
            schema: frame_coordinate_schema(),
            boundary: InputBoundaryKind::FrameStart,
        }
    }

    pub fn step() -> Self {
        Self {
            schema: step_coordinate_schema(),
            boundary: InputBoundaryKind::Step,
        }
    }

    pub fn cycle() -> Self {
        Self {
            schema: cycle_coordinate_schema(),
            boundary: InputBoundaryKind::Cycle,
        }
    }

    pub fn tick() -> Self {
        Self {
            schema: tick_coordinate_schema(),
            boundary: InputBoundaryKind::Tick,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema.id.trim().is_empty() {
            return Err("coordinate policy has an empty schema ID".into());
        }
        if let Some(expected) = self.built_in_schema() {
            if self.schema != expected {
                return Err(format!(
                    "coordinate policy schema {:?} does not match {:?} boundary; expected {:?}",
                    self.schema, self.boundary, expected
                ));
            }
        }
        Ok(())
    }

    pub fn validate_coordinate(&self, coordinate: &InputCoordinate) -> Result<(), String> {
        self.validate()?;
        let valid = match (self.boundary, coordinate) {
            (InputBoundaryKind::FrameStart, InputCoordinate::Frame { .. })
            | (InputBoundaryKind::Step, InputCoordinate::Step { .. })
            | (InputBoundaryKind::Cycle, InputCoordinate::Cycle { .. })
            | (InputBoundaryKind::Tick, InputCoordinate::Tick { .. }) => true,
            (InputBoundaryKind::BundleDefined, InputCoordinate::BundleDefined { schema, .. }) => {
                schema == &self.schema
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(format!(
                "input coordinate does not match {:?} policy {}",
                self.boundary, self.schema.id
            ))
        }
    }

    fn built_in_schema(&self) -> Option<SchemaRef> {
        match self.boundary {
            InputBoundaryKind::FrameStart => Some(frame_coordinate_schema()),
            InputBoundaryKind::Step => Some(step_coordinate_schema()),
            InputBoundaryKind::Cycle => Some(cycle_coordinate_schema()),
            InputBoundaryKind::Tick => Some(tick_coordinate_schema()),
            InputBoundaryKind::BundleDefined => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameBoundaryKind {
    Hardware,
    WorkGrouped,
    Callback,
    SampleGrouped,
    Synthetic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameCoordinateDescriptor {
    pub schema: SchemaRef,
    pub boundary: FrameBoundaryKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepUnit {
    Instruction,
    Action,
    Sample,
    Tick,
    BundleDefined(SchemaRef),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepCoordinateDescriptor {
    pub schema: SchemaRef,
    pub semantic_unit: StepUnit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionCoordinateCatalog {
    pub frame: FrameCoordinateDescriptor,
    pub step: Option<StepCoordinateDescriptor>,
}

impl ExecutionCoordinateCatalog {
    pub fn validate(&self) -> Result<(), String> {
        validate_schema(&self.frame.schema, "frame coordinate")?;
        if let Some(step) = &self.step {
            validate_schema(&step.schema, "step coordinate")?;
            if matches!(step.semantic_unit, StepUnit::BundleDefined(ref schema) if schema != &step.schema)
            {
                return Err("bundle-defined step unit must use the step coordinate schema".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputDeliveryMode {
    Scheduled,
    Live,
}

/// Bundle-owned versioned semantics for how an accepted input changes machine
/// state. GlassVM carries the schema reference but does not define a universal
/// actuation language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputApplicationSemantics {
    pub schema: SchemaRef,
}

impl InputApplicationSemantics {
    pub fn new(schema: SchemaRef) -> Result<Self, String> {
        validate_schema(&schema, "input application")?;
        Ok(Self { schema })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputSpec {
    pub id: crate::InputId,
    pub payload_schema: SchemaRef,
    pub schema_family: Option<SchemaFamilyId>,
    pub coordinate_policy: CoordinatePolicy,
    pub delivery_modes: BTreeSet<InputDeliveryMode>,
    pub application: InputApplicationSemantics,
}

impl InputSpec {
    pub fn validate(&self) -> Result<(), String> {
        validate_schema(&self.payload_schema, "input payload")?;
        self.coordinate_policy.validate()?;
        self.application.validate()?;
        if self.delivery_modes.is_empty() {
            return Err(format!("input {} declares no delivery mode", self.id));
        }
        Ok(())
    }
}

impl InputApplicationSemantics {
    fn validate(&self) -> Result<(), String> {
        validate_schema(&self.schema, "input application")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputCatalog {
    pub schema: SchemaRef,
    pub inputs: Vec<InputSpec>,
}

impl InputCatalog {
    pub fn empty() -> Self {
        Self {
            schema: input_catalog_schema(),
            inputs: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_schema(&self.schema, "input catalog")?;
        let mut seen = HashSet::new();
        for input in &self.inputs {
            input.validate()?;
            if !seen.insert(input.id.clone()) {
                return Err(format!(
                    "input catalog declares {} more than once",
                    input.id
                ));
            }
        }
        Ok(())
    }

    pub fn find(&self, id: &crate::InputId) -> Option<&InputSpec> {
        self.inputs.iter().find(|input| &input.id == id)
    }

    pub fn family_ids(&self, family: &SchemaFamilyId) -> Vec<crate::InputId> {
        let mut ids = self
            .inputs
            .iter()
            .filter(|input| input.schema_family.as_ref() == Some(family))
            .map(|input| input.id.clone())
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }
}

fn validate_schema(schema: &SchemaRef, kind: &str) -> Result<(), String> {
    if schema.id.trim().is_empty() {
        return Err(format!("{kind} has an empty schema ID"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InputId, SchemaVersion};

    fn schema(id: &str) -> SchemaRef {
        SchemaRef::new(id, SchemaVersion::V1)
    }

    fn input(id: &str, family: Option<SchemaFamilyId>) -> InputSpec {
        InputSpec {
            id: InputId::new(id).unwrap(),
            payload_schema: schema("test.input.payload"),
            schema_family: family,
            coordinate_policy: CoordinatePolicy::frame_start(),
            delivery_modes: BTreeSet::from([InputDeliveryMode::Scheduled]),
            application: InputApplicationSemantics::new(schema("test.input.application")).unwrap(),
        }
    }

    #[test]
    fn coordinate_policies_require_matching_versioned_boundaries() {
        assert!(
            CoordinatePolicy::frame_start()
                .validate_coordinate(&InputCoordinate::frame(0))
                .is_ok()
        );
        assert!(
            CoordinatePolicy::frame_start()
                .validate_coordinate(&InputCoordinate::step(0))
                .is_err()
        );
        assert!(CoordinatePolicy::new(schema("wrong"), InputBoundaryKind::FrameStart).is_err());

        let bundle_schema = schema("test.coordinate.sweep");
        let policy =
            CoordinatePolicy::new(bundle_schema.clone(), InputBoundaryKind::BundleDefined).unwrap();
        assert!(
            policy
                .validate_coordinate(&InputCoordinate::bundle_defined(
                    bundle_schema,
                    StructuredValue::Unsigned(0)
                ))
                .is_ok()
        );
    }

    #[test]
    fn input_catalog_rejects_duplicate_ids_and_empty_delivery_modes() {
        let family = SchemaFamilyId::new("glassvm.input.digital-key").unwrap();
        let first = input("chip8.key.0", Some(family.clone()));
        let mut empty_modes = input("chip8.key.1", Some(family));
        empty_modes.delivery_modes.clear();
        let catalog = InputCatalog {
            schema: input_catalog_schema(),
            inputs: vec![first.clone(), first],
        };
        assert!(catalog.validate().is_err());
        let catalog = InputCatalog {
            schema: input_catalog_schema(),
            inputs: vec![empty_modes],
        };
        assert!(catalog.validate().is_err());
    }

    #[test]
    fn family_membership_is_explicit_and_resolves_in_canonical_id_order() {
        let family = SchemaFamilyId::new("glassvm.input.digital-key").unwrap();
        let catalog = InputCatalog {
            schema: input_catalog_schema(),
            inputs: vec![
                input("chip8.key.1", Some(family.clone())),
                input("chip8.reset", None),
                input("chip8.key.0", Some(family.clone())),
            ],
        };
        catalog.validate().unwrap();
        assert_eq!(
            catalog.family_ids(&family),
            vec![
                InputId::new("chip8.key.0").unwrap(),
                InputId::new("chip8.key.1").unwrap()
            ]
        );
    }

    #[test]
    fn coordinate_descriptors_allow_frame_without_step() {
        let catalog = ExecutionCoordinateCatalog {
            frame: FrameCoordinateDescriptor {
                schema: frame_coordinate_schema(),
                boundary: FrameBoundaryKind::Hardware,
            },
            step: None,
        };
        assert!(catalog.validate().is_ok());
    }

    #[test]
    fn schedules_resolve_by_ordinal_not_caller_insertion_order() {
        let catalog = InputCatalog {
            schema: input_catalog_schema(),
            inputs: vec![input("chip8.key.0", None)],
        };
        let payload =
            TypedInputPayload::new(schema("test.input.payload"), StructuredValue::Unsigned(1))
                .unwrap();
        let schedule = InputSchedule {
            schema: input_schedule_schema(),
            entries: vec![
                ScheduledInput {
                    ordinal: 2,
                    coordinate: InputCoordinate::frame(1),
                    input_id: InputId::new("chip8.key.0").unwrap(),
                    payload: payload.clone(),
                },
                ScheduledInput {
                    ordinal: 1,
                    coordinate: InputCoordinate::frame(1),
                    input_id: InputId::new("chip8.key.0").unwrap(),
                    payload,
                },
            ],
        };
        let resolved = schedule.resolve(&catalog).unwrap();
        assert_eq!(
            resolved
                .entries
                .iter()
                .map(|entry| entry.ordinal)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn empty_schedule_is_an_explicit_valid_no_input_contract() {
        let schedule = InputSchedule::empty();
        assert!(schedule.validate_shape().is_ok());
        assert!(
            schedule
                .resolve(&InputCatalog::empty())
                .unwrap()
                .entries
                .is_empty()
        );
    }

    #[test]
    fn schedule_shape_rejects_duplicate_ordinals_and_malformed_payloads() {
        let payload = TypedInputPayload {
            schema: SchemaRef::new("", SchemaVersion::V1),
            value: StructuredValue::Unsigned(1),
        };
        let entry = ScheduledInput {
            ordinal: 0,
            coordinate: InputCoordinate::frame(0),
            input_id: InputId::new("test.input").unwrap(),
            payload,
        };
        let duplicate = InputSchedule {
            schema: input_schedule_schema(),
            entries: vec![entry.clone(), entry],
        };
        assert!(duplicate.validate_shape().is_err());
    }

    #[test]
    fn schedules_reject_duplicate_ordinals_and_payload_schema_mismatches() {
        let catalog = InputCatalog {
            schema: input_catalog_schema(),
            inputs: vec![input("chip8.key.0", None)],
        };
        let entry = |ordinal, payload_schema| ScheduledInput {
            ordinal,
            coordinate: InputCoordinate::frame(0),
            input_id: InputId::new("chip8.key.0").unwrap(),
            payload: TypedInputPayload::new(schema(payload_schema), StructuredValue::Unsigned(1))
                .unwrap(),
        };
        assert!(
            InputSchedule {
                schema: input_schedule_schema(),
                entries: vec![
                    entry(0, "test.input.payload"),
                    entry(0, "test.input.payload")
                ],
            }
            .resolve(&catalog)
            .is_err()
        );
        assert!(
            InputSchedule {
                schema: input_schedule_schema(),
                entries: vec![entry(0, "other.payload")],
            }
            .resolve(&catalog)
            .is_err()
        );
    }

    #[test]
    fn input_value_evidence_is_schema_qualified_and_validated() {
        let payload = TypedInputPayload::new(
            schema("test.input.payload"),
            StructuredValue::Text("on".into()),
        )
        .unwrap();
        let evidence = InputValueEvidence::new(
            InputId::new("chip8.key.0").unwrap(),
            schema("test.input.payload"),
            InputSource::Scheduled { ordinal: 4 },
            InputCoordinate::frame(2),
            payload,
        );
        evidence.validate().unwrap();
        assert!(!evidence.payload.canonical_cbor_bytes().unwrap().is_empty());
    }
}
