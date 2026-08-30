//! Bounded, bundle-owned capability normalization.
//!
//! The normalizer observes borrowed emissions and therefore cannot retain an
//! incoming emission through this interface. Implementations own only their
//! capability-specific reducer state and publish outputs during finalization.

use glassvm_normalizer_contract::{
    CapabilityOutput, CapabilityReceipt, CapabilityRequest, CapabilityStatus,
};

use crate::{
    Emission, EmissionSink, EvidenceReceipt, EvidenceStatus, PreparedObservation, SinkError,
    SnapshotArtifact,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizerError {
    message: String,
}

impl NormalizerError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for NormalizerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(formatter)
    }
}

impl std::error::Error for NormalizerError {}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizerOutput {
    pub capabilities: Vec<CapabilityOutput>,
    pub receipts: Vec<CapabilityReceipt>,
}

impl NormalizerOutput {
    pub fn failed(requests: &[CapabilityRequest], error: &NormalizerError) -> Self {
        Self {
            capabilities: Vec::new(),
            receipts: requests
                .iter()
                .map(|request| CapabilityReceipt {
                    id: request.id.clone(),
                    status: CapabilityStatus::Failed,
                    output_schema: None,
                    logical_bytes: 0,
                    message: Some(error.message().to_owned()),
                })
                .collect(),
        }
    }

    /// Add capability fulfillment to the caller's evidence receipt without
    /// changing the machine execution result.
    pub fn apply_to_receipt(&self, receipt: &mut EvidenceReceipt) {
        receipt.capabilities = self.receipts.clone();
        if self
            .receipts
            .iter()
            .any(|capability| capability.status == CapabilityStatus::Failed)
        {
            receipt.status = EvidenceStatus::Failed;
            receipt.fulfilled_guarantee = None;
        } else if self
            .receipts
            .iter()
            .any(|capability| capability.status == CapabilityStatus::Incomplete)
        {
            receipt.status = EvidenceStatus::Incomplete;
            receipt.fulfilled_guarantee = None;
        }
    }
}

/// Bundle-facing lifecycle for bounded capability reducers.
///
/// `observe` receives a borrowed emission. An implementation may update
/// bounded reducer state, but this API does not give it ownership of the
/// incoming event, native envelope, frame, or snapshot.
pub trait Normalizer: Send {
    fn begin(&mut self, requests: &[CapabilityRequest]) -> Result<(), NormalizerError>;

    fn observe(&mut self, emission: &Emission<'_>) -> Result<(), NormalizerError>;

    /// Finalize from bounded reducer state and, at most, one validated final
    /// machine snapshot. This is deliberately not a completed evidence view:
    /// no collector, chunk list, replay package, or retained emission bag is
    /// passed through this boundary.
    fn finalize(
        &mut self,
        final_state: Option<&SnapshotArtifact>,
    ) -> Result<NormalizerOutput, NormalizerError>;
}

/// Emission sink that observes a normalizer and forwards every machine
/// emission to the caller-owned downstream sink.
///
/// A normalizer error is recorded and observation stops, but the emission is
/// still forwarded and the sink remains successful. This keeps normalizer
/// failure separate from machine/sink execution failure.
#[derive(Debug)]
pub struct NormalizerDriver<N, S> {
    normalizer: N,
    downstream: S,
    requests: Vec<CapabilityRequest>,
    preparation_receipts: Vec<CapabilityReceipt>,
    failure: Option<NormalizerError>,
}

impl<N, S> NormalizerDriver<N, S>
where
    N: Normalizer,
{
    pub fn new(mut normalizer: N, downstream: S, requests: Vec<CapabilityRequest>) -> Self {
        let failure = normalizer.begin(&requests).err();
        Self {
            normalizer,
            downstream,
            requests,
            preparation_receipts: Vec::new(),
            failure,
        }
    }

    /// Construct a driver from the prospective preparation result, carrying
    /// optional root capabilities downgraded during preparation into the
    /// retrospective receipt without passing them to the normalizer.
    pub fn new_with_prepared_observation(
        normalizer: N,
        downstream: S,
        prepared: &PreparedObservation,
    ) -> Self {
        let mut driver = Self::new(normalizer, downstream, prepared.normalizer_requests());
        driver.preparation_receipts = prepared.unavailable_capability_receipts();
        driver
    }

    pub fn normalizer_failure(&self) -> Option<&NormalizerError> {
        self.failure.as_ref()
    }

    pub fn finish(
        mut self,
        final_state: Option<&SnapshotArtifact>,
    ) -> Result<NormalizerRun<S>, SinkError>
    where
        S: EmissionSink,
    {
        let mut output = if let Some(error) = &self.failure {
            NormalizerOutput::failed(&self.requests, error)
        } else {
            match self.normalizer.finalize(final_state) {
                Ok(output) => output,
                Err(error) => {
                    self.failure = Some(error.clone());
                    NormalizerOutput::failed(&self.requests, &error)
                }
            }
        };
        if !self.preparation_receipts.is_empty() {
            let mut receipts = std::mem::take(&mut self.preparation_receipts);
            receipts.extend(output.receipts);
            output.receipts = receipts;
        }

        self.downstream
            .record_capability_outputs(&output.capabilities);
        self.downstream.record_capability_receipts(&output.receipts);

        Ok(NormalizerRun {
            downstream: self.downstream,
            output,
            failure: self.failure,
        })
    }
}

impl<N, S> EmissionSink for NormalizerDriver<N, S>
where
    N: Normalizer,
    S: EmissionSink,
{
    fn emit(&mut self, emission: Emission<'_>) -> Result<(), SinkError> {
        if self.failure.is_none()
            && let Err(error) = self.normalizer.observe(&emission)
        {
            self.failure = Some(error);
        }
        self.downstream.emit(emission)
    }
}

#[derive(Debug)]
pub struct NormalizerRun<S> {
    downstream: S,
    pub output: NormalizerOutput,
    pub failure: Option<NormalizerError>,
}

impl<S> NormalizerRun<S> {
    pub fn into_parts(self) -> (S, NormalizerOutput) {
        (self.downstream, self.output)
    }

