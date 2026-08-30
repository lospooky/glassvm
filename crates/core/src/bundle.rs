use crate::{
    EmulatorBackend, ExecutionCoordinateCatalog, ExecutionRequest, LiveInputCapability, MachineId,
    MachineIdentity, Normalizer, NormalizerCatalog, ObservationRequest, PreparedObservation,
    PreparedRun, SchemaRef, SchemaVersion, VerifierBackend, VersionStamp,
};
use glassvm_normalizer_contract::CapabilityRequest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineSemantics {
    pub reset: String,
    pub boot: String,
    pub timing: String,
    pub memory: String,
    pub instruction_schema: SchemaRef,
    pub state_schema: SchemaRef,
    pub native_event_schema: SchemaRef,
    pub execution_coordinates: ExecutionCoordinateCatalog,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineDescriptor {
    pub id: MachineId,
    pub display_name: String,
    pub bundle_version: VersionStamp,
    pub machine_version: VersionStamp,
    pub emulator_version: VersionStamp,
    pub variants: Vec<String>,
    pub semantics: MachineSemantics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactEncoding {
    RawBytes,
    Extension(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineArtifactSpec {
    pub schema: SchemaRef,
    pub encoding: ArtifactEncoding,
    pub min_bytes: usize,
    pub max_bytes: usize,
    pub alignment_bytes: usize,
    pub load_address: Option<u64>,
}

impl MachineArtifactSpec {
    pub fn validate_constraints(&self) -> Result<(), String> {
        if self.min_bytes > self.max_bytes {
            return Err("artifact minimum length exceeds maximum length".into());
        }
        if self.alignment_bytes == 0 {
            return Err("artifact alignment must be non-zero".into());
        }
        Ok(())
    }

    pub fn validate(&self, artifact: &[u8]) -> Result<(), String> {
        self.validate_constraints()?;
        if artifact.len() < self.min_bytes {
            return Err(format!(
                "artifact has {} bytes; minimum is {}",
                artifact.len(),
                self.min_bytes
            ));
        }
        if artifact.len() > self.max_bytes {
            return Err(format!(
                "artifact has {} bytes; maximum is {}",
                artifact.len(),
                self.max_bytes
            ));
        }
        if !artifact.len().is_multiple_of(self.alignment_bytes) {
            return Err(format!(
                "artifact length {} is not aligned to {} bytes",
                artifact.len(),
                self.alignment_bytes
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineContract {
    pub artifact: MachineArtifactSpec,
    pub inputs: crate::InputCatalog,
}

impl MachineContract {
    pub fn validate(&self, live_input_capability_present: bool) -> Result<(), String> {
        self.artifact.validate_constraints()?;
        self.inputs.validate()?;
        if self.inputs.inputs.iter().any(|input| {
            input
                .delivery_modes
                .contains(&crate::InputDeliveryMode::Live)
        }) && !live_input_capability_present
        {
            return Err(
                "machine contract declares live inputs but bundle has no live-input capability"
                    .into(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyzeResult {
    pub capabilities: Vec<glassvm_normalizer_contract::CapabilityOutput>,
}

pub trait StaticAnalyzerBackend: Send + Sync {
    fn machine_id(&self) -> MachineId;
    fn analyze_bytes(&self, artifact: &[u8]) -> Result<AnalyzeResult, String>;
}

/// Complete machine-specific package behind GlassVM's universal ontology.
pub trait MachineBundle: Send + Sync {
    fn descriptor(&self) -> &MachineDescriptor;
    fn contract(&self) -> &MachineContract;
    fn emulator(&self) -> &dyn EmulatorBackend;
    fn static_analyzer(&self) -> Option<&dyn StaticAnalyzerBackend>;
    fn verifier(&self) -> Option<&dyn VerifierBackend>;
    /// Optional synchronous live-input construction for a prepared run.
    fn live_input_capability(&self) -> Option<&dyn LiveInputCapability> {
        None
    }

    /// Prepare the execution-side contract once, before session construction.
    fn prepare_run(&self, request: &ExecutionRequest) -> Result<PreparedRun, String> {
        request.validate_envelope()?;
        if request.machine_id != self.descriptor().id {
            return Err(format!(
                "execution request targets {}; bundle provides {}",
                request.machine_id,
                self.descriptor().id
            ));
        }

        let configuration_schema = self.emulator().config_schema();

        let prepared = PreparedRun::prepare(
            request.run_id.clone(),
            &self.contract().artifact,
            &request.artifact,
            MachineIdentity {
                machine_id: self.descriptor().id.clone(),
                bundle_version: self.descriptor().bundle_version.clone(),
                machine_version: self.descriptor().machine_version.clone(),
                emulator_version: self.descriptor().emulator_version.clone(),
                contract_schema: SchemaRef::new("glassvm.machine_contract", SchemaVersion::V1),
            },
            &request.configuration,
            &configuration_schema,
            &self.contract().inputs,
            &request.input_schedule,
            &request.execution_controls,
            &self.emulator().execution_limit_catalog(),
        )?;
        crate::create_live_input_controller(self.live_input_capability(), &prepared)?;
        Ok(prepared)
    }
    /// Describe the bundle-owned, machine-independent capability outputs.
    fn normalizer_catalog(&self) -> NormalizerCatalog;

    /// Resolve the canonical observation request against this bundle's
    /// normalizer catalog before execution. The returned value is a
    /// prospective negotiated contract; it is distinct from the
    /// retrospective `EvidenceReceipt` emitted after a run.
    fn prepare_observation(
        &self,
        request: &ObservationRequest,
    ) -> Result<PreparedObservation, Vec<String>> {
        PreparedObservation::prepare_with_input_catalog(
            &self.normalizer_catalog(),
            &self.contract().inputs,
            request,
        )
    }

    /// Construct a per-run normalizer for the requested capabilities.
    fn create_normalizer(
        &self,
        _requests: &[CapabilityRequest],
    ) -> Result<Option<Box<dyn Normalizer>>, String>;

    fn validate_contract(&self) -> Result<(), Vec<String>> {
        let descriptor = self.descriptor();
        let contract = self.contract();
        let mut errors = Vec::new();

        if descriptor.id != self.emulator().machine_id() {
            errors.push("emulator machine ID does not match bundle descriptor".into());
        }
        match self.static_analyzer() {
            Some(analyzer) if descriptor.id != analyzer.machine_id() => {
                errors.push("static-analyzer machine ID does not match bundle descriptor".into());
            }
            None => {}
            Some(_) => {}
        }
        match self.verifier() {
            Some(verifier) if descriptor.id != verifier.machine_id() => {
                errors.push("verifier machine ID does not match bundle descriptor".into());
            }
            None => {}
            Some(_) => {}
        }
        if let Err(error) = self.normalizer_catalog().validate() {
            errors.push(format!("normalizer catalog is invalid: {error}"));
        }
        if let Err(error) = contract.validate(self.live_input_capability().is_some()) {
            errors.push(format!("machine contract is invalid: {error}"));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::SchemaVersion;

    #[test]
    fn slim_machine_contract_validates_artifact_inputs_and_live_service_consistency() {
        let artifact = MachineArtifactSpec {
            schema: SchemaRef::new("test.artifact", SchemaVersion::V1),
            encoding: ArtifactEncoding::RawBytes,
            min_bytes: 1,
            max_bytes: 8,
            alignment_bytes: 1,
            load_address: None,
        };
        let scheduled = MachineContract {
            artifact: artifact.clone(),
            inputs: crate::InputCatalog::empty(),
        };
        assert!(scheduled.validate(false).is_ok());

        let live = MachineContract {
            artifact,
            inputs: crate::InputCatalog {
                schema: crate::input_catalog_schema(),
                inputs: vec![crate::InputSpec {
                    id: crate::InputId::new("test.live").unwrap(),
                    payload_schema: SchemaRef::new("test.input", SchemaVersion::V1),
                    schema_family: None,
                    coordinate_policy: crate::CoordinatePolicy::frame_start(),
                    delivery_modes: BTreeSet::from([crate::InputDeliveryMode::Live]),
                    application: crate::InputApplicationSemantics::new(SchemaRef::new(
                        "test.application",
                        SchemaVersion::V1,
                    ))
                    .unwrap(),
                }],
            },
        };
        assert!(live.validate(false).is_err());
        assert!(live.validate(true).is_ok());
    }
}
