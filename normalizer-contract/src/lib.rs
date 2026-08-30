//! Machine-independent catalog and value types for bundle-owned normalizers.
//!
//! This crate deliberately does not depend on GlassVM execution types. A
//! bundle implements the normalizer behavior, GlassVM transports requests and
//! receipts, and downstream consumers apply their own policy to the returned
//! values.

use std::collections::BTreeSet;
use std::fmt;

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

pub const NORMALIZER_CONTRACT_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(2, 1, 0);
pub const CAPABILITY_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

fn validate_canonical_segmented_id(kind: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{kind} must not be empty"));
    }
    for segment in value.split('.') {
        if segment.is_empty() {
            return Err(format!("{kind} {value:?} contains an empty segment"));
        }
        if !segment.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        }) {
            return Err(format!(
                "{kind} {value:?} contains a character outside lowercase ASCII letters, digits, '_' and '-'"
            ));
        }
    }
    Ok(())
}

macro_rules! canonical_segmented_id {
    ($(#[$meta:meta])* $name:ident, $label:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, String> {
                let value = value.into();
                validate_canonical_segmented_id($label, &value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl TryFrom<String> for $name {
            type Error = String;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = String;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(DeError::custom)
            }
        }
    };
}

canonical_segmented_id!(
    /// Canonical bundle-scoped semantic identity for one runtime input.
    InputId,
    "input ID"
);

canonical_segmented_id!(
    /// Canonical unversioned semantic namespace grouping related input schemas.
    SchemaFamilyId,
    "schema family ID"
);

/// Machine-independent capability IDs whose schemas are owned by this
/// contract. A bundle may advertise one only when its normalizer implements
/// the corresponding reducer and conformance tests. Representation versions
/// belong to `CapabilitySchema.version`; they are not encoded in the ID.
pub mod standard_capabilities {
    /// Bounded counts and call-depth summary derived from normalized control-
    /// flow events. This is a profile, not a retained control-flow graph.
    pub const CONTROL_FLOW_MOTIFS: &str = "glassvm.control_flow.motifs";

    /// Bounded state read/write activity profile derived from normalized
    /// state observations.
    pub const MEMORY_STATE_MOTIFS: &str = "glassvm.memory.state_motifs";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl SchemaVersion {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapabilityId(pub String);

impl CapabilityId {
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        validate_canonical_segmented_id("capability ID", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilitySchema {
    pub id: CapabilityId,
    pub version: SchemaVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum InputValueSelector {
    InputId(InputId),
    SchemaFamily(SchemaFamilyId),
}

impl InputValueSelector {
    fn sort_key(&self) -> (u8, &str) {
        match self {
            Self::InputId(id) => (0, id.as_str()),
            Self::SchemaFamily(family) => (1, family.as_str()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizedAccessRequirement {
    AfterOnly,
    BeforeAndAfter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameCaptureRequirement {
    Hashes,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotRequirement {
    Final,
    EverySteps,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityDependency {
    NormalizedEvents,
    NormalizedEventKinds {
        kinds: Vec<String>,
    },
    NormalizedState {
        reads: bool,
        writes: bool,
        state_diffs: bool,
        access: NormalizedAccessRequirement,
    },
    NativeEvidence {
        schema_id: String,
    },
    NativeEventKinds {
        kinds: Vec<String>,
    },
    Frames {
        capture: FrameCaptureRequirement,
    },
    Snapshots {
        capture: SnapshotRequirement,
    },
    FinalState,
    InputValueEvidence {
        selectors: Vec<InputValueSelector>,
    },
    Capability(CapabilityId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostClass {
    Negligible,
    Bounded,
    Linear,
    Heavy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDescriptor {
    pub schema: CapabilitySchema,
    pub output_type: String,
    pub dependencies: Vec<CapabilityDependency>,
    pub cost_class: CostClass,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityRequest {
    pub id: CapabilityId,
    pub required: bool,
    pub parameters: Value,
}

impl CapabilityRequest {
    pub fn required(id: CapabilityId) -> Self {
        Self {
            id,
            required: true,
            parameters: Value::Object(Default::default()),
        }
    }

    pub fn optional(id: CapabilityId) -> Self {
        Self {
            id,
            required: false,
            parameters: Value::Object(Default::default()),
        }
    }
}

impl CapabilityId {
    pub fn schema(&self) -> CapabilitySchema {
        CapabilitySchema {
            id: self.clone(),
            version: CAPABILITY_SCHEMA_VERSION,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityOutput {
    pub schema: CapabilitySchema,
    pub value: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Fulfilled,
    Unavailable,
    Failed,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityReceipt {
    pub id: CapabilityId,
    pub status: CapabilityStatus,
    pub output_schema: Option<CapabilitySchema>,
    pub logical_bytes: u64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizerCatalog {
    pub schema_version: SchemaVersion,
    pub normalizer_id: String,
    pub normalizer_version: String,
    pub capabilities: Vec<CapabilityDescriptor>,
}

impl NormalizerCatalog {
    pub fn empty(normalizer_id: impl Into<String>, normalizer_version: impl Into<String>) -> Self {
        Self {
            schema_version: NORMALIZER_CONTRACT_SCHEMA_VERSION,
            normalizer_id: normalizer_id.into(),
            normalizer_version: normalizer_version.into(),
            capabilities: Vec::new(),
        }
    }

    pub fn find(&self, id: &CapabilityId) -> Option<&CapabilityDescriptor> {
        self.capabilities
            .iter()
            .find(|capability| capability.schema.id == *id)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != NORMALIZER_CONTRACT_SCHEMA_VERSION {
            return Err(format!(
                "unsupported normalizer catalog schema {:?}; expected {:?}",
                self.schema_version, NORMALIZER_CONTRACT_SCHEMA_VERSION
            ));
        }
        if self.normalizer_id.trim().is_empty() {
            return Err("normalizer catalog has an empty normalizer ID".into());
        }
        if self.normalizer_version.trim().is_empty() {
            return Err("normalizer catalog has an empty normalizer version".into());
        }

        let mut seen = std::collections::BTreeSet::new();
        for capability in &self.capabilities {
            CapabilityId::new(capability.schema.id.as_str().to_owned())?;
            if !seen.insert(capability.schema.id.clone()) {
                return Err(format!(
                    "normalizer catalog declares capability {} more than once",
                    capability.schema.id.as_str()
                ));
            }
            if capability.output_type.trim().is_empty() {
                return Err(format!(
                    "normalizer capability {} has an empty output type",
                    capability.schema.id.as_str()
                ));
            }
            for dependency in &capability.dependencies {
                match dependency {
                    CapabilityDependency::NormalizedEventKinds { kinds }
                    | CapabilityDependency::NativeEventKinds { kinds } => {
                        if kinds.is_empty() || kinds.iter().any(|kind| kind.trim().is_empty()) {
                            return Err(format!(
                                "normalizer capability {} has an empty event-kind prerequisite",
                                capability.schema.id.as_str()
                            ));
                        }
                        let mut event_kinds = std::collections::BTreeSet::new();
                        if kinds.iter().any(|kind| !event_kinds.insert(kind)) {
                            return Err(format!(
                                "normalizer capability {} repeats an event-kind prerequisite",
                                capability.schema.id.as_str()
                            ));
                        }
                    }
                    CapabilityDependency::NativeEvidence { schema_id }
                        if schema_id.trim().is_empty() =>
                    {
                        return Err(format!(
                            "normalizer capability {} has an empty native-evidence schema",
                            capability.schema.id.as_str()
                        ));
                    }
                    CapabilityDependency::Capability(id) => {
                        CapabilityId::new(id.as_str().to_owned())?;
                    }
                    CapabilityDependency::InputValueEvidence { selectors } => {
                        if selectors.is_empty() {
                            return Err(format!(
                                "normalizer capability {} has an empty input-value prerequisite",
                                capability.schema.id.as_str()
                            ));
                        }
                        let mut seen = BTreeSet::new();
                        if selectors
                            .iter()
                            .any(|selector| !seen.insert(selector.clone()))
                        {
                            return Err(format!(
                                "normalizer capability {} repeats an input-value selector",
                                capability.schema.id.as_str()
                            ));
                        }
                        if selectors
                            .windows(2)
                            .any(|window| window[0].sort_key() > window[1].sort_key())
                        {
                            return Err(format!(
                                "normalizer capability {} has non-canonical input-value selectors",
                                capability.schema.id.as_str()
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_ids_use_canonical_segmented_names() {
        assert!(CapabilityId::new("glassvm.visual.interestingness").is_ok());
        assert!(CapabilityId::new("glassvm.visual").is_ok());
        assert!(CapabilityId::new("hexwell.reaction_dynamics").is_ok());
        assert!(CapabilityId::new("glassvm.Visual.interestingness").is_err());
        assert!(CapabilityId::new("glassvm.visual..interestingness").is_err());
    }

    #[test]
    fn requests_and_catalogs_round_trip() {
        let id = CapabilityId::new("glassvm.control_flow.motifs").unwrap();
        let request = CapabilityRequest::required(id.clone());
        let encoded = serde_json::to_vec(&request).unwrap();
        let decoded: CapabilityRequest = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, request);

        let catalog = NormalizerCatalog {
            schema_version: NORMALIZER_CONTRACT_SCHEMA_VERSION,
            normalizer_id: "test.normalizer".into(),
            normalizer_version: "test.v1".into(),
            capabilities: vec![CapabilityDescriptor {
                schema: CapabilitySchema {
                    id: id.clone(),
                    version: SchemaVersion::new(1, 0, 0),
                },
                output_type: "object".into(),
                dependencies: vec![CapabilityDependency::NormalizedEvents],
                cost_class: CostClass::Bounded,
            }],
        };
        let encoded = serde_json::to_vec(&catalog).unwrap();
        let decoded: NormalizerCatalog = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, catalog);
        assert!(decoded.validate().is_ok());
        assert!(decoded.find(&id).is_some());
    }

    #[test]
    fn catalogs_reject_duplicate_or_malformed_capabilities() {
        let id = CapabilityId::new("glassvm.visual.interestingness.v1").unwrap();
        let descriptor = CapabilityDescriptor {
            schema: id.schema(),
            output_type: "object".into(),
            dependencies: vec![CapabilityDependency::NativeEvidence {
                schema_id: "glassvm.observation.v1".into(),
            }],
            cost_class: CostClass::Bounded,
        };
        let mut catalog = NormalizerCatalog::empty("test.normalizer", "test.v1");
        catalog.capabilities = vec![descriptor.clone(), descriptor];
        assert!(catalog.validate().is_err());
    }

    #[test]
    fn input_value_selectors_require_nonempty_canonical_unique_lists() {
        let input = InputId::new("chip8.key.0").unwrap();
        let family = SchemaFamilyId::new("glassvm.input.digital-key").unwrap();
        let capability = CapabilityId::new("test.input_values.v1").unwrap();
        let descriptor = CapabilityDescriptor {
            schema: capability.schema(),
            output_type: "json.object".into(),
            dependencies: vec![CapabilityDependency::InputValueEvidence {
                selectors: vec![
                    InputValueSelector::InputId(input),
                    InputValueSelector::SchemaFamily(family),
                ],
            }],
            cost_class: CostClass::Bounded,
        };
        let catalog = NormalizerCatalog {
            schema_version: NORMALIZER_CONTRACT_SCHEMA_VERSION,
            normalizer_id: "test.normalizer".into(),
            normalizer_version: "test.v1".into(),
            capabilities: vec![descriptor],
        };
        catalog.validate().unwrap();
    }

    #[test]
    fn input_value_selectors_reject_duplicates_and_noncanonical_order() {
        let a = InputValueSelector::InputId(InputId::new("chip8.key.0").unwrap());
        let b = InputValueSelector::InputId(InputId::new("chip8.key.1").unwrap());
        let capability = CapabilityId::new("test.input_values.v1").unwrap();
        let make_catalog = |selectors| NormalizerCatalog {
            schema_version: NORMALIZER_CONTRACT_SCHEMA_VERSION,
            normalizer_id: "test.normalizer".into(),
            normalizer_version: "test.v1".into(),
            capabilities: vec![CapabilityDescriptor {
                schema: capability.schema(),
                output_type: "json.object".into(),
                dependencies: vec![CapabilityDependency::InputValueEvidence { selectors }],
                cost_class: CostClass::Bounded,
            }],
        };

        assert!(make_catalog(vec![a.clone(), a.clone()]).validate().is_err());
        assert!(make_catalog(vec![b, a]).validate().is_err());
    }
}