    pub fn into_inner(self) -> S {
        self.downstream
    }

    pub fn apply_to_receipt(&self, receipt: &mut EvidenceReceipt) {
        self.output.apply_to_receipt(receipt);
    }
}

#[cfg(test)]
mod tests {
    use glassvm_normalizer_contract::{
        CapabilityDependency, CapabilityDescriptor, CapabilityId, CapabilitySchema, CostClass,
        NormalizerCatalog,
    };
    use serde_json::json;

    use super::*;
    use crate::{
        EVENT_SCHEMA_VERSION, EventContext, EventKind, ExecutionEvent, MachineId,
        ObservationRequest, RunId, VersionStamp, canonical_json_bytes,
    };

    #[derive(Debug, Default)]
    struct CountingSink {
        emissions: usize,
        canonical_capabilities: usize,
    }

    impl EmissionSink for CountingSink {
        fn emit(&mut self, _emission: Emission<'_>) -> Result<(), SinkError> {
            self.emissions += 1;
            Ok(())
        }

        fn record_capability_outputs(&mut self, outputs: &[CapabilityOutput]) {
            self.canonical_capabilities += outputs.len();
        }
    }

    #[derive(Debug, Default)]
    struct CountingNormalizer {
        observed_events: u64,
        observed_native: u64,
        requested: Vec<CapabilityRequest>,
    }

    impl Normalizer for CountingNormalizer {
        fn begin(&mut self, requests: &[CapabilityRequest]) -> Result<(), NormalizerError> {
            self.requested = requests.to_vec();
            Ok(())
        }

        fn observe(&mut self, emission: &Emission<'_>) -> Result<(), NormalizerError> {
            match emission {
                Emission::Event(_) => self.observed_events += 1,
                Emission::NativeEvidence(_) => self.observed_native += 1,
                _ => {}
            }
            Ok(())
        }

        fn finalize(
            &mut self,
            final_state: Option<&SnapshotArtifact>,
        ) -> Result<NormalizerOutput, NormalizerError> {
            assert!(final_state.is_none());
            let id = CapabilityId::new("test.observed_counts.v1").unwrap();
            if !self.requested.iter().any(|request| request.id == id) {
                return Ok(NormalizerOutput {
                    capabilities: Vec::new(),
                    receipts: self
                        .requested
                        .iter()
                        .map(|request| CapabilityReceipt {
                            id: request.id.clone(),
                            status: if request.required {
                                CapabilityStatus::Failed
                            } else {
                                CapabilityStatus::Unavailable
                            },
                            output_schema: None,
                            logical_bytes: 0,
                            message: Some(
                                "test normalizer did not produce the requested capability".into(),
                            ),
                        })
                        .collect(),
                });
            }
            let value = json!({
                "events": self.observed_events,
                "native": self.observed_native,
            });
            let logical_bytes = canonical_json_bytes(&value).unwrap().len() as u64;
            Ok(NormalizerOutput {
                capabilities: vec![CapabilityOutput {
                    schema: CapabilitySchema {
                        id: id.clone(),
                        version: crate::CAPABILITY_SCHEMA_VERSION,
                    },
                    value,
                }],
                receipts: vec![CapabilityReceipt {
                    id,
                    status: CapabilityStatus::Fulfilled,
                    output_schema: Some(CapabilitySchema {
                        id: CapabilityId::new("test.observed_counts.v1").unwrap(),
                        version: crate::CAPABILITY_SCHEMA_VERSION,
                    }),
                    logical_bytes,
                    message: None,
                }],
            })
        }
    }

