use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    BudgetKind, CapabilityReceipt, ChannelReceipt, ChannelStatus, ContentDigest, EmissionChannel,
    EmissionFailure, EvidenceReceipt, EvidenceStatus, ExecutionEvent, ExecutionRequest,
    InputValueEvidence, MachineId, NativeEvidenceEnvelope, ObservationRequest, OverflowPolicy,
    RunId, RunResult, SchemaRef, SchemaVersion, VersionStamp, canonical_json_bytes,
    state_fingerprint,
};
use glassvm_normalizer_contract::CapabilityOutput;

pub const FRAME_ARTIFACT_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

/// Requested raw evidence for one completed machine frame.
///
/// `frame`, `step`, and `sequence` intentionally have different meanings:
/// `frame` identifies the machine frame, `step` locates it in machine
/// execution, and `sequence` is the source stream coordinate. Consumers must
/// not treat `sequence` as a cross-channel merge or storage-order key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameArtifact {
    pub schema: SchemaRef,
    pub representation_schema: SchemaRef,
    pub run_id: RunId,
    pub machine_id: MachineId,
    pub machine_version: VersionStamp,
    pub sequence: u64,
    pub step: u64,
    pub frame: u64,
    pub evidence: FrameEvidence,
}

/// Typed frame evidence. `representation_schema` identifies the versioned
/// bundle representation for the selected variant; it replaces a free-form
/// fingerprint algorithm string or JSON payload contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameEvidence {
    Fingerprint { bytes: Vec<u8> },
    Full { bytes: Vec<u8> },
}

