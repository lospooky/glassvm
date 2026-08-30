use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ConfigField, ConfigFieldKind, MachineConfigSchema, SchemaRef, SchemaVersion, StructuredValue,
    canonical_cbor_bytes,
};

pub const EXECUTION_LIMIT_CATALOG_SCHEMA_VERSION: SchemaVersion = SchemaVersion::V1;

pub fn execution_limit_catalog_schema() -> SchemaRef {
    SchemaRef::new(
        "glassvm.execution_limit_catalog",
        EXECUTION_LIMIT_CATALOG_SCHEMA_VERSION,
    )
}

impl MachineConfigSchema {
    pub fn validate(&self) -> Result<(), String> {
        validate_schema(&self.schema, "machine configuration schema")?;
        let mut seen = BTreeSet::new();
        for field in &self.fields {
            if field.key.trim().is_empty() {
                return Err("machine configuration has a field with an empty key".into());
            }
            if !seen.insert(field.key.clone()) {
                return Err(format!(
                    "machine configuration declares field {} more than once",
                    field.key
                ));
            }
            if let ConfigFieldKind::Int { min, max, step } = &field.kind
                && (min > max || *step <= 0)
            {
                return Err(format!(
                    "machine configuration field {} has invalid integer bounds",
                    field.key
                ));
            }
            if let ConfigFieldKind::Float { min, max, step } = &field.kind
                && (min > max || *step < 0.0 || !step.is_finite())
            {
                return Err(format!(
                    "machine configuration field {} has invalid float bounds",
                    field.key
                ));
            }
            let default = json_to_structured(&field.default_value)?;
            validate_field_value(field, &default)?;
        }
        Ok(())
    }
}

/// A schema-qualified structured value used for machine configuration and
/// caller-imposed bundle limits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedStructuredValue {
    pub schema: SchemaRef,
    pub value: StructuredValue,
}

impl TypedStructuredValue {
    pub fn validate(&self) -> Result<(), String> {
        validate_schema(&self.schema, "typed structured value")?;
        canonical_cbor_bytes(&self.value).map(|_| ())
    }
}

/// Concrete construction-time machine parameterization submitted by a caller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineConfiguration {
    pub schema: SchemaRef,
    pub value: StructuredValue,
}

impl MachineConfiguration {
    pub fn new(schema: SchemaRef, value: StructuredValue) -> Result<Self, String> {
        let configuration = Self { schema, value };
        configuration.validate_shape()?;
        Ok(configuration)
    }

    pub fn validate_shape(&self) -> Result<(), String> {
        validate_schema(&self.schema, "machine configuration")?;
        if !matches!(self.value, StructuredValue::Map(_)) {
            return Err("machine configuration value must be a string-keyed map".into());
        }
        canonical_cbor_bytes(&self.value).map(|_| ())
    }

    /// Materialize the declared defaults as an explicit configuration value.
    /// Preparation therefore never carries an omitted, ambiguous machine
    /// configuration into session construction.
    pub fn defaults(schema: &MachineConfigSchema) -> Result<Self, String> {
        schema.validate()?;
        let mut values = BTreeMap::new();
        for field in &schema.fields {
            values.insert(field.key.clone(), json_to_structured(&field.default_value)?);
        }
        Self::new(schema.schema.clone(), StructuredValue::Map(values))
    }

