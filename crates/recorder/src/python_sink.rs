//! Python-facing sink contract types and bounded synchronous bridge.
//!
//! This module defines the owned records and outcome vocabulary shared by
//! Python bundle bindings. The borrowed core emission API remains unchanged;
//! the bridge batches borrowed core emissions into bounded owned records
//! before synchronously delivering them to Python consumers.

use std::mem;

use glassvm_core::{
    CapabilityOutput, CapabilityReceipt, CommonMetrics, ContentDigest, Emission, EmissionSink,
    EvidenceReceipt, ExecutionControls, ExecutionEvent, ExecutionRequest, FrameArtifact,
    InputSchedule, InputValueEvidence, MachineConfiguration, MachineId, NativeEvidenceEnvelope,
    ObservationRequest, RunId, RunResult, SchemaVersion, SinkError, SnapshotArtifact,
    canonical_json_bytes,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::types::PyModule;

/// Metadata visible at run start. Artifact bytes are intentionally not copied
/// into the consumer record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunStartedRecord {
    pub schema_version: SchemaVersion,
    pub run_id: RunId,
    pub machine_id: MachineId,
    pub artifact_length_bytes: u64,
    pub prepared_observation_id: Option<ContentDigest>,
    pub configuration: MachineConfiguration,
    pub input_schedule: InputSchedule,
    pub observation: ObservationRequest,
    pub execution_controls: ExecutionControls,
}

/// Terminal machine metrics. Capability outputs are delivered through their
/// distinct callback and are not duplicated in this record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunFinishedRecord {
    pub common: CommonMetrics,
}

impl From<&RunResult> for RunFinishedRecord {
    fn from(result: &RunResult) -> Self {
        Self {
            common: result.common.clone(),
        }
    }
}

impl From<&ExecutionRequest> for RunStartedRecord {
    fn from(request: &ExecutionRequest) -> Self {
        Self {
            schema_version: request.schema_version,
            run_id: request.run_id.clone(),
            machine_id: request.machine_id.clone(),
            artifact_length_bytes: request.artifact.len() as u64,
            prepared_observation_id: request.prepared_observation_id.clone(),
            configuration: request.configuration.clone(),
            input_schedule: request.input_schedule.clone(),
            observation: request.observation.clone(),
            execution_controls: request.execution_controls.clone(),
        }
    }
}

/// Owned form of one core emission for eventual Python conversion.
///
/// This is a transport representation, not a collected-run model. Consumers
/// receive bounded slices of these records; no code here retains run history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "record", rename_all = "snake_case")]
pub enum OwnedEmission {
    RunStarted(RunStartedRecord),
    Event(ExecutionEvent),
    NativeEvidence(NativeEvidenceEnvelope),
    InputValueEvidence(InputValueEvidence),
    Frame(FrameArtifact),
    Snapshot(SnapshotArtifact),
    EvidenceReceipt(EvidenceReceipt),
    RunFinished(RunFinishedRecord),
}