impl FrameArtifact {
    #[allow(clippy::too_many_arguments)]
    pub fn fingerprint(
        run_id: RunId,
        machine_id: MachineId,
        machine_version: VersionStamp,
        representation_schema: SchemaRef,
        sequence: u64,
        step: u64,
        frame: u64,
        bytes: Vec<u8>,
    ) -> Self {
        Self::new(
            run_id,
            machine_id,
            machine_version,
            representation_schema,
            sequence,
            step,
            frame,
            FrameEvidence::Fingerprint { bytes },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn full(
        run_id: RunId,
        machine_id: MachineId,
        machine_version: VersionStamp,
        representation_schema: SchemaRef,
        sequence: u64,
        step: u64,
        frame: u64,
        bytes: Vec<u8>,
    ) -> Self {
        Self::new(
            run_id,
            machine_id,
            machine_version,
            representation_schema,
            sequence,
            step,
            frame,
            FrameEvidence::Full { bytes },
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        run_id: RunId,
        machine_id: MachineId,
        machine_version: VersionStamp,
        representation_schema: SchemaRef,
        sequence: u64,
        step: u64,
        frame: u64,
        evidence: FrameEvidence,
    ) -> Self {
        Self {
            schema: SchemaRef::new("glassvm.frame_artifact", FRAME_ARTIFACT_SCHEMA_VERSION),
            representation_schema,
            run_id,
            machine_id,
            machine_version,
            sequence,
            step,
            frame,
            evidence,
        }
    }

    pub fn logical_payload_bytes(&self) -> u64 {
        match &self.evidence {
            FrameEvidence::Fingerprint { bytes } | FrameEvidence::Full { bytes } => {
                bytes.len() as u64
            }
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let expected = SchemaRef::new("glassvm.frame_artifact", FRAME_ARTIFACT_SCHEMA_VERSION);
        if self.schema != expected {
            return Err(format!(
                "unsupported frame-artifact envelope schema {} {}",
                self.schema.id, self.schema.version
            ));
        }
        if self.representation_schema.id.trim().is_empty() {
            return Err("frame representation schema cannot be empty".into());
        }
        if self.logical_payload_bytes() == 0 {
            return Err("frame artifact evidence cannot be empty".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotArtifact {
    pub schema: SchemaRef,
    pub state_schema: SchemaRef,
    pub run_id: RunId,
    pub arch: MachineId,
    pub machine_version: VersionStamp,
    pub sequence: u64,
    pub step: u64,
    pub byte_len: u64,
    pub digest: ContentDigest,
    pub bytes: Vec<u8>,
}

impl SnapshotArtifact {
    pub fn new(
        run_id: RunId,
        arch: MachineId,
        machine_version: VersionStamp,
        sequence: u64,
        step: u64,
        bytes: Vec<u8>,
    ) -> Self {
        let state_schema = SchemaRef::new(
            format!("{}.state.snapshot", arch.as_str()),
            SchemaVersion::V1,
        );
        Self::new_typed(
            run_id,
            arch,
            machine_version,
            state_schema,
            sequence,
            step,
            bytes,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_typed(
        run_id: RunId,
        arch: MachineId,
        machine_version: VersionStamp,
        state_schema: SchemaRef,
        sequence: u64,
        step: u64,
        bytes: Vec<u8>,
    ) -> Self {
        let byte_len = bytes.len() as u64;
        let digest = state_fingerprint(&state_schema, &bytes);
        Self {
            schema: SchemaRef::new("glassvm.snapshot", SchemaVersion::V1),
            state_schema,
            run_id,
            arch,
            machine_version,
            sequence,
            step,
            byte_len,
            digest,
            bytes,
        }
    }

    pub fn validate_integrity(&self) -> Result<(), String> {
        let expected_schema = SchemaRef::new("glassvm.snapshot", SchemaVersion::V1);
        if self.schema != expected_schema {
            return Err(format!(
                "unsupported snapshot envelope schema {:?} version {}",
                self.schema.id, self.schema.version
            ));
        }
        if self.byte_len != self.bytes.len() as u64 {
            return Err(format!(
                "snapshot declares {} bytes but contains {}",
                self.byte_len,
                self.bytes.len()
            ));
        }
        let actual = state_fingerprint(&self.state_schema, &self.bytes);
        if self.digest != actual {
            return Err(format!(
                "snapshot digest mismatch: expected {}, computed {}",
                self.digest, actual
            ));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum Emission<'a> {
    RunStarted(&'a ExecutionRequest),
    Event(&'a ExecutionEvent),
    NativeEvidence(&'a NativeEvidenceEnvelope),
    InputValueEvidence(&'a InputValueEvidence),
    Frame(&'a FrameArtifact),
    Snapshot(&'a SnapshotArtifact),
    EvidenceReceipt(&'a EvidenceReceipt),
    RunFinished(&'a RunResult),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinkError {
    message: String,
}

impl SinkError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for SinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl std::error::Error for SinkError {}

/// Caller-owned destination for one execution's transient emissions.
pub trait EmissionSink: Send {
    fn emit(&mut self, emission: Emission<'_>) -> Result<(), SinkError>;

    /// Record canonical capability outputs for durable consumers.
    fn record_capability_outputs(&mut self, _outputs: &[CapabilityOutput]) {}

    /// Record retrospective capability fulfillment without introducing a new
    /// emission or transferring ownership of observed evidence.
    fn record_capability_receipts(&mut self, _receipts: &[CapabilityReceipt]) {}
}

impl<T: EmissionSink + ?Sized> EmissionSink for &mut T {
    fn emit(&mut self, emission: Emission<'_>) -> Result<(), SinkError> {
        (**self).emit(emission)
    }

    fn record_capability_receipts(&mut self, receipts: &[CapabilityReceipt]) {
        (**self).record_capability_receipts(receipts);
    }

    fn record_capability_outputs(&mut self, outputs: &[CapabilityOutput]) {
        (**self).record_capability_outputs(outputs);
    }
}

#[derive(Debug, Default)]
pub struct NullSink;

impl EmissionSink for NullSink {
    fn emit(&mut self, _emission: Emission<'_>) -> Result<(), SinkError> {
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct ChannelAccounting {
    item_count: u64,
    logical_bytes: u64,
    status: Option<ChannelStatus>,
}

impl ChannelAccounting {
    fn receipt(self, channel: EmissionChannel, requested: bool) -> ChannelReceipt {
        ChannelReceipt {
            channel,
            requested,
            status: if requested {
                self.status.unwrap_or(ChannelStatus::Complete)
            } else {
                ChannelStatus::Omitted
            },
            item_count: self.item_count,
            logical_bytes: self.logical_bytes,
            encoded_bytes: None,
        }
    }
}

/// Synchronous budget/accounting sink used by callers and conformance tests.
///
/// This sink measures logical payload bytes before forwarding an emission. It
/// does not measure encoded or filesystem bytes and does not retain a trace.
/// `emit_receipt()` is deliberately explicit so a caller can publish it
/// through its own terminal-envelope policy exactly once.
#[derive(Debug)]
pub struct BudgetedSink<S> {
    inner: S,
    request: ObservationRequest,
    run_id: RunId,
    machine_id: MachineId,
    machine_version: VersionStamp,
    normalized_events: ChannelAccounting,
    capabilities: ChannelAccounting,
    native_evidence: ChannelAccounting,
    frames: ChannelAccounting,
    snapshots: ChannelAccounting,
    input_value_evidence: ChannelAccounting,
    envelope: ChannelAccounting,
    capability_receipts: Vec<CapabilityReceipt>,
    receipt_emitted: bool,
    total_logical_bytes: u64,
    exceeded: Option<BudgetKind>,
    failure: Option<EmissionFailure>,
}

impl<S> BudgetedSink<S> {
    pub fn new(
        inner: S,
        request: ObservationRequest,
        run_id: RunId,
        machine_id: MachineId,
        machine_version: VersionStamp,
    ) -> Result<Self, SinkError> {
        request.validate().map_err(SinkError::new)?;
        Ok(Self {
            inner,
            request,
            run_id,
            machine_id,
            machine_version,
            normalized_events: ChannelAccounting::default(),
            capabilities: ChannelAccounting::default(),
            native_evidence: ChannelAccounting::default(),
            frames: ChannelAccounting::default(),
            snapshots: ChannelAccounting::default(),
            input_value_evidence: ChannelAccounting::default(),
            envelope: ChannelAccounting::default(),
            capability_receipts: Vec::new(),
            receipt_emitted: false,
            total_logical_bytes: 0,
            exceeded: None,
            failure: None,
        })
    }

    pub fn receipt(&self) -> EvidenceReceipt {
        let mut channels = self.channels();
        let envelope_complete = self.envelope.item_count >= 2;
        if !envelope_complete && self.failure.is_none() {
            if let Some(channel) = channels
                .iter_mut()
                .find(|channel| channel.channel == EmissionChannel::Envelope)
            {
                channel.status = ChannelStatus::Omitted;
            }
        }
        let capability_failed = self
            .capability_receipts
            .iter()
            .any(|receipt| receipt.status == glassvm_normalizer_contract::CapabilityStatus::Failed);
        let capability_incomplete = self.capability_receipts.iter().any(|receipt| {
            receipt.status == glassvm_normalizer_contract::CapabilityStatus::Incomplete
        });
        let status = if self.failure.is_some() || capability_failed {
            EvidenceStatus::Failed
        } else if self
            .channels()
            .iter()
            .any(|channel| channel.status == ChannelStatus::Truncated)
            || !envelope_complete
            || capability_incomplete
        {
            EvidenceStatus::Incomplete
        } else {
            EvidenceStatus::Complete
        };
        EvidenceReceipt {
            schema_version: crate::EMISSION_SCHEMA_VERSION,
            run_id: self.run_id.clone(),
            machine_id: self.machine_id.clone(),
            machine_version: self.machine_version.clone(),
            requested_guarantee: self.request.guarantee,
            fulfilled_guarantee: (status == EvidenceStatus::Complete)
                .then_some(self.request.guarantee),
            status,
            channels,
            capabilities: self.capability_receipts.clone(),
            budgets: crate::BudgetReceipt {
                configured: self.request.budgets.clone(),
                exceeded: self.exceeded,
                observed_logical_bytes: self.total_logical_bytes,
            },
            failure: self.failure.clone(),
        }
    }

    pub fn into_inner(self) -> S {
        self.inner
    }

    /// Build and forward the current receipt through the downstream sink.
    ///
    /// Publication is caller-owned but exactly once per budgeted sink. A
    /// downstream publication failure is recorded as sink failure so a later
    /// inspection of `receipt()` reports the failed publication.
    pub fn emit_receipt(&mut self) -> Result<EvidenceReceipt, SinkError>
    where
        S: EmissionSink,
    {
        if self.receipt_emitted {
            return Err(SinkError::new(
                "budgeted sink already emitted an evidence receipt",
            ));
        }
        let receipt = self.receipt();
        self.emit(Emission::EvidenceReceipt(&receipt))?;
        self.receipt_emitted = true;
        Ok(receipt)
    }

    fn channels(&self) -> Vec<ChannelReceipt> {
        vec![
            self.normalized_events.receipt(
                EmissionChannel::NormalizedEvents,
                !matches!(
                    self.request.normalized_events.events,
                    crate::EventSelection::None
                ),
            ),
            self.capabilities.receipt(
                EmissionChannel::Capabilities,
                !self.request.capabilities.is_empty(),
            ),
            self.native_evidence.receipt(
                EmissionChannel::NativeEvidence,
                self.request.native_evidence.enabled,
            ),
            self.frames.receipt(
                EmissionChannel::Frames,
                !matches!(self.request.frames.capture, crate::FrameCapture::None),
            ),
            self.snapshots.receipt(
                EmissionChannel::Snapshots,
                !matches!(self.request.snapshots.capture, crate::SnapshotCapture::None),
            ),
            self.input_value_evidence.receipt(
                EmissionChannel::InputValueEvidence,
                self.request.input_value_evidence.enabled,
            ),
            self.envelope.receipt(EmissionChannel::Envelope, true),
        ]
    }

    fn account_plan(
        &mut self,
        channel: EmissionChannel,
        item_count: u64,
        logical_bytes: u64,
    ) -> Result<bool, SinkError> {
        let (current_items, current_bytes, item_limit, byte_limit, item_kind, byte_kind) =
            match channel {
                EmissionChannel::NormalizedEvents => (
                    self.normalized_events.item_count,
                    self.normalized_events.logical_bytes,
                    self.request.budgets.max_normalized_events,
                    self.request.budgets.max_normalized_bytes,
                    BudgetKind::NormalizedEvents,
                    BudgetKind::NormalizedBytes,
                ),
                EmissionChannel::Capabilities => (
                    self.capabilities.item_count,
                    self.capabilities.logical_bytes,
                    None,
                    self.request.budgets.max_capability_bytes,
                    BudgetKind::CapabilityBytes,
                    BudgetKind::CapabilityBytes,
                ),
                EmissionChannel::NativeEvidence => (
                    self.native_evidence.item_count,
                    self.native_evidence.logical_bytes,
                    self.request.budgets.max_native_events,
                    self.request.budgets.max_native_bytes,
                    BudgetKind::NativeEvents,
                    BudgetKind::NativeBytes,
                ),
                EmissionChannel::Snapshots => (
                    self.snapshots.item_count,
                    self.snapshots.logical_bytes,
                    None,
                    self.request.budgets.max_snapshot_bytes,
                    BudgetKind::SnapshotBytes,
                    BudgetKind::SnapshotBytes,
                ),
                EmissionChannel::InputValueEvidence => (
                    self.input_value_evidence.item_count,
                    self.input_value_evidence.logical_bytes,
                    self.request.budgets.max_input_value_events,
                    self.request.budgets.max_input_value_bytes,
                    BudgetKind::InputValueEvents,
                    BudgetKind::InputValueBytes,
                ),
                EmissionChannel::Frames => (
                    0,
                    0,
                    None,
                    self.request.budgets.max_frame_bytes,
                    BudgetKind::FrameBytes,
                    BudgetKind::FrameBytes,
                ),
                EmissionChannel::Envelope => (
                    0,
                    0,
                    None,
                    None,
                    BudgetKind::TotalLogicalBytes,
                    BudgetKind::TotalLogicalBytes,
                ),
            };
        let next_items = current_items
            .checked_add(item_count)
            .ok_or_else(|| SinkError::new("emission item count overflow"))?;
        let next_bytes = current_bytes
            .checked_add(logical_bytes)
            .ok_or_else(|| SinkError::new("emission byte count overflow"))?;
        let next_total = self
            .total_logical_bytes
            .checked_add(logical_bytes)
            .ok_or_else(|| SinkError::new("total emission byte count overflow"))?;
        let exceeded = item_limit
            .filter(|limit| next_items > *limit)
            .map(|_| item_kind)
            .or_else(|| {
                byte_limit
                    .filter(|limit| next_bytes > *limit)
                    .map(|_| byte_kind)
            })
            .or_else(|| {
                self.request
                    .budgets
                    .max_total_logical_bytes
                    .filter(|limit| next_total > *limit)
                    .map(|_| BudgetKind::TotalLogicalBytes)
            });
        let Some(exceeded) = exceeded else {
            return Ok(true);
        };

        self.exceeded = Some(exceeded);
        let message = format!("emission budget exceeded: {exceeded:?}");
        match self.request.overflow {
            OverflowPolicy::Fail => {
                self.mark_failure(Some(channel), message.clone());
                Err(SinkError::new(message))
            }
            OverflowPolicy::AllowIncomplete => {
                self.mark_status(channel, ChannelStatus::Truncated);
                Ok(false)
            }
        }
    }

    fn commit(&mut self, channel: EmissionChannel, item_count: u64, logical_bytes: u64) {
        let target = match channel {
            EmissionChannel::NormalizedEvents => &mut self.normalized_events,
            EmissionChannel::Capabilities => &mut self.capabilities,
            EmissionChannel::NativeEvidence => &mut self.native_evidence,
            EmissionChannel::Frames => &mut self.frames,
            EmissionChannel::Snapshots => &mut self.snapshots,
            EmissionChannel::InputValueEvidence => &mut self.input_value_evidence,
            EmissionChannel::Envelope => &mut self.envelope,
        };
        target.item_count += item_count;
        target.logical_bytes += logical_bytes;
        self.total_logical_bytes += logical_bytes;
    }

    fn mark_status(&mut self, channel: EmissionChannel, status: ChannelStatus) {
        let target = match channel {
            EmissionChannel::NormalizedEvents => &mut self.normalized_events,
            EmissionChannel::Capabilities => &mut self.capabilities,
            EmissionChannel::NativeEvidence => &mut self.native_evidence,
            EmissionChannel::Frames => &mut self.frames,
            EmissionChannel::Snapshots => &mut self.snapshots,
            EmissionChannel::InputValueEvidence => &mut self.input_value_evidence,
            EmissionChannel::Envelope => &mut self.envelope,
        };
        target.status = Some(status);
    }

    fn mark_failure(&mut self, channel: Option<EmissionChannel>, message: String) {
        if let Some(channel) = channel {
            self.mark_status(channel, ChannelStatus::Failed);
        }
        self.failure = Some(EmissionFailure { channel, message });
    }

    fn measure<T: serde::Serialize>(value: &T) -> Result<u64, SinkError> {
        let bytes = canonical_json_bytes(value).map_err(SinkError::new)?;
        u64::try_from(bytes.len()).map_err(|_| SinkError::new("logical byte count overflow"))
    }
}

impl<S: EmissionSink> EmissionSink for BudgetedSink<S> {
    fn emit(&mut self, emission: Emission<'_>) -> Result<(), SinkError> {
        let (channel, item_count, logical_bytes) = match &emission {
            Emission::Event(event) => (
                Some(EmissionChannel::NormalizedEvents),
                1,
                Self::measure(*event)?,
            ),
            Emission::NativeEvidence(evidence) => (
                Some(EmissionChannel::NativeEvidence),
                1,
                Self::measure(*evidence)?,
            ),
            Emission::InputValueEvidence(evidence) => (
                Some(EmissionChannel::InputValueEvidence),
                1,
                Self::measure(*evidence)?,
            ),
            Emission::Frame(frame) => (Some(EmissionChannel::Frames), 1, Self::measure(*frame)?),
            Emission::Snapshot(snapshot) => (
                Some(EmissionChannel::Snapshots),
                1,
                u64::try_from(snapshot.bytes.len())
                    .map_err(|_| SinkError::new("snapshot byte count overflow"))?,
            ),
            Emission::RunStarted(_) | Emission::RunFinished(_) => {
                (Some(EmissionChannel::Envelope), 1, 0)
            }
            Emission::EvidenceReceipt(_) => (None, 0, 0),
        };

        if let Some(channel) = channel
            && !self.account_plan(channel, item_count, logical_bytes)?
        {
            return Ok(());
        }
        if let Err(error) = self.inner.emit(emission) {
            self.mark_failure(channel, error.to_string());
            return Err(error);
        }
        if let Some(channel) = channel {
            self.commit(channel, item_count, logical_bytes);
        }
        Ok(())
    }

    fn record_capability_receipts(&mut self, receipts: &[CapabilityReceipt]) {
        self.capability_receipts.extend_from_slice(receipts);
        self.inner.record_capability_receipts(receipts);
    }

    fn record_capability_outputs(&mut self, outputs: &[CapabilityOutput]) {
        for output in outputs {
            let logical_bytes = match Self::measure(output) {
                Ok(bytes) => bytes,
                Err(error) => {
                    self.mark_failure(Some(EmissionChannel::Capabilities), error.message().into());
                    continue;
                }
            };
            match self.account_plan(EmissionChannel::Capabilities, 1, logical_bytes) {
                Ok(true) => {
                    self.inner
                        .record_capability_outputs(std::slice::from_ref(output));
                    self.commit(EmissionChannel::Capabilities, 1, logical_bytes);
                }
                Ok(false) | Err(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BudgetedSink, ChannelStatus, EventContext, EventKind, EventSelection, EvidenceStatus,
        InputId, InputSource, InputValueEvidence, MachineId, ObservationRequest, OverflowPolicy,
        SchemaRef, SchemaVersion, StructuredValue, TypedInputPayload, VersionStamp,
    };

    fn event(sequence: u64) -> ExecutionEvent {
        ExecutionEvent::from_context(
            &EventContext {
                arch: MachineId::from("test"),
                machine_version: VersionStamp::from("test.v1"),
                run_id: RunId::from("run"),
                sequence,
                step: sequence,
                cycle_or_tick: Some(sequence),
                frame: None,
                pc: None,
                instruction: None,
            },
            EventKind::StepStarted,
        )
    }

    #[derive(Debug, Default)]
    struct RejectReceipt;

    impl EmissionSink for RejectReceipt {
        fn emit(&mut self, emission: Emission<'_>) -> Result<(), SinkError> {
            if matches!(emission, Emission::EvidenceReceipt(_)) {
                Err(SinkError::new("receipt publication rejected"))
            } else {
                Ok(())
            }
        }
    }

    #[derive(Debug, Default)]
    struct CapabilityOutputSink {
        outputs: usize,
    }

    impl EmissionSink for CapabilityOutputSink {
        fn emit(&mut self, _emission: Emission<'_>) -> Result<(), SinkError> {
            Ok(())
        }

        fn record_capability_outputs(&mut self, outputs: &[CapabilityOutput]) {
            self.outputs += outputs.len();
        }
    }

    #[test]
    fn budget_fail_produces_failed_receipt_without_forwarding_overflow() {
        let mut request = ObservationRequest::summary();
        request.normalized_events.events = EventSelection::All;
        request.budgets.max_normalized_events = Some(1);
        let mut sink = BudgetedSink::new(
            NullSink,
            request,
            RunId::from("run"),
            MachineId::from("test"),
            VersionStamp::from("test.v1"),
        )
        .expect("budget sink");
        sink.emit(Emission::Event(&event(0))).expect("first event");
        let error = sink
            .emit(Emission::Event(&event(1)))
            .expect_err("second event exceeds budget");
        assert!(error.message().contains("NormalizedEvents"));

        let receipt = sink.receipt();
        assert_eq!(receipt.status, EvidenceStatus::Failed);
        assert_eq!(receipt.fulfilled_guarantee, None);
        assert_eq!(receipt.budgets.exceeded, Some(BudgetKind::NormalizedEvents));
        assert_eq!(
            receipt.channels[0].status,
            ChannelStatus::Failed,
            "normalized channel records the failed overflow"
        );
    }

    #[test]
    fn frame_budget_is_accounted_independently_from_normalized_events() {
        let mut request = ObservationRequest::fitness();
        request.frames.capture = crate::FrameCapture::Full;
        request.budgets.max_frame_bytes = Some(8);
        request.overflow = OverflowPolicy::AllowIncomplete;
        let run_id = RunId::from("frame-budget");
        let machine_id = MachineId::from("test-machine");
        let machine_version = VersionStamp::from("v1");
        let frame = FrameArtifact::full(
            run_id.clone(),
            machine_id.clone(),
            machine_version.clone(),
            SchemaRef::new("test.frame", SchemaVersion::V1),
            7,
            11,
            3,
            vec![1; 32],
        );
        let mut sink = BudgetedSink::new(NullSink, request, run_id, machine_id, machine_version)
            .expect("budget sink");

        sink.emit(Emission::Frame(&frame))
            .expect("incomplete frame is dropped");
        let receipt = sink.receipt();
        let frames = receipt
            .channels
            .iter()
            .find(|channel| channel.channel == EmissionChannel::Frames)
            .expect("frame channel");
        assert_eq!(frames.status, ChannelStatus::Truncated);
        assert_eq!(frames.item_count, 0);
        assert_eq!(receipt.budgets.exceeded, Some(BudgetKind::FrameBytes));
    }

    #[test]
    fn input_value_budget_is_accounted_on_its_own_channel() {
        let mut request = ObservationRequest::summary();
        request.input_value_evidence.enabled = true;
        request.budgets.max_input_value_events = Some(1);
        let run_id = RunId::from("input-value-budget");
        let machine_id = MachineId::from("test-machine");
        let machine_version = VersionStamp::from("v1");
        let payload = TypedInputPayload::new(
            SchemaRef::new("test.input.payload", SchemaVersion::V1),
            StructuredValue::Unsigned(7),
        )
        .unwrap();
        let evidence = InputValueEvidence::new(
            InputId::new("chip8.key.0").unwrap(),
            payload.schema.clone(),
            InputSource::Scheduled { ordinal: 0 },
            crate::InputCoordinate::frame(0),
            payload,
        );
        let mut sink = BudgetedSink::new(NullSink, request, run_id, machine_id, machine_version)
            .expect("budget sink");
        sink.emit(Emission::InputValueEvidence(&evidence))
            .expect("first input value");
        let receipt = sink.receipt();
        let channel = receipt
            .channels
            .iter()
            .find(|channel| channel.channel == EmissionChannel::InputValueEvidence)
            .expect("input-value channel");
        assert_eq!(channel.status, ChannelStatus::Complete);
        assert_eq!(channel.item_count, 1);
    }

    #[test]
    fn allow_incomplete_skips_overflow_and_emits_receipt() {
        let mut request = ObservationRequest::summary();
        request.normalized_events.events = EventSelection::All;
        request.budgets.max_normalized_events = Some(1);
        request.overflow = OverflowPolicy::AllowIncomplete;
        let mut sink = BudgetedSink::new(
            NullSink,
            request,
            RunId::from("run"),
            MachineId::from("test"),
            VersionStamp::from("test.v1"),
        )
        .expect("budget sink");
        sink.emit(Emission::Event(&event(0))).expect("first event");
        sink.emit(Emission::Event(&event(1)))
            .expect("overflow is explicitly incomplete");
        let receipt = sink.emit_receipt().expect("receipt");
        assert_eq!(receipt.status, EvidenceStatus::Incomplete);
        assert_eq!(receipt.fulfilled_guarantee, None);
        assert_eq!(
            receipt.channels[0].status,
            ChannelStatus::Truncated,
            "normalized channel records allowed truncation"
        );
        assert_eq!(
            receipt.budgets.observed_logical_bytes,
            receipt.channels[0].logical_bytes
        );
    }

    #[test]
    fn published_receipt_preserves_capabilities_and_rejects_duplicates() {
        let mut sink = BudgetedSink::new(
            NullSink,
            ObservationRequest::summary(),
            RunId::from("run"),
            MachineId::from("test"),
            VersionStamp::from("test.v1"),
        )
        .expect("budget sink");
        let capability = glassvm_normalizer_contract::CapabilityReceipt {
            id: glassvm_normalizer_contract::CapabilityId::new("test.capability.v1")
                .expect("capability"),
            status: glassvm_normalizer_contract::CapabilityStatus::Fulfilled,
            output_schema: None,
            logical_bytes: 7,
            message: None,
        };
        sink.record_capability_receipts(std::slice::from_ref(&capability));

        let receipt = sink.emit_receipt().expect("receipt");
        assert_eq!(receipt.capabilities, vec![capability]);
        let duplicate = sink
            .emit_receipt()
            .expect_err("receipt publication must be exactly once");
        assert!(duplicate.message().contains("already emitted"));
    }

    #[test]
    fn capability_output_bytes_are_hard_budgeted_before_forwarding() {
        let mut request = ObservationRequest::summary();
        request
            .capabilities
            .push(glassvm_normalizer_contract::CapabilityRequest::required(
                glassvm_normalizer_contract::CapabilityId::new("test.capability")
                    .expect("capability"),
            ));
        request.budgets.max_capability_bytes = Some(1);
        request.overflow = OverflowPolicy::AllowIncomplete;
        let mut sink = BudgetedSink::new(
            CapabilityOutputSink::default(),
            request,
            RunId::from("run"),
            MachineId::from("test"),
            VersionStamp::from("test.v1"),
        )
        .expect("budget sink");
        let output = CapabilityOutput {
            schema: glassvm_normalizer_contract::CapabilityId::new("test.capability")
                .expect("capability")
                .schema(),
            value: serde_json::json!({"value": 7}),
        };

        sink.record_capability_outputs(std::slice::from_ref(&output));
        let receipt = sink.receipt();
        let capabilities = receipt
            .channels
            .iter()
            .find(|channel| channel.channel == EmissionChannel::Capabilities)
            .expect("capability channel");
        assert_eq!(capabilities.status, ChannelStatus::Truncated);
        assert_eq!(capabilities.item_count, 0);
        assert_eq!(sink.into_inner().outputs, 0);
    }

    #[test]
    fn receipt_publication_failure_is_reflected_in_sink_accounting() {
        let mut sink = BudgetedSink::new(
            RejectReceipt,
            ObservationRequest::summary(),
            RunId::from("run"),
            MachineId::from("test"),
            VersionStamp::from("test.v1"),
        )
        .expect("budget sink");

        let error = sink
            .emit_receipt()
            .expect_err("downstream receipt publication must fail");
        assert!(error.message().contains("publication rejected"));
        let receipt = sink.receipt();
        assert_eq!(receipt.status, EvidenceStatus::Failed);
        assert!(receipt.failure.is_some());
    }

    #[test]
    fn snapshot_envelope_detects_length_payload_and_schema_tampering() {
        let snapshot = SnapshotArtifact::new_typed(
            RunId::from("snapshot-run"),
            MachineId::from("test"),
            VersionStamp::from("test.v1"),
            SchemaRef::new("test.state", SchemaVersion::V1),
            4,
            3,
            vec![1, 2, 3, 4],
        );
        snapshot.validate_integrity().expect("valid snapshot");

        let mut wrong_length = snapshot.clone();
        wrong_length.byte_len += 1;
        assert!(
            wrong_length
                .validate_integrity()
                .expect_err("length mismatch")
                .contains("declares")
        );

        let mut wrong_payload = snapshot.clone();
        wrong_payload.bytes[0] ^= 0xff;
        assert!(
            wrong_payload
                .validate_integrity()
                .expect_err("digest mismatch")
                .contains("digest mismatch")
        );

        let mut wrong_schema = snapshot;
        wrong_schema.schema.version = SchemaVersion::new(2, 0, 0);
        assert!(
            wrong_schema
                .validate_integrity()
                .expect_err("schema mismatch")
                .contains("unsupported snapshot envelope")
        );
    }
}