    #[derive(Debug, Default)]
    struct FailingNormalizer;

    impl Normalizer for FailingNormalizer {
        fn begin(&mut self, _requests: &[CapabilityRequest]) -> Result<(), NormalizerError> {
            Ok(())
        }

        fn observe(&mut self, _emission: &Emission<'_>) -> Result<(), NormalizerError> {
            Err(NormalizerError::new("normalizer observation failed"))
        }

        fn finalize(
            &mut self,
            _final_state: Option<&SnapshotArtifact>,
        ) -> Result<NormalizerOutput, NormalizerError> {
            unreachable!("driver must not finalize a failed normalizer")
        }
    }

    #[derive(Debug, Default)]
    struct FinalStateNormalizer;

    impl Normalizer for FinalStateNormalizer {
        fn begin(&mut self, _requests: &[CapabilityRequest]) -> Result<(), NormalizerError> {
            Ok(())
        }

        fn observe(&mut self, _emission: &Emission<'_>) -> Result<(), NormalizerError> {
            Ok(())
        }

        fn finalize(
            &mut self,
            final_state: Option<&SnapshotArtifact>,
        ) -> Result<NormalizerOutput, NormalizerError> {
            let snapshot = final_state
                .ok_or_else(|| NormalizerError::new("final snapshot was not provided"))?;
            let id = CapabilityId::new("test.final_state.v1").unwrap();
            let schema = id.schema();
            let value = json!({
                "final_state_schema": snapshot.state_schema,
                "final_state_bytes": snapshot.byte_len,
            });
            let logical_bytes = canonical_json_bytes(&value)
                .map_err(NormalizerError::new)?
                .len() as u64;
            Ok(NormalizerOutput {
                capabilities: vec![CapabilityOutput {
                    schema: schema.clone(),
                    value,
                }],
                receipts: vec![CapabilityReceipt {
                    id,
                    status: CapabilityStatus::Fulfilled,
                    output_schema: Some(schema),
                    logical_bytes,
                    message: None,
                }],
            })
        }
    }

    fn event() -> ExecutionEvent {
        let context = EventContext {
            arch: MachineId::from("test"),
            machine_version: VersionStamp::from("test-v1"),
            run_id: RunId::from("normalizer-test"),
            sequence: 0,
            step: 0,
            cycle_or_tick: None,
            frame: Some(0),
            pc: None,
            instruction: None,
        };
        let mut event = ExecutionEvent::from_context(&context, EventKind::RunStarted);
        event.schema_version = EVENT_SCHEMA_VERSION;
        event
    }

    #[test]
    fn normalizer_observes_borrowed_emissions_and_emits_at_finalize() {
        let capability =
            CapabilityRequest::required(CapabilityId::new("test.observed_counts.v1").unwrap());
        let mut driver = NormalizerDriver::new(
            CountingNormalizer::default(),
            CountingSink::default(),
            vec![capability],
        );
        let event = event();
        driver.emit(Emission::Event(&event)).unwrap();
        let run = driver.finish(None).unwrap();
        assert_eq!(run.output.capabilities.len(), 1);
        assert_eq!(run.output.receipts[0].status, CapabilityStatus::Fulfilled);
        let sink = run.into_inner();
        assert_eq!(sink.emissions, 1);
        assert_eq!(sink.canonical_capabilities, 1);
    }

    #[test]
    fn normalizer_capability_receipts_reach_the_budgeted_evidence_receipt() {
        let capability =
            CapabilityRequest::required(CapabilityId::new("test.observed_counts.v1").unwrap());
        let downstream = crate::BudgetedSink::new(
            crate::NullSink,
            ObservationRequest::summary(),
            RunId::from("normalizer-test"),
            MachineId::from("test"),
            VersionStamp::from("test-v1"),
        )
        .unwrap();
        let mut driver =
            NormalizerDriver::new(CountingNormalizer::default(), downstream, vec![capability]);
        let event = event();
        driver.emit(Emission::Event(&event)).unwrap();
        let run = driver.finish(None).unwrap();
        let receipt = run.into_inner().receipt();
        assert_eq!(receipt.capabilities.len(), 1);
        assert_eq!(receipt.capabilities[0].status, CapabilityStatus::Fulfilled);
        assert_eq!(
            receipt.capabilities[0].id.as_str(),
            "test.observed_counts.v1"
        );
    }

