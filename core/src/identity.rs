use std::fmt;

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub use glassvm_normalizer_contract::{InputId, SchemaFamilyId};

/// Stable architecture identifier used by registries, traces, and replay data.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MachineId(pub String);

impl MachineId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MachineId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&str> for MachineId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for MachineId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Consumer-supplied identity for one execution.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunId(pub String);

impl RunId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&str> for RunId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RunId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Opaque version string tied to a concrete bundle, emulator, or machine model.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VersionStamp(pub String);

impl VersionStamp {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VersionStamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&str> for VersionStamp {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Semantic version of a serialized GlassVM schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl SchemaVersion {
    pub const V1: Self = Self::new(1, 0, 0);

    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Names and versions one native or normalized data schema.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaRef {
    pub id: String,
    pub version: SchemaVersion,
}

impl SchemaRef {
    pub fn new(id: impl Into<String>, version: SchemaVersion) -> Self {
        Self {
            id: id.into(),
            version,
        }
    }
}

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
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.0)
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

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    };
}

canonical_segmented_id!(
    /// Canonical bundle-scoped identity for one caller-imposed execution limit.
    ExecutionLimitId,
    "execution limit ID"
);

/// Stable per-run identity assigned to an accepted live input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InputInstanceId(u64);

impl InputInstanceId {
    pub const FIRST: Self = Self(0);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }

    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

impl std::fmt::Display for InputInstanceId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_segmented_ids_accept_only_the_locked_grammar() {
        assert!(InputId::new("chip8.key.0").is_ok());
        assert!(SchemaFamilyId::new("glassvm.input.digital-key").is_ok());
        assert!(ExecutionLimitId::new("wasm.fuel").is_ok());

        for invalid in [
            "",
            ".chip8",
            "chip8.",
            "chip8..key",
            "Chip8.key",
            "chip8/key",
            "café",
        ] {
            assert!(
                InputId::new(invalid).is_err(),
                "accepted invalid ID {invalid:?}"
            );
        }
    }

    #[test]
    fn canonical_segmented_ids_reject_noncanonical_deserialization() {
        assert!(serde_json::from_str::<InputId>(r#""chip8.key.0""#).is_ok());
        assert!(serde_json::from_str::<InputId>(r#""CHIP8.KEY.0""#).is_err());
    }

    #[test]
    fn live_input_instances_start_at_zero_and_allocate_monotonically() {
        assert_eq!(InputInstanceId::FIRST.as_u64(), 0);
        assert_eq!(InputInstanceId::FIRST.next(), Some(InputInstanceId::new(1)));
        assert_eq!(InputInstanceId::new(u64::MAX).next(), None);
    }
}