    /// Validate against the bundle declaration, resolve defaults, and return
    /// the immutable effective configuration used by a prepared session.
    pub fn prepare(&self, schema: &MachineConfigSchema) -> Result<PreparedConfiguration, String> {
        self.validate_shape()?;
        schema.validate()?;
        if self.schema != schema.schema {
            return Err(format!(
                "machine configuration schema {} {} does not match declared schema {} {}",
                self.schema.id, self.schema.version, schema.schema.id, schema.schema.version
            ));
        }

        let submitted = match &self.value {
            StructuredValue::Map(values) => values,
            _ => unreachable!("validate_shape checked configuration map shape"),
        };
        let declarations = schema
            .fields
            .iter()
            .map(|field| (field.key.as_str(), field))
            .collect::<BTreeMap<_, _>>();
        for key in submitted.keys() {
            if !declarations.contains_key(key.as_str()) {
                return Err(format!(
                    "machine configuration contains unknown field {key:?}"
                ));
            }
        }

        let mut effective = BTreeMap::new();
        for field in &schema.fields {
            let value = match submitted.get(&field.key) {
                Some(value) => value.clone(),
                None => json_to_structured(&field.default_value)?,
            };
            validate_field_value(field, &value)?;
            effective.insert(field.key.clone(), value);
        }
        let value = StructuredValue::Map(effective);
        let canonical_bytes = canonical_cbor_bytes(&value)?;
        Ok(PreparedConfiguration {
            schema: schema.schema.clone(),
            value,
            canonical_bytes,
        })
    }
}

/// Effective configuration after schema validation and default resolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedConfiguration {
    pub schema: SchemaRef,
    pub value: StructuredValue,
    #[serde(default)]
    pub canonical_bytes: Vec<u8>,
}