impl<'a> From<Emission<'a>> for OwnedEmission {
    fn from(emission: Emission<'a>) -> Self {
        match emission {
            Emission::RunStarted(request) => Self::RunStarted(request.into()),
            Emission::Event(event) => Self::Event(event.clone()),
            Emission::NativeEvidence(evidence) => Self::NativeEvidence(evidence.clone()),
            Emission::InputValueEvidence(evidence) => Self::InputValueEvidence(evidence.clone()),
            Emission::Frame(frame) => Self::Frame(frame.clone()),
            Emission::Snapshot(snapshot) => Self::Snapshot(snapshot.clone()),
            Emission::EvidenceReceipt(receipt) => Self::EvidenceReceipt(receipt.clone()),
            Emission::RunFinished(result) => Self::RunFinished(result.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PythonSinkCallbackKind {
    EmissionBatch,
    CapabilityOutputs,
    CapabilityReceipts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PythonSinkFailure {
    pub sink_index: usize,
    pub sink_identity: Option<String>,
    pub callback: PythonSinkCallbackKind,
    pub batch_size: usize,
    pub successfully_delivered_items: usize,
    pub exception_type: String,
    pub exception_message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PythonSinkDeliveryStatus {
    NotRequested,
    Complete,
    Failed,
}

/// Separate accounting for Python consumer delivery. This is not evidence,
/// machine-execution, or reference-recorder status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PythonSinkDeliveryOutcome {
    pub status: PythonSinkDeliveryStatus,
    pub sink_count: usize,
    pub batches_delivered: u64,
    pub items_delivered: u64,
    pub logical_bytes_delivered: u64,
    pub failure: Option<PythonSinkFailure>,
}

impl PythonSinkDeliveryOutcome {
    pub const fn not_requested() -> Self {
        Self {
            status: PythonSinkDeliveryStatus::NotRequested,
            sink_count: 0,
            batches_delivered: 0,
            items_delivered: 0,
            logical_bytes_delivered: 0,
            failure: None,
        }
    }
}

/// Rust-side callback contract implemented by the Python consumer adapter.
pub trait PythonEmissionConsumer {
    fn consumer_count(&self) -> usize {
        1
    }

    fn emit_batch(&mut self, emissions: &[OwnedEmission]) -> Result<(), PythonSinkFailure>;

    fn record_capability_outputs(
        &mut self,
        outputs: &[CapabilityOutput],
    ) -> Result<(), PythonSinkFailure>;

    fn record_capability_receipts(
        &mut self,
        receipts: &[CapabilityReceipt],
    ) -> Result<(), PythonSinkFailure>;
}

/// Fan-out adapter for Python objects implementing `emit_batch`,
/// `record_capability_outputs`, and `record_capability_receipts`.
///
/// Callback arguments are owned Python values decoded once for fan-out and
/// should be treated as read-only by consumers. The synchronous bridge holds
/// the GIL while invoking each callback; no worker or queue is involved.
#[cfg(feature = "python")]
pub struct PythonCallbackConsumer {
    consumers: Vec<(Option<String>, Py<PyAny>)>,
}

#[cfg(feature = "python")]
impl PythonCallbackConsumer {
    pub fn new(consumers: Vec<(Option<String>, Py<PyAny>)>) -> Self {
        Self { consumers }
    }

    fn decode_batch<T: Serialize>(py: Python<'_>, values: &[T]) -> PyResult<Py<PyAny>> {
        let json = serde_json::to_string(values)
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))?;
        let json_module = PyModule::import(py, "json")?;
        Ok(json_module.getattr("loads")?.call1((json,))?.unbind())
    }

    fn call<T: Serialize>(
        &mut self,
        method: &str,
        values: &[T],
        callback: PythonSinkCallbackKind,
    ) -> Result<(), PythonSinkFailure> {
        Python::attach(|py| {
            let batch = Self::decode_batch(py, values)
                .map_err(|error| Self::failure(callback, error, values.len(), 0, None, 0))?;
            let mut delivered = 0;
            for (index, (identity, consumer)) in self.consumers.iter().enumerate() {
                if let Err(error) = consumer.bind(py).call_method1(method, (batch.bind(py),)) {
                    return Err(Self::failure(
                        callback,
                        error,
                        values.len(),
                        index,
                        identity.clone(),
                        delivered,
                    ));
                }
                delivered += values.len();
            }
            Ok(())
        })
    }

    fn failure(
        callback: PythonSinkCallbackKind,
        error: PyErr,
        batch_size: usize,
        sink_index: usize,
        sink_identity: Option<String>,
        successfully_delivered_items: usize,
    ) -> PythonSinkFailure {
        Python::attach(|py| PythonSinkFailure {
            sink_index,
            sink_identity,
            callback,
            batch_size,
            successfully_delivered_items,
            exception_type: error
                .get_type(py)
                .name()
                .map(|name| name.to_string())
                .unwrap_or_else(|_| "PythonException".into()),
            exception_message: error.to_string(),
        })
    }
}

#[cfg(feature = "python")]
impl PythonEmissionConsumer for PythonCallbackConsumer {
    fn consumer_count(&self) -> usize {
        self.consumers.len()
    }

    fn emit_batch(&mut self, emissions: &[OwnedEmission]) -> Result<(), PythonSinkFailure> {
        self.call(
            "emit_batch",
            emissions,
            PythonSinkCallbackKind::EmissionBatch,
        )
    }

    fn record_capability_outputs(
        &mut self,
        outputs: &[CapabilityOutput],
    ) -> Result<(), PythonSinkFailure> {
        self.call(
            "record_capability_outputs",
            outputs,
            PythonSinkCallbackKind::CapabilityOutputs,
        )
    }

    fn record_capability_receipts(
        &mut self,
        receipts: &[CapabilityReceipt],
    ) -> Result<(), PythonSinkFailure> {
        self.call(
            "record_capability_receipts",
            receipts,
            PythonSinkCallbackKind::CapabilityReceipts,
        )
    }
}

/// Conservative initial batching targets. These are implementation defaults,
/// not frozen public tuning promises; benchmark results should guide any API
/// commitment.
pub const DEFAULT_PYTHON_BATCH_MAX_ITEMS: usize = 32;
pub const DEFAULT_PYTHON_BATCH_MAX_LOGICAL_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PythonBatchPolicy {
    pub max_items: usize,
    pub max_logical_bytes: usize,
}

impl Default for PythonBatchPolicy {
    fn default() -> Self {
        Self {
            max_items: DEFAULT_PYTHON_BATCH_MAX_ITEMS,
            max_logical_bytes: DEFAULT_PYTHON_BATCH_MAX_LOGICAL_BYTES,
        }
    }
}

#[derive(Default)]
struct PendingBatch {
    records: Vec<OwnedEmission>,
    logical_bytes: usize,
}

/// Synchronous bounded fan-out from borrowed core emissions to owned consumer
/// records. It retains at most one bounded batch; a single valid record larger
/// than the target byte size is delivered by itself.
pub struct PythonBatchingSink {
    consumers: Vec<Box<dyn PythonEmissionConsumer + Send>>,
    identities: Vec<Option<String>>,
    policy: PythonBatchPolicy,
    batch: PendingBatch,
    outcome: PythonSinkDeliveryOutcome,
    failure: Option<PythonSinkFailure>,
}

impl PythonBatchingSink {
    pub fn new(
        consumers: Vec<(Option<String>, Box<dyn PythonEmissionConsumer + Send>)>,
        policy: PythonBatchPolicy,
    ) -> Result<Self, SinkError> {
        if policy.max_items == 0 || policy.max_logical_bytes == 0 {
            return Err(SinkError::new(
                "Python batch item and logical-byte limits must be non-zero",
            ));
        }
        let (identities, consumers): (Vec<_>, Vec<_>) = consumers.into_iter().unzip();
        let sink_count = consumers
            .iter()
            .map(|consumer| consumer.consumer_count())
            .sum();
        Ok(Self {
            outcome: PythonSinkDeliveryOutcome {
                status: if consumers.is_empty() {
                    PythonSinkDeliveryStatus::NotRequested
                } else {
                    PythonSinkDeliveryStatus::Complete
                },
                sink_count,
                batches_delivered: 0,
                items_delivered: 0,
                logical_bytes_delivered: 0,
                failure: None,
            },
            consumers,
            identities,
            policy,
            batch: PendingBatch::default(),
            failure: None,
        })
    }

    pub fn outcome(&self) -> PythonSinkDeliveryOutcome {
        self.outcome.clone()
    }

    /// Deliver anything still buffered. Call at run completion, including
    /// when no subsequent emission will arrive to trigger a limit flush.
    pub fn finish(&mut self) -> Result<(), SinkError> {
        self.flush_batch()
    }

    fn record_size(record: &OwnedEmission) -> Result<usize, SinkError> {
        let bytes = canonical_json_bytes(record).map_err(SinkError::new)?;
        Ok(bytes.len())
    }

    fn flush_batch(&mut self) -> Result<(), SinkError> {
        if let Some(failure) = &self.failure {
            return Err(SinkError::new(format!(
                "Python sink delivery previously failed: {}: {}",
                failure.exception_type, failure.exception_message
            )));
        }
        if self.batch.records.is_empty() {
            return Ok(());
        }
        let batch = mem::take(&mut self.batch);
        let batch_size = batch.records.len();
        let mut sink_offset = 0;
        let mut delivered_items = 0;
        for index in 0..self.consumers.len() {
            let fanout_count = self.consumers[index].consumer_count();
            if let Err(mut failure) = self.consumers[index].emit_batch(&batch.records) {
                if fanout_count == 1 {
                    failure.sink_index = sink_offset;
                    failure.sink_identity = self.identities[index].clone();
                } else {
                    failure.sink_index += sink_offset;
                }
                failure.callback = PythonSinkCallbackKind::EmissionBatch;
                failure.batch_size = batch_size;
                failure.successfully_delivered_items =
                    delivered_items + failure.successfully_delivered_items;
                self.failure = Some(failure.clone());
                self.outcome.status = PythonSinkDeliveryStatus::Failed;
                self.outcome.failure = Some(failure);
                return Err(SinkError::new("Python emission batch callback failed"));
            }
            delivered_items += batch_size * fanout_count;
            self.outcome.batches_delivered += fanout_count as u64;
            sink_offset += fanout_count;
        }
        self.outcome.items_delivered += delivered_items as u64;
        self.outcome.logical_bytes_delivered += (batch.logical_bytes as u64) * sink_offset as u64;
        Ok(())
    }

    fn deliver_barrier(&mut self, emission: Emission<'_>) -> Result<(), SinkError> {
        self.flush_batch()?;
        self.append(emission)?;
        self.flush_batch()
    }

    fn append(&mut self, emission: Emission<'_>) -> Result<(), SinkError> {
        let record = OwnedEmission::from(emission);
        let bytes = Self::record_size(&record)?;
        if !self.batch.records.is_empty()
            && (self.batch.records.len() >= self.policy.max_items
                || self.batch.logical_bytes.saturating_add(bytes) > self.policy.max_logical_bytes)
        {
            self.flush_batch()?;
        }
        self.batch.logical_bytes = self.batch.logical_bytes.saturating_add(bytes);
        self.batch.records.push(record);
        if self.batch.records.len() >= self.policy.max_items
            || self.batch.logical_bytes >= self.policy.max_logical_bytes
            || bytes > self.policy.max_logical_bytes
        {
            self.flush_batch()?;
        }
        Ok(())
    }

    fn deliver_capability_outputs(&mut self, outputs: &[CapabilityOutput]) {
        if let Err(error) = self.flush_batch() {
            let _ = error;
            return;
        }
        let logical_bytes = outputs
            .iter()
            .filter_map(|output| canonical_json_bytes(output).ok())
            .map(|bytes| bytes.len() as u64)
            .sum::<u64>();
        let mut sink_offset = 0;
        let mut delivered_items = 0;
        for index in 0..self.consumers.len() {
            let fanout_count = self.consumers[index].consumer_count();
            if let Err(mut failure) = self.consumers[index].record_capability_outputs(outputs) {
                if fanout_count == 1 {
                    failure.sink_index = sink_offset;
                    failure.sink_identity = self.identities[index].clone();
                } else {
                    failure.sink_index += sink_offset;
                }
                failure.callback = PythonSinkCallbackKind::CapabilityOutputs;
                failure.batch_size = outputs.len();
                failure.successfully_delivered_items += delivered_items;
                self.latch_failure(failure);
                break;
            }
            self.outcome.batches_delivered += fanout_count as u64;
            self.outcome.items_delivered += (outputs.len() * fanout_count) as u64;
            self.outcome.logical_bytes_delivered += logical_bytes * fanout_count as u64;
            delivered_items += outputs.len() * fanout_count;
            sink_offset += fanout_count;
        }
    }

    fn deliver_capability_receipts(&mut self, receipts: &[CapabilityReceipt]) {
        if let Err(error) = self.flush_batch() {
            let _ = error;
            return;
        }
        let logical_bytes = receipts
            .iter()
            .filter_map(|receipt| canonical_json_bytes(receipt).ok())
            .map(|bytes| bytes.len() as u64)
            .sum::<u64>();
        let mut sink_offset = 0;
        let mut delivered_items = 0;
        for index in 0..self.consumers.len() {
            let fanout_count = self.consumers[index].consumer_count();
            if let Err(mut failure) = self.consumers[index].record_capability_receipts(receipts) {
                if fanout_count == 1 {
                    failure.sink_index = sink_offset;
                    failure.sink_identity = self.identities[index].clone();
                } else {
                    failure.sink_index += sink_offset;
                }
                failure.callback = PythonSinkCallbackKind::CapabilityReceipts;
                failure.batch_size = receipts.len();
                failure.successfully_delivered_items += delivered_items;
                self.latch_failure(failure);
                break;
            }
            self.outcome.batches_delivered += fanout_count as u64;
            self.outcome.items_delivered += (receipts.len() * fanout_count) as u64;
            self.outcome.logical_bytes_delivered += logical_bytes * fanout_count as u64;
            delivered_items += receipts.len() * fanout_count;
            sink_offset += fanout_count;
        }
    }

    fn latch_failure(&mut self, failure: PythonSinkFailure) {
        self.outcome.status = PythonSinkDeliveryStatus::Failed;
        self.outcome.failure = Some(failure.clone());
        self.failure = Some(failure);
    }
}

impl EmissionSink for PythonBatchingSink {
    fn emit(&mut self, emission: Emission<'_>) -> Result<(), SinkError> {
        if self.consumers.is_empty() || self.outcome.sink_count == 0 {
            return Ok(());
        }
        if self.failure.is_some() {
            return self.flush_batch();
        }
        match emission {
            Emission::RunStarted(_) | Emission::EvidenceReceipt(_) | Emission::RunFinished(_) => {
                self.deliver_barrier(emission)
            }
            other => self.append(other),
        }
    }

    fn record_capability_outputs(&mut self, outputs: &[CapabilityOutput]) {
        if !self.consumers.is_empty() && self.failure.is_none() {
            self.deliver_capability_outputs(outputs);
        }
    }

    fn record_capability_receipts(&mut self, receipts: &[CapabilityReceipt]) {
        if !self.consumers.is_empty() && self.failure.is_none() {
            self.deliver_capability_receipts(receipts);
        }
    }
}

#[cfg(test)]
mod batching_tests {
    use std::sync::{Arc, Mutex};

    use glassvm_core::{FrameArtifact, MachineId, SchemaRef, VersionStamp};

    use super::*;

    #[derive(Default)]
    struct Calls(Arc<Mutex<Vec<Vec<OwnedEmission>>>>);

    impl PythonEmissionConsumer for Calls {
        fn emit_batch(&mut self, emissions: &[OwnedEmission]) -> Result<(), PythonSinkFailure> {
            self.0.lock().unwrap().push(emissions.to_vec());
            Ok(())
        }

        fn record_capability_outputs(
            &mut self,
            _outputs: &[CapabilityOutput],
        ) -> Result<(), PythonSinkFailure> {
            Ok(())
        }

        fn record_capability_receipts(
            &mut self,
            _receipts: &[CapabilityReceipt],
        ) -> Result<(), PythonSinkFailure> {
            Ok(())
        }
    }

    fn frame(index: u64, payload_size: usize) -> FrameArtifact {
        FrameArtifact::full(
            RunId::from("run"),
            MachineId::from("test-machine"),
            VersionStamp::from("test.v1"),
            SchemaRef::new("test.frame", SchemaVersion::new(1, 0, 0)),
            index,
            index,
            index,
            vec![index as u8; payload_size],
        )
    }

    fn sink(
        calls: &Arc<Mutex<Vec<Vec<OwnedEmission>>>>,
        policy: PythonBatchPolicy,
    ) -> PythonBatchingSink {
        PythonBatchingSink::new(
            vec![(
                Some("test-consumer".into()),
                Box::new(Calls(Arc::clone(calls))),
            )],
            policy,
        )
        .unwrap()
    }

    #[test]
    fn batches_respect_item_bound_and_preserve_order() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut sink = sink(
            &calls,
            PythonBatchPolicy {
                max_items: 2,
                max_logical_bytes: usize::MAX,
            },
        );
        for index in 0..3 {
            let frame = frame(index, 1);
            sink.emit(Emission::Frame(&frame)).unwrap();
        }
        sink.finish().unwrap();

        let calls = calls.lock().unwrap();
        assert_eq!(calls.iter().map(Vec::len).collect::<Vec<_>>(), [2, 1]);
        let sequences = calls
            .iter()
            .flatten()
            .map(|record| match record {
                OwnedEmission::Frame(frame) => frame.sequence,
                other => panic!("unexpected record: {other:?}"),
            })
            .collect::<Vec<_>>();
        assert_eq!(sequences, [0, 1, 2]);
    }

    #[test]
    fn capability_callbacks_flush_pending_emissions_first() {
        struct Ordered(Arc<Mutex<Vec<&'static str>>>);

        impl PythonEmissionConsumer for Ordered {
            fn emit_batch(
                &mut self,
                _emissions: &[OwnedEmission],
            ) -> Result<(), PythonSinkFailure> {
                self.0.lock().unwrap().push("emissions");
                Ok(())
            }

            fn record_capability_outputs(
                &mut self,
                _outputs: &[CapabilityOutput],
            ) -> Result<(), PythonSinkFailure> {
                self.0.lock().unwrap().push("outputs");
                Ok(())
            }

            fn record_capability_receipts(
                &mut self,
                _receipts: &[CapabilityReceipt],
            ) -> Result<(), PythonSinkFailure> {
                self.0.lock().unwrap().push("receipts");
                Ok(())
            }
        }

        let order = Arc::new(Mutex::new(Vec::new()));
        let consumer: Box<dyn PythonEmissionConsumer + Send> =
            Box::new(Ordered(Arc::clone(&order)));
        let mut sink = PythonBatchingSink::new(
            vec![(Some("ordered".into()), consumer)],
            PythonBatchPolicy::default(),
        )
        .unwrap();
        let first = frame(0, 1);
        let second = frame(1, 1);
        sink.emit(Emission::Frame(&first)).unwrap();
        sink.record_capability_outputs(&[]);
        sink.emit(Emission::Frame(&second)).unwrap();
        sink.record_capability_receipts(&[]);
        sink.finish().unwrap();
        assert_eq!(
            *order.lock().unwrap(),
            ["emissions", "outputs", "emissions", "receipts"]
        );
    }

    #[test]
    fn byte_limit_flushes_and_oversized_valid_record_is_singleton() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let first = frame(0, 8);
        let second = frame(2, 8);
        let small_size =
            PythonBatchingSink::record_size(&OwnedEmission::Frame(first.clone())).unwrap();
        let mut sink = sink(
            &calls,
            PythonBatchPolicy {
                max_items: 10,
                max_logical_bytes: small_size * 2,
            },
        );
        let oversized = frame(1, 512);
        sink.emit(Emission::Frame(&first)).unwrap();
        sink.emit(Emission::Frame(&second)).unwrap();
        sink.emit(Emission::Frame(&oversized)).unwrap();
        sink.finish().unwrap();

        let calls = calls.lock().unwrap();
        assert_eq!(calls.iter().map(Vec::len).collect::<Vec<_>>(), [2, 1]);
        let OwnedEmission::Frame(large_frame) = &calls[1][0] else {
            panic!("expected frame artifact singleton");
        };
        assert!(large_frame.logical_payload_bytes() > 80);
    }

    #[test]
    fn invalid_zero_batch_limits_are_rejected() {
        let error = PythonBatchingSink::new(
            Vec::new(),
            PythonBatchPolicy {
                max_items: 0,
                max_logical_bytes: 1,
            },
        )
        .err()
        .unwrap();
        assert!(error.to_string().contains("must be non-zero"));
    }

    #[cfg(feature = "python")]
    #[test]
    fn python_fanout_reuses_one_owned_batch_value() {
        use std::ffi::CString;

        use pyo3::types::PyModule;

        Python::initialize();
        Python::attach(|py| {
            let code = CString::new(
                "class Sink:\n    def __init__(self): self.batches = []\n    def emit_batch(self, batch): self.batches.append(batch)\n    def record_capability_outputs(self, outputs): pass\n    def record_capability_receipts(self, receipts): pass\n",
            )
            .unwrap();
            let module =
                PyModule::from_code(py, code.as_c_str(), c"sink_test.py", c"sink_test").unwrap();
            let first = module.getattr("Sink").unwrap().call0().unwrap().unbind();
            let second = module.getattr("Sink").unwrap().call0().unwrap().unbind();
            let mut consumer = PythonCallbackConsumer::new(vec![
                (Some("first".into()), first.clone_ref(py)),
                (Some("second".into()), second.clone_ref(py)),
            ]);
            let record = OwnedEmission::Frame(frame(0, 4));
            consumer.emit_batch(&[record]).unwrap();

            let first_batch = first
                .bind(py)
                .getattr("batches")
                .unwrap()
                .get_item(0)
                .unwrap();
            let second_batch = second
                .bind(py)
                .getattr("batches")
                .unwrap()
                .get_item(0)
                .unwrap();
            assert!(first_batch.is(&second_batch));
            assert_eq!(first_batch.len().unwrap(), 1);
        });
    }
}