    #[test]
    fn normalizer_failure_is_reflected_in_the_evidence_receipt() {
        let mut driver = NormalizerDriver::new(
            FailingNormalizer,
            crate::BudgetedSink::new(
                crate::NullSink,
                ObservationRequest::summary(),
                RunId::from("normalizer-test"),
                MachineId::from("test"),
                VersionStamp::from("test-v1"),
            )
            .unwrap(),
            vec![CapabilityRequest::required(
                CapabilityId::new("test.failed.v1").unwrap(),
            )],
        );
        let event = event();
        driver.emit(Emission::Event(&event)).unwrap();
        assert!(driver.normalizer_failure().is_some());
        let run = driver.finish(None).unwrap();
        assert_eq!(run.output.receipts[0].status, CapabilityStatus::Failed);
        let receipt = run.into_inner().receipt();
        assert_eq!(receipt.status, EvidenceStatus::Failed);
        assert!(receipt.fulfilled_guarantee.is_none());
        assert_eq!(receipt.capabilities[0].status, CapabilityStatus::Failed);
    }

    #[test]
    fn normalizer_receives_only_the_explicit_final_snapshot() {
        let capability =
            CapabilityRequest::required(CapabilityId::new("test.final_state.v1").unwrap());
        let driver = NormalizerDriver::new(
            FinalStateNormalizer,
            CountingSink::default(),
            vec![capability],
        );
        let snapshot = SnapshotArtifact::new(
            RunId::from("normalizer-test"),
            MachineId::from("test"),
            VersionStamp::from("test-v1"),
            4,
            8,
            vec![1, 2, 3, 4],
        );
        let run = driver.finish(Some(&snapshot)).unwrap();
        assert_eq!(run.output.receipts[0].status, CapabilityStatus::Fulfilled);
        assert_eq!(
            run.output.capabilities[0].value["final_state_bytes"],
            json!(4)
        );
        assert_eq!(run.into_inner().emissions, 0);
    }

    #[test]
    fn prepared_optional_downgrade_reaches_the_evidence_receipt() {
        let id = CapabilityId::new("test.optional.v1").unwrap();
        let catalog = NormalizerCatalog {
            schema_version: crate::NORMALIZER_CONTRACT_SCHEMA_VERSION,
            normalizer_id: "test.normalizer".into(),
            normalizer_version: "test.v1".into(),
            capabilities: vec![CapabilityDescriptor {
                schema: id.schema(),
                output_type: "json.object".into(),
                dependencies: vec![CapabilityDependency::NormalizedEvents],
                cost_class: CostClass::Bounded,
            }],
        };
        let mut request = ObservationRequest::summary();
        request.capabilities = vec![CapabilityRequest::optional(id.clone())];
        let prepared = PreparedObservation::prepare(&catalog, &request).unwrap();
        let downstream = crate::BudgetedSink::new(
            crate::NullSink,
            request,
            RunId::from("normalizer-test"),
            MachineId::from("test"),
            VersionStamp::from("test-v1"),
        )
        .unwrap();
        let driver = NormalizerDriver::new_with_prepared_observation(
            CountingNormalizer::default(),
            downstream,
            &prepared,
        );
        let run = driver.finish(None).unwrap();
        let receipt = run.into_inner().receipt();
        assert_eq!(receipt.capabilities.len(), 1);
        assert_eq!(receipt.capabilities[0].id, id);
        assert_eq!(
            receipt.capabilities[0].status,
            CapabilityStatus::Unavailable
        );
        assert_ne!(receipt.status, EvidenceStatus::Failed);
    }

    #[test]
    fn normalizer_marks_missing_optional_capabilities_without_failing_evidence() {
        let driver = NormalizerDriver::new(
            CountingNormalizer::default(),
            CountingSink::default(),
            vec![CapabilityRequest::optional(
                CapabilityId::new("test.optional.v1").unwrap(),
            )],
        );
        let run = driver.finish(None).unwrap();
        assert_eq!(run.output.receipts[0].status, CapabilityStatus::Unavailable);
        let mut receipt = crate::BudgetedSink::new(
            crate::NullSink,
            ObservationRequest::summary(),
            RunId::from("normalizer-test"),
            MachineId::from("test"),
            VersionStamp::from("test-v1"),
        )
        .unwrap()
        .receipt();
        run.apply_to_receipt(&mut receipt);
        assert_ne!(receipt.status, EvidenceStatus::Failed);
    }
}