impl PreparedConfiguration {
    pub fn validate(&self) -> Result<(), String> {
        validate_schema(&self.schema, "prepared machine configuration")?;
        let expected = canonical_cbor_bytes(&self.value)?;
        if self.canonical_bytes != expected {
            return Err("prepared configuration canonical bytes do not match value".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionLimitDescriptor {
    pub limit_id: crate::ExecutionLimitId,
    pub value_schema: SchemaRef,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionLimitCatalog {
    pub schema: SchemaRef,
    pub limits: Vec<ExecutionLimitDescriptor>,
}

impl ExecutionLimitCatalog {
    pub fn empty() -> Self {
        Self {
            schema: execution_limit_catalog_schema(),
            limits: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != execution_limit_catalog_schema() {
            return Err(format!(
                "unsupported execution-limit catalog schema {} {}; expected {} {}",
                self.schema.id,
                self.schema.version,
                execution_limit_catalog_schema().id,
                execution_limit_catalog_schema().version
            ));
        }
        let mut seen = BTreeSet::new();
        for descriptor in &self.limits {
            validate_schema(&descriptor.value_schema, "execution-limit value")?;
            if descriptor.description.trim().is_empty() {
                return Err(format!(
                    "execution limit {} has an empty description",
                    descriptor.limit_id
                ));
            }
            if !seen.insert(descriptor.limit_id.clone()) {
                return Err(format!(
                    "execution-limit catalog declares {} more than once",
                    descriptor.limit_id
                ));
            }
        }
        Ok(())
    }

    pub fn find(&self, limit_id: &crate::ExecutionLimitId) -> Option<&ExecutionLimitDescriptor> {
        self.limits
            .iter()
            .find(|descriptor| &descriptor.limit_id == limit_id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleExecutionLimit {
    pub limit_id: crate::ExecutionLimitId,
    pub value: TypedStructuredValue,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionControls {
    pub frame_limit: Option<u64>,
    pub step_limit: Option<u64>,
    pub bundle_limits: Vec<BundleExecutionLimit>,
}

impl ExecutionControls {
    pub fn prepare(
        &self,
        catalog: &ExecutionLimitCatalog,
    ) -> Result<PreparedExecutionControls, String> {
        catalog.validate()?;
        let mut seen = BTreeSet::new();
        let mut bundle_limits = self.bundle_limits.clone();
        for limit in &bundle_limits {
            if !seen.insert(limit.limit_id.clone()) {
                return Err(format!(
                    "execution controls repeat bundle limit {}",
                    limit.limit_id
                ));
            }
            let descriptor = catalog.find(&limit.limit_id).ok_or_else(|| {
                format!(
                    "execution controls reference unsupported limit {}",
                    limit.limit_id
                )
            })?;
            if limit.value.schema != descriptor.value_schema {
                return Err(format!(
                    "execution limit {} uses schema {} {}; expected {} {}",
                    limit.limit_id,
                    limit.value.schema.id,
                    limit.value.schema.version,
                    descriptor.value_schema.id,
                    descriptor.value_schema.version
                ));
            }
            limit.value.validate()?;
        }
        bundle_limits.sort_by(|left, right| left.limit_id.cmp(&right.limit_id));
        Ok(PreparedExecutionControls {
            frame_limit: self.frame_limit,
            step_limit: self.step_limit,
            bundle_limits,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedExecutionControls {
    pub frame_limit: Option<u64>,
    pub step_limit: Option<u64>,
    pub bundle_limits: Vec<BundleExecutionLimit>,
}

impl PreparedExecutionControls {
    pub fn validate(&self) -> Result<(), String> {
        if self
            .bundle_limits
            .windows(2)
            .any(|window| window[0].limit_id >= window[1].limit_id)
        {
            return Err("prepared bundle limits must be sorted and unique".into());
        }
        Ok(())
    }
}

fn validate_field_value(field: &ConfigField, value: &StructuredValue) -> Result<(), String> {
    let valid = match &field.kind {
        ConfigFieldKind::Bool => matches!(value, StructuredValue::Bool(_)),
        ConfigFieldKind::Int { min, max, step } => {
            let number = match value {
                StructuredValue::Signed(value) => Some(*value),
                StructuredValue::Unsigned(value) => i64::try_from(*value).ok(),
                _ => None,
            };
            number.is_some_and(|number| {
                number >= *min
                    && number <= *max
                    && (*step == 0 || (number - *min).rem_euclid(*step) == 0)
            })
        }
        ConfigFieldKind::Float { min, max, step } => {
            let number = match value {
                StructuredValue::Float(value) => Some(*value),
                StructuredValue::Signed(value) => Some(*value as f64),
                StructuredValue::Unsigned(value) => Some(*value as f64),
                _ => None,
            };
            number.is_some_and(|number| {
                number.is_finite()
                    && number >= *min
                    && number <= *max
                    && (*step == 0.0 || ((number - *min) / *step).fract() == 0.0)
            })
        }
        ConfigFieldKind::Enum { options } => {
            matches!(value, StructuredValue::Text(value) if options.iter().any(|option| option == value))
        }
        ConfigFieldKind::String => matches!(value, StructuredValue::Text(_)),
    };
    if valid {
        Ok(())
    } else {
        Err(format!(
            "configuration field {:?} has an invalid value",
            field.key
        ))
    }
}

fn json_to_structured(value: &Value) -> Result<StructuredValue, String> {
    Ok(match value {
        Value::Null => StructuredValue::Null,
        Value::Bool(value) => StructuredValue::Bool(*value),
        Value::Number(value) => {
            if let Some(value) = value.as_u64() {
                StructuredValue::Unsigned(value)
            } else if let Some(value) = value.as_i64() {
                StructuredValue::Signed(value)
            } else if let Some(value) = value.as_f64() {
                if !value.is_finite() {
                    return Err("configuration defaults reject non-finite floats".into());
                }
                StructuredValue::Float(value)
            } else {
                return Err("configuration default has an unsupported number".into());
            }
        }
        Value::String(value) => StructuredValue::Text(value.clone()),
        Value::Array(values) => StructuredValue::Array(
            values
                .iter()
                .map(json_to_structured)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Value::Object(values) => StructuredValue::Map(
            values
                .iter()
                .map(|(key, value)| Ok((key.clone(), json_to_structured(value)?)))
                .collect::<Result<BTreeMap<_, _>, String>>()?,
        ),
    })
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
    use crate::SchemaVersion;

    fn schema(id: &str) -> SchemaRef {
        SchemaRef::new(id, SchemaVersion::V1)
    }

    fn config_schema() -> MachineConfigSchema {
        MachineConfigSchema {
            schema: schema("test.machine.configuration"),
            fields: vec![
                ConfigField {
                    key: "instructions_per_frame".into(),
                    label: "Instructions per frame".into(),
                    description: "machine work quantum".into(),
                    kind: ConfigFieldKind::Int {
                        min: 1,
                        max: 100,
                        step: 1,
                    },
                    default_value: Value::from(12),
                },
                ConfigField {
                    key: "machine_seed".into(),
                    label: "Machine seed".into(),
                    description: "state-evolution seed".into(),
                    kind: ConfigFieldKind::Int {
                        min: 0,
                        max: i64::MAX,
                        step: 1,
                    },
                    default_value: Value::from(0),
                },
            ],
        }
    }

    #[test]
    fn configuration_resolves_defaults_and_rejects_unknown_or_invalid_fields() {
        let schema = config_schema();
        let configuration = MachineConfiguration::new(
            schema.schema.clone(),
            StructuredValue::Map(BTreeMap::from([(
                "machine_seed".into(),
                StructuredValue::Unsigned(7),
            )])),
        )
        .unwrap();
        let prepared = configuration.prepare(&schema).unwrap();
        assert_eq!(
            prepared.value,
            StructuredValue::Map(BTreeMap::from([
                (
                    "instructions_per_frame".into(),
                    StructuredValue::Unsigned(12)
                ),
                ("machine_seed".into(), StructuredValue::Unsigned(7)),
            ]))
        );
        prepared.validate().unwrap();

        let unknown = MachineConfiguration::new(
            schema.schema.clone(),
            StructuredValue::Map(BTreeMap::from([(
                "unknown".into(),
                StructuredValue::Unsigned(1),
            )])),
        )
        .unwrap();
        assert!(unknown.prepare(&schema).is_err());
        let invalid = MachineConfiguration::new(
            schema.schema.clone(),
            StructuredValue::Map(BTreeMap::from([(
                "instructions_per_frame".into(),
                StructuredValue::Unsigned(0),
            )])),
        )
        .unwrap();
        assert!(invalid.prepare(&schema).is_err());
    }

    #[test]
    fn explicit_default_and_omission_produce_equal_prepared_configuration() {
        let schema = config_schema();
        let omitted =
            MachineConfiguration::new(schema.schema.clone(), StructuredValue::Map(BTreeMap::new()))
                .unwrap()
                .prepare(&schema)
                .unwrap();
        let explicit = MachineConfiguration::new(
            schema.schema.clone(),
            StructuredValue::Map(BTreeMap::from([
                (
                    "instructions_per_frame".into(),
                    StructuredValue::Unsigned(12),
                ),
                ("machine_seed".into(), StructuredValue::Unsigned(0)),
            ])),
        )
        .unwrap()
        .prepare(&schema)
        .unwrap();
        assert_eq!(omitted, explicit);
    }

    #[test]
    fn execution_limits_are_declared_validated_and_canonicalized() {
        let id = crate::ExecutionLimitId::new("test.work_budget").unwrap();
        let catalog = ExecutionLimitCatalog {
            schema: execution_limit_catalog_schema(),
            limits: vec![ExecutionLimitDescriptor {
                limit_id: id.clone(),
                value_schema: schema("test.work_limit"),
                description: "caller work bound".into(),
            }],
        };
        let controls = ExecutionControls {
            frame_limit: None,
            step_limit: Some(10),
            bundle_limits: vec![BundleExecutionLimit {
                limit_id: id.clone(),
                value: TypedStructuredValue {
                    schema: schema("test.work_limit"),
                    value: StructuredValue::Unsigned(100),
                },
            }],
        };
        let prepared = controls.prepare(&catalog).unwrap();
        prepared.validate().unwrap();
        assert_eq!(prepared.bundle_limits[0].limit_id, id);
        assert!(
            ExecutionControls::default()
                .prepare(&ExecutionLimitCatalog::empty())
                .is_ok()
        );
    }
}
