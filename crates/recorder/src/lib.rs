//! Versioned reference-recorder format types.
//!
//! This crate defines the bounded, channel-separated durable-format contract
//! and its reference recorder implementations. File persistence remains
//! intentionally limited to the versioned segment format; it does not own a
//! retention policy or Refinery storage policy.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use glassvm_core::{
    BudgetedSink, CapabilityId, CapabilityOutput, CapabilitySchema, ContentDigest,
    EMISSION_SCHEMA_VERSION, Emission, EmissionSink, EventSelection, EvidenceReceipt,
    EvidenceStatus, ExecutionEvent, FrameArtifact, FrameCapture, InputValueEvidence, MachineId,
    NativeEvidenceEnvelope, ObservationRequest, RunId, SchemaRef, SchemaVersion, SinkError,
    SnapshotArtifact, SnapshotCapture, VersionStamp, canonical_json_bytes,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const REFERENCE_RUN_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);
pub const REFERENCE_SEGMENT_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);
pub const RECORDER_RECEIPT_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 1, 0);
pub const SEGMENT_FORMAT_VERSION: u8 = 1;
const SEGMENT_MAGIC: &[u8; 4] = b"GVS1";
const BLOCK_MAGIC: &[u8; 4] = b"BLK1";
const FOOTER_MAGIC: &[u8; 4] = b"GVF1";
static STAGING_TOKEN: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceChannel {
    NormalizedEvents,
    NativeEvidence,
    InputValueEvidence,
    Frames,
    Snapshots,
    CapabilityOutputs,
}

/// Bounded aggregate accounting for one persisted evidence channel.
///
/// `logical_bytes` counts canonical-JSON record sizes. `encoded_bytes` counts
/// encoded block payload bytes, excluding segment/block framing and metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ChannelRecorderStats {
    pub segment_count: u64,
    pub record_count: u64,
    pub block_count: u64,
    pub logical_bytes: u64,
    pub encoded_bytes: u64,
}

impl ChannelRecorderStats {
    fn checked_add_footer(&self, footer: &SegmentFooter) -> Result<Self, RecorderError> {
        let checked = |current: u64, increment: u64, label: &str| {
            current
                .checked_add(increment)
                .ok_or_else(|| RecorderError::new(format!("channel {label} counter overflowed")))
        };
        Ok(Self {
            segment_count: checked(self.segment_count, 1, "segment-count")?,
            record_count: checked(
                self.record_count,
                footer.header.record_count,
                "record-count",
            )?,
            block_count: checked(self.block_count, footer.block_count, "block-count")?,
            logical_bytes: checked(
                self.logical_bytes,
                footer.header.logical_bytes,
                "logical-byte",
            )?,
            encoded_bytes: checked(self.encoded_bytes, footer.encoded_bytes, "encoded-byte")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecordedRecord {
    NormalizedEvent(ExecutionEvent),
    NativeEvidence(NativeEvidenceEnvelope),
    InputValueEvidence(InputValueEvidence),
    Frame(FrameArtifact),
    Snapshot(SnapshotArtifact),
    CapabilityOutput(CapabilityOutputRecord),
}

impl RecordedRecord {
    pub fn channel(&self) -> ReferenceChannel {
        match self {
            Self::NormalizedEvent(_) => ReferenceChannel::NormalizedEvents,
            Self::NativeEvidence(_) => ReferenceChannel::NativeEvidence,
            Self::InputValueEvidence(_) => ReferenceChannel::InputValueEvidence,
            Self::Frame(_) => ReferenceChannel::Frames,
            Self::Snapshot(_) => ReferenceChannel::Snapshots,
            Self::CapabilityOutput(_) => ReferenceChannel::CapabilityOutputs,
        }
    }

    pub fn sequence(&self) -> Option<u64> {
        match self {
            Self::NormalizedEvent(event) => Some(event.sequence),
            Self::NativeEvidence(evidence) => Some(evidence.sequence),
            Self::InputValueEvidence(_) => None,
            Self::Frame(frame) => Some(frame.sequence),
            Self::Snapshot(snapshot) => Some(snapshot.sequence),
            Self::CapabilityOutput(_) => None,
        }
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Self::NormalizedEvent(_) => Ok(()),
            Self::NativeEvidence(evidence) => evidence.validate(),
            Self::InputValueEvidence(evidence) => evidence.validate(),
            Self::Frame(frame) => frame.validate(),
            Self::Snapshot(snapshot) => snapshot.validate_integrity(),
            Self::CapabilityOutput(output) => output.validate(),
        }
    }

    fn logical_bytes(&self) -> Result<u64, String> {
        Ok(canonical_json_bytes(self)?.len() as u64)
    }
}

#[derive(Debug)]
pub struct FileSegmentWriter {
    writer: BufWriter<File>,
    prelude: SegmentPrelude,
    block: Vec<u8>,
    block_logical_bytes: u64,
    record_count: u64,
    logical_bytes: u64,
    first_sequence: Option<u64>,
    last_sequence: Option<u64>,
    block_count: u64,
    encoded_bytes: u64,
}

impl FileSegmentWriter {
    pub fn create(
        path: impl AsRef<Path>,
        channel: ReferenceChannel,
        segment_index: u64,
        limits: RecorderLimits,
    ) -> Result<Self, RecorderError> {
        limits.validate().map_err(RecorderError::new)?;
        let prelude = SegmentPrelude::new(channel, segment_index, limits);
        let file = File::create(path).map_err(io_error)?;
        let mut writer = BufWriter::new(file);
        writer.write_all(SEGMENT_MAGIC).map_err(io_error)?;
        write_cbor_frame(&mut writer, &prelude)?;
        Ok(Self {
            writer,
            prelude,
            block: Vec::new(),
            block_logical_bytes: 0,
            record_count: 0,
            logical_bytes: 0,
            first_sequence: None,
            last_sequence: None,
            block_count: 0,
            encoded_bytes: 0,
        })
    }

    pub fn prelude(&self) -> &SegmentPrelude {
        &self.prelude
    }

    fn would_exceed_segment(&self, record: &RecordedRecord) -> Result<bool, RecorderError> {
        if record.channel() != self.prelude.channel {
            return Err(RecorderError::new(
                "record channel does not match the segment channel",
            ));
        }
        record.validate().map_err(RecorderError::new)?;
        let logical_bytes = record.logical_bytes().map_err(RecorderError::new)?;
        if logical_bytes > self.prelude.limits.max_block_logical_bytes {
            return Err(RecorderError::new(format!(
                "record logical size {logical_bytes} exceeds block hard limit {}",
                self.prelude.limits.max_block_logical_bytes
            )));
        }
        Ok(self.record_count >= self.prelude.limits.max_segment_records
            || self
                .logical_bytes
                .checked_add(logical_bytes)
                .is_none_or(|bytes| bytes > self.prelude.limits.max_segment_logical_bytes))
    }

    pub fn record(&mut self, record: RecordedRecord) -> Result<(), RecorderError> {
        let logical_bytes = record.logical_bytes().map_err(RecorderError::new)?;
        if self.would_exceed_segment(&record)? {
            return Err(RecorderError::new("segment hard limit reached"));
        }
        let next_segment_bytes = self
            .logical_bytes
            .checked_add(logical_bytes)
            .ok_or_else(|| RecorderError::new("segment logical-byte count overflowed"))?;
        if next_segment_bytes > self.prelude.limits.max_segment_logical_bytes {
            return Err(RecorderError::new("segment logical-byte limit reached"));
        }
        let next_block_bytes = self
            .block_logical_bytes
            .checked_add(logical_bytes)
            .ok_or_else(|| RecorderError::new("block logical-byte count overflowed"))?;
        if self.block_logical_bytes > 0
            && next_block_bytes > self.prelude.limits.max_block_logical_bytes
        {
            self.flush_block()?;
        }

        let encoded = serde_cbor::to_vec(&record).map_err(codec_error)?;
        let encoded_len = u32::try_from(encoded.len())
            .map_err(|_| RecorderError::new("encoded record exceeds u32 framing limit"))?;
        self.block.extend_from_slice(&encoded_len.to_le_bytes());
        self.block.extend_from_slice(&encoded);
        self.block_logical_bytes = if self.block_logical_bytes == 0 {
            logical_bytes
        } else {
            next_block_bytes
        };
        self.record_count += 1;
        self.logical_bytes = next_segment_bytes;
        if let Some(sequence) = record.sequence() {
            self.first_sequence.get_or_insert(sequence);
            self.last_sequence = Some(sequence);
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<SegmentFooter, RecorderError> {
        self.flush_block()?;
        let header = SegmentHeader {
            schema: SchemaRef::new(
                "glassvm.reference_segment",
                REFERENCE_SEGMENT_SCHEMA_VERSION,
            ),
            channel: self.prelude.channel.clone(),
            segment_index: self.prelude.segment_index,
            record_count: self.record_count,
            first_sequence: self.first_sequence,
            last_sequence: self.last_sequence,
            logical_bytes: self.logical_bytes,
        };
        let footer = SegmentFooter {
            header,
            block_count: self.block_count,
            encoded_bytes: self.encoded_bytes,
            completed: true,
        };
        footer
            .validate(&self.prelude.limits)
            .map_err(RecorderError::new)?;
        self.writer.write_all(FOOTER_MAGIC).map_err(io_error)?;
        write_cbor_frame(&mut self.writer, &footer)?;
        self.writer.flush().map_err(io_error)?;
        self.writer
            .into_inner()
            .map_err(|error| io_error(error.into_error()))?
            .sync_all()
            .map_err(io_error)?;
        Ok(footer)
    }

    fn flush_block(&mut self) -> Result<(), RecorderError> {
        if self.block.is_empty() {
            return Ok(());
        }
        self.writer.write_all(BLOCK_MAGIC).map_err(io_error)?;
        self.writer
            .write_all(&self.block_logical_bytes.to_le_bytes())
            .map_err(io_error)?;
        let encoded_bytes = u64::try_from(self.block.len())
            .map_err(|_| RecorderError::new("encoded block size overflowed"))?;
        self.writer
            .write_all(&encoded_bytes.to_le_bytes())
            .map_err(io_error)?;
        self.writer.write_all(&self.block).map_err(io_error)?;
        self.encoded_bytes = self
            .encoded_bytes
            .checked_add(encoded_bytes)
            .ok_or_else(|| RecorderError::new("encoded byte count overflowed"))?;
        self.block_count += 1;
        self.block.clear();
        self.block_logical_bytes = 0;
        Ok(())
    }
}

#[derive(Debug)]
struct ReadBlock {
    cursor: Cursor<Vec<u8>>,
    logical_length: u64,
    logical_bytes: u64,
}

#[derive(Debug)]
pub struct FileSegmentReader {
    reader: BufReader<File>,
    prelude: SegmentPrelude,
    block: Option<ReadBlock>,
    footer: Option<SegmentFooter>,
    record_count: u64,
    logical_bytes: u64,
    first_sequence: Option<u64>,
    last_sequence: Option<u64>,
    block_count: u64,
}

impl FileSegmentReader {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, RecorderError> {
        let file = File::open(path).map_err(io_error)?;
        let mut reader = BufReader::new(file);
        let mut marker = [0; 4];
        reader.read_exact(&mut marker).map_err(io_error)?;
        if &marker != SEGMENT_MAGIC {
            return Err(RecorderError::new("invalid reference-segment magic"));
        }
        let prelude: SegmentPrelude = read_cbor_frame(&mut reader)?;
        prelude.validate().map_err(RecorderError::new)?;
        Ok(Self {
            reader,
            prelude,
            block: None,
            footer: None,
            record_count: 0,
            logical_bytes: 0,
            first_sequence: None,
            last_sequence: None,
            block_count: 0,
        })
    }

    pub fn prelude(&self) -> &SegmentPrelude {
        &self.prelude
    }

    pub fn footer(&self) -> Option<&SegmentFooter> {
        self.footer.as_ref()
    }

    pub fn next_record(&mut self) -> Result<Option<RecordedRecord>, RecorderError> {
        if self.footer.is_some() {
            return Ok(None);
        }
        if let Some(block) = &mut self.block {
            if block.cursor.position() < block.cursor.get_ref().len() as u64 {
                let record = read_record_from_block(block)?;
                if record.channel() != self.prelude.channel {
                    return Err(RecorderError::new(
                        "record channel does not match the segment prelude",
                    ));
                }
                let logical_bytes = record.logical_bytes().map_err(RecorderError::new)?;
                block.logical_bytes = block
                    .logical_bytes
                    .checked_add(logical_bytes)
                    .ok_or_else(|| RecorderError::new("block logical-byte count overflowed"))?;
                if block.logical_bytes > block.logical_length {
                    return Err(RecorderError::new(
                        "block records exceed declared logical bytes",
                    ));
                }
                self.record_count = self
                    .record_count
                    .checked_add(1)
                    .ok_or_else(|| RecorderError::new("segment record count overflowed"))?;
                if self.record_count > self.prelude.limits.max_segment_records {
                    return Err(RecorderError::new(
                        "segment records exceed the hard record limit",
                    ));
                }
                self.logical_bytes = self
                    .logical_bytes
                    .checked_add(logical_bytes)
                    .ok_or_else(|| RecorderError::new("segment logical-byte count overflowed"))?;
                if self.logical_bytes > self.prelude.limits.max_segment_logical_bytes {
                    return Err(RecorderError::new(
                        "segment records exceed the hard logical-byte limit",
                    ));
                }
                if let Some(sequence) = record.sequence() {
                    self.first_sequence.get_or_insert(sequence);
                    self.last_sequence = Some(sequence);
                }
                return Ok(Some(record));
            }
            let block = self.block.take().expect("block was present");
            if block.logical_bytes != block.logical_length {
                return Err(RecorderError::new(
                    "block records do not match declared logical bytes",
                ));
            }
            self.block_count += 1;
        }

        let mut marker = [0; 4];
        match self.reader.read_exact(&mut marker) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                return Err(RecorderError::new("segment ended without a footer"));
            }
            Err(error) => return Err(io_error(error)),
        }
        if &marker == FOOTER_MAGIC {
            let footer: SegmentFooter = read_cbor_frame(&mut self.reader)?;
            self.validate_footer(&footer)?;
            self.footer = Some(footer);
            return Ok(None);
        }
        if &marker != BLOCK_MAGIC {
            return Err(RecorderError::new("invalid segment block or footer marker"));
        }
        let logical_length = read_u64(&mut self.reader)?;
        let encoded_length = read_u64(&mut self.reader)?;
        if logical_length == 0 || logical_length > self.prelude.limits.max_block_logical_bytes {
            return Err(RecorderError::new(
                "block logical length exceeds the hard block limit",
            ));
        }
        let max_encoded = self
            .prelude
            .limits
            .max_segment_logical_bytes
            .saturating_add(self.prelude.limits.max_block_logical_bytes);
        if encoded_length == 0 || encoded_length > max_encoded {
            return Err(RecorderError::new(
                "block encoded length exceeds bounded reader allowance",
            ));
        }
        let encoded_length_usize = usize::try_from(encoded_length)
            .map_err(|_| RecorderError::new("block encoded length does not fit usize"))?;
        let mut bytes = vec![0; encoded_length_usize];
        self.reader.read_exact(&mut bytes).map_err(io_error)?;
        self.block = Some(ReadBlock {
            cursor: Cursor::new(bytes),
            logical_length,
            logical_bytes: 0,
        });
        self.next_record()
    }

    fn validate_footer(&self, footer: &SegmentFooter) -> Result<(), RecorderError> {
        footer
            .validate(&self.prelude.limits)
            .map_err(RecorderError::new)?;
        if footer.header.channel != self.prelude.channel
            || footer.header.segment_index != self.prelude.segment_index
        {
            return Err(RecorderError::new(
                "segment footer identity does not match its prelude",
            ));
        }
        if footer.header.record_count != self.record_count
            || footer.header.logical_bytes != self.logical_bytes
            || footer.header.first_sequence != self.first_sequence
            || footer.header.last_sequence != self.last_sequence
            || footer.block_count != self.block_count
        {
            return Err(RecorderError::new(
                "segment footer accounting does not match its records",
            ));
        }
        Ok(())
    }
}

fn write_cbor_frame<W: Write, T: Serialize>(
    writer: &mut W,
    value: &T,
) -> Result<(), RecorderError> {
    let encoded = serde_cbor::to_vec(value).map_err(codec_error)?;
    let length = u32::try_from(encoded.len())
        .map_err(|_| RecorderError::new("CBOR frame exceeds u32 framing limit"))?;
    writer.write_all(&length.to_le_bytes()).map_err(io_error)?;
    writer.write_all(&encoded).map_err(io_error)
}

fn read_cbor_frame<R: Read, T: for<'de> Deserialize<'de>>(
    reader: &mut R,
) -> Result<T, RecorderError> {
    let length = read_u32(reader)? as usize;
    if length > 16 * 1024 * 1024 {
        return Err(RecorderError::new(
            "CBOR frame exceeds bounded metadata limit",
        ));
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).map_err(io_error)?;
    serde_cbor::from_slice(&bytes).map_err(codec_error)
}

fn read_record_from_block(block: &mut ReadBlock) -> Result<RecordedRecord, RecorderError> {
    let remaining = block.cursor.get_ref().len() as u64 - block.cursor.position();
    if remaining < 4 {
        return Err(RecorderError::new(
            "block ended before a complete record length",
        ));
    }
    let record_length = read_u32(&mut block.cursor)? as u64;
    if record_length > remaining - 4 {
        return Err(RecorderError::new("record length exceeds block boundary"));
    }
    let mut bytes = vec![0; record_length as usize];
    block.cursor.read_exact(&mut bytes).map_err(io_error)?;
    let record: RecordedRecord = serde_cbor::from_slice(&bytes).map_err(codec_error)?;
    record.validate().map_err(RecorderError::new)?;
    Ok(record)
}

fn read_u32<R: Read>(reader: &mut R) -> Result<u32, RecorderError> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes).map_err(io_error)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64<R: Read>(reader: &mut R) -> Result<u64, RecorderError> {
    let mut bytes = [0; 8];
    reader.read_exact(&mut bytes).map_err(io_error)?;
    Ok(u64::from_le_bytes(bytes))
}

fn io_error(error: io::Error) -> RecorderError {
    RecorderError::new(format!("segment I/O failed: {error}"))
}

fn codec_error(error: impl fmt::Display) -> RecorderError {
    RecorderError::new(format!("segment codec failed: {error}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecorderError {
    message: String,
}

impl RecorderError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RecorderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(formatter)
    }
}

impl std::error::Error for RecorderError {}

fn validate_evidence_identity(
    receipt: &EvidenceReceipt,
    manifest: &ReferenceRunManifest,
) -> Result<(), String> {
    if receipt.schema_version != EMISSION_SCHEMA_VERSION {
        return Err(format!(
            "unsupported evidence-receipt schema version {}",
            receipt.schema_version
        ));
    }
    if receipt.run_id != manifest.run_id {
        return Err("evidence receipt run ID does not match the manifest".into());
    }
    if receipt.machine_id != manifest.machine_id {
        return Err("evidence receipt machine ID does not match the manifest".into());
    }
    if receipt.machine_version != manifest.machine_version {
        return Err("evidence receipt machine version does not match the manifest".into());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecorderLimits {
    /// Hard maximum logical bytes in one encoded data block.
    pub max_block_logical_bytes: u64,
    /// Hard maximum number of records in one segment.
    pub max_segment_records: u64,
    /// Hard maximum logical bytes buffered in one segment.
    pub max_segment_logical_bytes: u64,
    /// Hard maximum live logical bytes buffered for one active channel.
    pub max_buffered_bytes_per_channel: u64,
}

impl RecorderLimits {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_block_logical_bytes == 0 {
            return Err("recorder block byte limit must be non-zero".into());
        }
        if self.max_segment_records == 0 {
            return Err("recorder segment record limit must be non-zero".into());
        }
        if self.max_segment_logical_bytes == 0 {
            return Err("recorder segment byte limit must be non-zero".into());
        }
        if self.max_buffered_bytes_per_channel == 0 {
            return Err("recorder channel buffer limit must be non-zero".into());
        }
        if self.max_block_logical_bytes > self.max_segment_logical_bytes {
            return Err("recorder block limit cannot exceed the segment byte limit".into());
        }
        if self.max_block_logical_bytes > self.max_buffered_bytes_per_channel {
            return Err("recorder block limit cannot exceed the channel buffer limit".into());
        }
        if self.max_buffered_bytes_per_channel > self.max_segment_logical_bytes {
            return Err(
                "recorder channel buffer limit cannot exceed the segment byte limit".into(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceRunManifest {
    pub schema: SchemaRef,
    pub run_id: RunId,
    pub machine_id: MachineId,
    pub machine_version: VersionStamp,
    pub request: ObservationRequest,
    pub prepared_observation_id: Option<ContentDigest>,
    pub requested_channels: BTreeSet<ReferenceChannel>,
    pub limits: RecorderLimits,
}

impl ReferenceRunManifest {
    pub fn validate(&self) -> Result<(), String> {
        let expected = SchemaRef::new("glassvm.reference_run", REFERENCE_RUN_SCHEMA_VERSION);
        if self.schema != expected {
            return Err(format!(
                "unsupported reference-run schema {} {}",
                self.schema.id, self.schema.version
            ));
        }
        self.request
            .validate()
            .map_err(|error| format!("invalid observation request: {error}"))?;
        self.limits.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SegmentHeader {
    pub schema: SchemaRef,
    pub channel: ReferenceChannel,
    pub segment_index: u64,
    pub record_count: u64,
    pub first_sequence: Option<u64>,
    pub last_sequence: Option<u64>,
    pub logical_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentRecordEncoding {
    Cbor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentBlockCompression {
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SegmentPrelude {
    pub format_version: u8,
    pub schema: SchemaRef,
    pub channel: ReferenceChannel,
    pub segment_index: u64,
    pub record_encoding: SegmentRecordEncoding,
    pub block_compression: SegmentBlockCompression,
    pub limits: RecorderLimits,
}

impl SegmentPrelude {
    fn new(channel: ReferenceChannel, segment_index: u64, limits: RecorderLimits) -> Self {
        Self {
            format_version: SEGMENT_FORMAT_VERSION,
            schema: SchemaRef::new(
                "glassvm.reference_segment",
                REFERENCE_SEGMENT_SCHEMA_VERSION,
            ),
            channel,
            segment_index,
            record_encoding: SegmentRecordEncoding::Cbor,
            block_compression: SegmentBlockCompression::None,
            limits,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.format_version != SEGMENT_FORMAT_VERSION {
            return Err(format!(
                "unsupported segment format version {}",
                self.format_version
            ));
        }
        let expected = SchemaRef::new(
            "glassvm.reference_segment",
            REFERENCE_SEGMENT_SCHEMA_VERSION,
        );
        if self.schema != expected {
            return Err(format!(
                "unsupported reference-segment schema {} {}",
                self.schema.id, self.schema.version
            ));
        }
        self.limits.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SegmentFooter {
    pub header: SegmentHeader,
    pub block_count: u64,
    pub encoded_bytes: u64,
    pub completed: bool,
}

impl SegmentFooter {
    fn validate(&self, limits: &RecorderLimits) -> Result<(), String> {
        if !self.completed {
            return Err("segment footer is not marked completed".into());
        }
        self.header.validate(limits)
    }
}

impl SegmentHeader {
    pub fn validate(&self, limits: &RecorderLimits) -> Result<(), String> {
        limits.validate()?;
        let expected = SchemaRef::new(
            "glassvm.reference_segment",
            REFERENCE_SEGMENT_SCHEMA_VERSION,
        );
        if self.schema != expected {
            return Err(format!(
                "unsupported reference-segment schema {} {}",
                self.schema.id, self.schema.version
            ));
        }
        if self.record_count > limits.max_segment_records {
            return Err(format!(
                "segment contains {} records, exceeding hard limit {}",
                self.record_count, limits.max_segment_records
            ));
        }
        if self.logical_bytes > limits.max_segment_logical_bytes {
            return Err(format!(
                "segment contains {} logical bytes, exceeding hard limit {}",
                self.logical_bytes, limits.max_segment_logical_bytes
            ));
        }
        if self.first_sequence.is_some() != self.last_sequence.is_some() {
            return Err("segment sequence bounds must be both present or both absent".into());
        }
        if let (Some(first), Some(last)) = (self.first_sequence, self.last_sequence)
            && last < first
        {
            return Err("segment sequence bounds are reversed".into());
        }
        if self.record_count == 0 && (self.first_sequence.is_some() || self.last_sequence.is_some())
        {
            return Err("empty segment cannot have sequence bounds".into());
        }
        Ok(())
    }
}

/// Durable capability output. The canonical capability ID is mandatory and is
/// the only capability identity persisted by the reference format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityOutputRecord {
    pub capability_id: CapabilityId,
    pub output_schema: CapabilitySchema,
    pub payload: Value,
}

impl CapabilityOutputRecord {
    pub fn validate(&self) -> Result<(), String> {
        if self.output_schema.id != self.capability_id {
            return Err(format!(
                "capability output schema {} does not match canonical capability ID {}",
                self.output_schema.id.as_str(),
                self.capability_id.as_str()
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecorderStatus {
    Complete,
    Incomplete,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecorderFailure {
    pub channel: Option<ReferenceChannel>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecorderReceipt {
    pub schema: SchemaRef,
    pub status: RecorderStatus,
    pub finalized: bool,
    pub evidence_receipt_published: bool,
    /// Evidence fulfillment is reported independently from recorder status.
    pub evidence_status: EvidenceStatus,
    /// Per-channel recording totals, in canonical `ReferenceChannel` order.
    /// The vector is bounded by the fixed set of recorder channels.
    pub channels: Vec<RecorderChannelReceipt>,
    pub segment_count: u64,
    pub record_count: u64,
    pub block_count: u64,
    pub logical_bytes: u64,
    pub encoded_bytes: Option<u64>,
    pub file_count: u64,
    pub failure: Option<RecorderFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecorderChannelReceipt {
    pub channel: ReferenceChannel,
    pub stats: ChannelRecorderStats,
}

impl RecorderReceipt {
    pub fn validate(&self) -> Result<(), String> {
        let expected = SchemaRef::new("glassvm.recorder_receipt", RECORDER_RECEIPT_SCHEMA_VERSION);
        if self.schema != expected {
            return Err(format!(
                "unsupported recorder-receipt schema {} {}",
                self.schema.id, self.schema.version
            ));
        }
        if self.finalized && !self.evidence_receipt_published {
            return Err("finalized recorder receipt must publish an evidence receipt".into());
        }
        if self.status == RecorderStatus::Complete && self.failure.is_some() {
            return Err("complete recorder receipt cannot contain a recorder failure".into());
        }
        if self.status == RecorderStatus::Failed && self.failure.is_none() {
            return Err("failed recorder receipt must contain a recorder failure".into());
        }
        let mut previous_channel: Option<&ReferenceChannel> = None;
        let mut totals = ChannelRecorderStats::default();
        for entry in &self.channels {
            if previous_channel.is_some_and(|previous| previous >= &entry.channel) {
                return Err(
                    "recorder channel statistics must be unique and canonically ordered".into(),
                );
            }
            previous_channel = Some(&entry.channel);
            let stats = &entry.stats;
            if stats.segment_count == 0
                || stats.record_count == 0
                || stats.block_count == 0
                || stats.logical_bytes == 0
                || stats.encoded_bytes == 0
            {
                return Err("recorded channel statistics must contain non-zero totals".into());
            }
            totals = ChannelRecorderStats {
                segment_count: totals
                    .segment_count
                    .checked_add(stats.segment_count)
                    .ok_or("recorder segment-count total overflowed")?,
                record_count: totals
                    .record_count
                    .checked_add(stats.record_count)
                    .ok_or("recorder record-count total overflowed")?,
                block_count: totals
                    .block_count
                    .checked_add(stats.block_count)
                    .ok_or("recorder block-count total overflowed")?,
                logical_bytes: totals
                    .logical_bytes
                    .checked_add(stats.logical_bytes)
                    .ok_or("recorder logical-byte total overflowed")?,
                encoded_bytes: totals
                    .encoded_bytes
                    .checked_add(stats.encoded_bytes)
                    .ok_or("recorder encoded-byte total overflowed")?,
            };
        }
        if totals.segment_count != self.segment_count
            || totals.record_count != self.record_count
            || totals.block_count != self.block_count
            || totals.logical_bytes != self.logical_bytes
        {
            return Err("recorder channel totals do not match receipt totals".into());
        }
        if let Some(encoded_bytes) = self.encoded_bytes
            && totals.encoded_bytes != encoded_bytes
        {
            return Err("recorder channel encoded-byte totals do not match receipt".into());
        }
        if self.status == RecorderStatus::Complete {
            let expected_files = self
                .segment_count
                .checked_add(3)
                .ok_or("recorder file-count total overflowed")?;
            if self.encoded_bytes.is_none() || self.file_count != expected_files {
                return Err("complete recorder receipt has inconsistent file totals".into());
            }
        }
        Ok(())
    }

    /// This is the mechanical receipt condition required before an atomic
    /// rename from the temporary run directory to its published location.
    /// Evidence failure does not make recorder failure and evidence failure the
    /// same status: a recorder can persist a complete package whose evidence
    /// status is `Failed`.
    pub fn is_publishable(&self) -> bool {
        self.finalized
            && self.status == RecorderStatus::Complete
            && self.evidence_receipt_published
            && self.validate().is_ok()
    }
}

/// A non-resumable file-backed recorder for one staged reference run.
/// Dropping it leaves the staging directory intact for inspection or explicit
/// cleanup; version 0.1 never attempts to resume it.
#[derive(Debug)]
pub struct FileRunRecorder {
    published_path: PathBuf,
    staging_path: PathBuf,
    manifest: ReferenceRunManifest,
    active: BTreeMap<ReferenceChannel, FileSegmentWriter>,
    next_segment_index: BTreeMap<ReferenceChannel, u64>,
    channel_stats: BTreeMap<ReferenceChannel, ChannelRecorderStats>,
    evidence_receipt: Option<EvidenceReceipt>,
}

/// Live emission adapter for the staged file recorder. It prepares the
/// recorder exactly once at `RunStarted`, then writes each channel directly to
/// its bounded segment stream.
#[derive(Debug)]
pub struct FileEmissionSink {
    published_path: PathBuf,
    machine_version: VersionStamp,
    limits: RecorderLimits,
    recorder: Option<FileRunRecorder>,
    failure: Option<SinkError>,
}

#[derive(Debug)]
pub struct FileRunResult {
    pub execution: glassvm_core::RunResult,
    pub evidence: EvidenceReceipt,
    pub recorder: RecorderReceipt,
    pub published: PublishedFileRun,
}

pub struct FileRunSession;

impl FileRunSession {
    pub fn run(
        session: &mut dyn glassvm_core::EmulatorSession,
        request: &glassvm_core::ExecutionRequest,
        published_path: impl AsRef<Path>,
        machine_version: VersionStamp,
        limits: RecorderLimits,
    ) -> Result<FileRunResult, SinkError> {
        let observation = &request.observation;
        let mut sink = BudgetedSink::new(
            FileEmissionSink::new(&published_path, machine_version.clone(), limits),
            observation.clone(),
            request.run_id.clone(),
            request.machine_id.clone(),
            machine_version,
        )?;
        let execution = session
            .execute(&mut sink)
            .map_err(|error| SinkError::new(format!("machine execution failed: {error}")))?;
        let evidence = sink.emit_receipt()?;
        let file_sink = sink.into_inner();
        let published = file_sink.finish()?;
        let recorder = published.recorder_receipt().clone();
        Ok(FileRunResult {
            execution,
            evidence,
            recorder,
            published,
        })
    }
}

impl FileEmissionSink {
    pub fn new(
        published_path: impl AsRef<Path>,
        machine_version: VersionStamp,
        limits: RecorderLimits,
    ) -> Self {
        Self {
            published_path: published_path.as_ref().to_path_buf(),
            machine_version,
            limits,
            recorder: None,
            failure: None,
        }
    }

    pub fn finish(self) -> Result<PublishedFileRun, SinkError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        self.recorder
            .ok_or_else(|| SinkError::new("file sink did not receive RunStarted"))?
            .finalize()
            .map_err(|error| SinkError::new(error.to_string()))
    }

    fn start(&mut self, request: &glassvm_core::ExecutionRequest) -> Result<(), SinkError> {
        if self.recorder.is_some() {
            return Err(SinkError::new(
                "file sink received more than one RunStarted",
            ));
        }
        let observation = &request.observation;
        let mut requested_channels = BTreeSet::new();
        if !matches!(observation.normalized_events.events, EventSelection::None) {
            requested_channels.insert(ReferenceChannel::NormalizedEvents);
        }
        if observation.native_evidence.enabled {
            requested_channels.insert(ReferenceChannel::NativeEvidence);
        }
        if observation.input_value_evidence.enabled {
            requested_channels.insert(ReferenceChannel::InputValueEvidence);
        }
        if !matches!(observation.frames.capture, FrameCapture::None) {
            requested_channels.insert(ReferenceChannel::Frames);
        }
        if !matches!(observation.snapshots.capture, SnapshotCapture::None) {
            requested_channels.insert(ReferenceChannel::Snapshots);
        }
        if !observation.capabilities.is_empty() {
            requested_channels.insert(ReferenceChannel::CapabilityOutputs);
        }
        let manifest = ReferenceRunManifest {
            schema: SchemaRef::new("glassvm.reference_run", REFERENCE_RUN_SCHEMA_VERSION),
            run_id: request.run_id.clone(),
            machine_id: request.machine_id.clone(),
            machine_version: self.machine_version.clone(),
            request: observation.clone(),
            prepared_observation_id: request.prepared_observation_id,
            requested_channels,
            limits: self.limits.clone(),
        };
        self.recorder = Some(
            FileRunRecorder::create(&self.published_path, manifest)
                .map_err(|error| SinkError::new(error.to_string()))?,
        );
        Ok(())
    }

    fn recorder_mut(&mut self) -> Result<&mut FileRunRecorder, SinkError> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        self.recorder
            .as_mut()
            .ok_or_else(|| SinkError::new("file sink received an emission before RunStarted"))
    }
}

impl EmissionSink for FileEmissionSink {
    fn emit(&mut self, emission: Emission<'_>) -> Result<(), SinkError> {
        match emission {
            Emission::RunStarted(request) => self.start(request),
            Emission::Event(event) => self
                .recorder_mut()?
                .record(RecordedRecord::NormalizedEvent(event.clone()))
                .map_err(|error| SinkError::new(error.to_string())),
            Emission::NativeEvidence(evidence) => self
                .recorder_mut()?
                .record(RecordedRecord::NativeEvidence(evidence.clone()))
                .map_err(|error| SinkError::new(error.to_string())),
            Emission::InputValueEvidence(evidence) => self
                .recorder_mut()?
                .record(RecordedRecord::InputValueEvidence(evidence.clone()))
                .map_err(|error| SinkError::new(error.to_string())),
            Emission::Frame(frame) => self
                .recorder_mut()?
                .record(RecordedRecord::Frame(frame.clone()))
                .map_err(|error| SinkError::new(error.to_string())),
            Emission::Snapshot(snapshot) => self
                .recorder_mut()?
                .record(RecordedRecord::Snapshot(snapshot.clone()))
                .map_err(|error| SinkError::new(error.to_string())),
            Emission::EvidenceReceipt(receipt) => self
                .recorder_mut()?
                .publish_evidence_receipt(receipt.clone())
                .map_err(|error| SinkError::new(error.to_string())),
            Emission::RunFinished(_) => Ok(()),
        }
    }

    fn record_capability_outputs(&mut self, outputs: &[CapabilityOutput]) {
        for output in outputs {
            let record = RecordedRecord::CapabilityOutput(CapabilityOutputRecord {
                capability_id: output.schema.id.clone(),
                output_schema: output.schema.clone(),
                payload: output.value.clone(),
            });
            let _ = self
                .recorder_mut()
                .and_then(|recorder| {
                    recorder
                        .record(record)
                        .map_err(|error| SinkError::new(error.to_string()))
                })
                .map_err(|error| self.failure = Some(error));
        }
    }
}

impl FileRunRecorder {
    pub fn create(
        published_path: impl AsRef<Path>,
        manifest: ReferenceRunManifest,
    ) -> Result<Self, RecorderError> {
        manifest.validate().map_err(RecorderError::new)?;
        let published_path = published_path.as_ref().to_path_buf();
        if published_path.exists() {
            return Err(RecorderError::new("published run directory already exists"));
        }
        let parent = published_path
            .parent()
            .ok_or_else(|| RecorderError::new("published run path has no parent directory"))?;
        fs::create_dir_all(parent).map_err(io_error)?;
        let staging_root = parent.join(".staging");
        fs::create_dir_all(&staging_root).map_err(io_error)?;
        let staging_path = create_staging_directory(&staging_root)?;
        write_json_file(&staging_path.join("manifest.json"), &manifest)?;
        Ok(Self {
            published_path,
            staging_path,
            manifest,
            active: BTreeMap::new(),
            next_segment_index: BTreeMap::new(),
            channel_stats: BTreeMap::new(),
            evidence_receipt: None,
        })
    }

    pub fn manifest(&self) -> &ReferenceRunManifest {
        &self.manifest
    }

    pub fn staging_path(&self) -> &Path {
        &self.staging_path
    }

    pub fn record(&mut self, record: RecordedRecord) -> Result<(), RecorderError> {
        let channel = record.channel();
        if !self.manifest.requested_channels.contains(&channel) {
            return Err(RecorderError::new("recorded channel was not requested"));
        }
        let rotate = match self.active.get(&channel) {
            Some(writer) => writer.would_exceed_segment(&record)?,
            None => false,
        };
        if rotate {
            self.close_channel(&channel)?;
        }
        if !self.active.contains_key(&channel) {
            self.open_channel(channel.clone())?;
        }
        self.active
            .get_mut(&channel)
            .expect("opened channel has an active writer")
            .record(record)
    }

    pub fn publish_evidence_receipt(
        &mut self,
        receipt: EvidenceReceipt,
    ) -> Result<(), RecorderError> {
        validate_evidence_identity(&receipt, &self.manifest).map_err(RecorderError::new)?;
        if self.evidence_receipt.is_some() {
            return Err(RecorderError::new(
                "evidence receipt was published more than once",
            ));
        }
        write_json_file(&self.staging_path.join("evidence-receipt.json"), &receipt)?;
        self.evidence_receipt = Some(receipt);
        Ok(())
    }

    pub fn finalize(mut self) -> Result<PublishedFileRun, RecorderError> {
        let evidence_receipt = self.evidence_receipt.clone().ok_or_else(|| {
            RecorderError::new("cannot finalize recorder before publishing evidence receipt")
        })?;
        let channels: Vec<_> = self.active.keys().cloned().collect();
        for channel in channels {
            self.close_channel(&channel)?;
        }
        let channels: Vec<_> = self
            .channel_stats
            .iter()
            .map(|(channel, stats)| RecorderChannelReceipt {
                channel: channel.clone(),
                stats: stats.clone(),
            })
            .collect();
        let totals = channels.iter().try_fold(
            ChannelRecorderStats::default(),
            |totals, entry| -> Result<ChannelRecorderStats, RecorderError> {
                let add = |left: u64, right: u64, label: &str| {
                    left.checked_add(right)
                        .ok_or_else(|| RecorderError::new(format!("run {label} total overflowed")))
                };
                Ok(ChannelRecorderStats {
                    segment_count: add(
                        totals.segment_count,
                        entry.stats.segment_count,
                        "segment-count",
                    )?,
                    record_count: add(
                        totals.record_count,
                        entry.stats.record_count,
                        "record-count",
                    )?,
                    block_count: add(totals.block_count, entry.stats.block_count, "block-count")?,
                    logical_bytes: add(
                        totals.logical_bytes,
                        entry.stats.logical_bytes,
                        "logical-byte",
                    )?,
                    encoded_bytes: add(
                        totals.encoded_bytes,
                        entry.stats.encoded_bytes,
                        "encoded-byte",
                    )?,
                })
            },
        )?;
        let file_count = totals
            .segment_count
            .checked_add(3)
            .ok_or_else(|| RecorderError::new("run file-count total overflowed"))?;
        let receipt = RecorderReceipt {
            schema: SchemaRef::new("glassvm.recorder_receipt", RECORDER_RECEIPT_SCHEMA_VERSION),
            status: RecorderStatus::Complete,
            finalized: true,
            evidence_receipt_published: true,
            evidence_status: evidence_receipt.status,
            channels,
            segment_count: totals.segment_count,
            record_count: totals.record_count,
            block_count: totals.block_count,
            logical_bytes: totals.logical_bytes,
            encoded_bytes: Some(totals.encoded_bytes),
            file_count,
            failure: None,
        };
        receipt.validate().map_err(RecorderError::new)?;
        write_json_file(&self.staging_path.join("recorder-receipt.json"), &receipt)?;
        sync_directory(&self.staging_path)?;
        fs::rename(&self.staging_path, &self.published_path).map_err(io_error)?;
        let parent = self
            .published_path
            .parent()
            .ok_or_else(|| RecorderError::new("published run path has no parent directory"))?;
        sync_directory(parent)?;
        PublishedFileRun::open(&self.published_path)
    }

    fn open_channel(&mut self, channel: ReferenceChannel) -> Result<(), RecorderError> {
        let index = *self.next_segment_index.entry(channel.clone()).or_insert(0);
        let directory = self.staging_path.join(channel_directory_name(&channel));
        fs::create_dir_all(&directory).map_err(io_error)?;
        let path = directory.join(segment_file_name(index));
        let writer =
            FileSegmentWriter::create(&path, channel.clone(), index, self.manifest.limits.clone())?;
        self.active.insert(channel, writer);
        Ok(())
    }

    fn close_channel(&mut self, channel: &ReferenceChannel) -> Result<(), RecorderError> {
        let Some(writer) = self.active.remove(channel) else {
            return Ok(());
        };
        let footer = writer.finish()?;
        let next_index = footer
            .header
            .segment_index
            .checked_add(1)
            .ok_or_else(|| RecorderError::new("channel segment index overflowed"))?;
        self.next_segment_index.insert(channel.clone(), next_index);
        let next_stats = self
            .channel_stats
            .get(channel)
            .cloned()
            .unwrap_or_default()
            .checked_add_footer(&footer)?;
        self.channel_stats.insert(channel.clone(), next_stats);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PublishedFileRun {
    root: PathBuf,
    manifest: ReferenceRunManifest,
    evidence_receipt: EvidenceReceipt,
    recorder_receipt: RecorderReceipt,
}

impl PublishedFileRun {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, RecorderError> {
        let root = path.as_ref().to_path_buf();
        let manifest: ReferenceRunManifest = read_json_file(&root.join("manifest.json"))?;
        manifest.validate().map_err(RecorderError::new)?;
        let evidence_receipt: EvidenceReceipt =
            read_json_file(&root.join("evidence-receipt.json"))?;
        validate_evidence_identity(&evidence_receipt, &manifest).map_err(RecorderError::new)?;
        let recorder_receipt: RecorderReceipt =
            read_json_file(&root.join("recorder-receipt.json"))?;
        if !recorder_receipt.is_publishable() {
            return Err(RecorderError::new(
                "run is not finalized with a publishable recorder receipt",
            ));
        }
        if recorder_receipt.evidence_status != evidence_receipt.status {
            return Err(RecorderError::new(
                "recorder receipt evidence status does not match evidence receipt",
            ));
        }
        let mut segment_count = 0u64;
        for channel in ReferenceChannel::all() {
            let actual_count = count_segment_files(&root, &channel)?;
            let recorded_count = recorder_receipt
                .channels
                .iter()
                .find(|entry| entry.channel == channel)
                .map_or(0, |entry| entry.stats.segment_count);
            if actual_count != recorded_count {
                return Err(RecorderError::new(format!(
                    "published {channel:?} segment count does not match recorder receipt"
                )));
            }
            segment_count = segment_count
                .checked_add(actual_count)
                .ok_or_else(|| RecorderError::new("published segment count overflowed"))?;
        }
        if segment_count != recorder_receipt.segment_count {
            return Err(RecorderError::new(
                "published segment count does not match recorder receipt",
            ));
        }
        Ok(Self {
            root,
            manifest,
            evidence_receipt,
            recorder_receipt,
        })
    }

    pub fn manifest(&self) -> &ReferenceRunManifest {
        &self.manifest
    }

    pub fn evidence_receipt(&self) -> &EvidenceReceipt {
        &self.evidence_receipt
    }

    pub fn recorder_receipt(&self) -> &RecorderReceipt {
        &self.recorder_receipt
    }

    pub fn segment_paths(&self, channel: ReferenceChannel) -> Result<Vec<PathBuf>, RecorderError> {
        segment_paths(&self.root, &channel)
    }

    pub fn open_segment(
        &self,
        channel: ReferenceChannel,
        segment_index: u64,
    ) -> Result<FileSegmentReader, RecorderError> {
        let path = self
            .root
            .join(channel_directory_name(&channel))
            .join(segment_file_name(segment_index));
        FileSegmentReader::open(path)
    }
}

impl ReferenceChannel {
    fn all() -> [Self; 6] {
        [
            Self::NormalizedEvents,
            Self::NativeEvidence,
            Self::InputValueEvidence,
            Self::Frames,
            Self::Snapshots,
            Self::CapabilityOutputs,
        ]
    }
}

fn create_staging_directory(staging_root: &Path) -> Result<PathBuf, RecorderError> {
    for _ in 0..16 {
        let token = STAGING_TOKEN.fetch_add(1, Ordering::Relaxed);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = staging_root.join(format!("run-{}-{timestamp}-{token}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(io_error(error)),
        }
    }
    Err(RecorderError::new(
        "could not allocate a unique staging directory",
    ))
}

fn channel_directory_name(channel: &ReferenceChannel) -> &'static str {
    match channel {
        ReferenceChannel::NormalizedEvents => "normalized",
        ReferenceChannel::NativeEvidence => "native",
        ReferenceChannel::InputValueEvidence => "input-values",
        ReferenceChannel::Frames => "frames",
        ReferenceChannel::Snapshots => "snapshots",
        ReferenceChannel::CapabilityOutputs => "capabilities",
    }
}

fn segment_file_name(index: u64) -> String {
    format!("segment-{index:06}.gvmseg")
}

fn segment_paths(root: &Path, channel: &ReferenceChannel) -> Result<Vec<PathBuf>, RecorderError> {
    let directory = root.join(channel_directory_name(channel));
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut paths = fs::read_dir(directory)
        .map_err(io_error)?
        .map(|entry| entry.map_err(io_error).map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "gvmseg")
    });
    paths.sort();
    Ok(paths)
}

fn count_segment_files(root: &Path, channel: &ReferenceChannel) -> Result<u64, RecorderError> {
    let directory = root.join(channel_directory_name(channel));
    if !directory.exists() {
        return Ok(0);
    }
    let mut count = 0u64;
    for entry in fs::read_dir(directory).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "gvmseg")
        {
            count = count
                .checked_add(1)
                .ok_or_else(|| RecorderError::new("segment file count overflowed"))?;
        }
    }
    Ok(count)
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<(), RecorderError> {
    let mut file = File::create(path).map_err(io_error)?;
    serde_json::to_writer_pretty(&mut file, value).map_err(codec_error)?;
    file.write_all(b"\n").map_err(io_error)?;
    file.sync_all().map_err(io_error)
}

fn read_json_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, RecorderError> {
    let file = File::open(path).map_err(io_error)?;
    serde_json::from_reader(BufReader::new(file)).map_err(codec_error)
}

fn sync_directory(path: &Path) -> Result<(), RecorderError> {
    File::open(path)
        .map_err(io_error)?
        .sync_all()
        .map_err(io_error)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use glassvm_core::{
        BudgetReceipt, EmissionBudgets, EvidenceGuarantee, InputCoordinate, InputId, InputSource,
        StructuredValue, TypedInputPayload,
    };

    fn manifest(limits: RecorderLimits, channels: &[ReferenceChannel]) -> ReferenceRunManifest {
        let run_id = RunId("run-1".into());
        let machine_id = MachineId("test.machine".into());
        let machine_version = VersionStamp("test-v1".into());
        let mut request = ObservationRequest::summary();
        request.input_value_evidence.enabled =
            channels.contains(&ReferenceChannel::InputValueEvidence);
        ReferenceRunManifest {
            schema: SchemaRef::new("glassvm.reference_run", REFERENCE_RUN_SCHEMA_VERSION),
            run_id,
            machine_id,
            machine_version,
            request,
            prepared_observation_id: None,
            requested_channels: channels.iter().cloned().collect(),
            limits,
        }
    }

    fn evidence_receipt(
        manifest: &ReferenceRunManifest,
        status: EvidenceStatus,
    ) -> EvidenceReceipt {
        EvidenceReceipt {
            schema_version: EMISSION_SCHEMA_VERSION,
            run_id: manifest.run_id.clone(),
            machine_id: manifest.machine_id.clone(),
            machine_version: manifest.machine_version.clone(),
            requested_guarantee: EvidenceGuarantee::Summary,
            fulfilled_guarantee: (status == EvidenceStatus::Complete)
                .then_some(EvidenceGuarantee::Summary),
            status,
            channels: Vec::new(),
            capabilities: Vec::new(),
            budgets: BudgetReceipt {
                configured: EmissionBudgets::default(),
                exceeded: None,
                observed_logical_bytes: 0,
            },
            failure: None,
        }
    }

    fn frame(manifest: &ReferenceRunManifest, sequence: u64, value: &str) -> RecordedRecord {
        RecordedRecord::Frame(FrameArtifact::full(
            manifest.run_id.clone(),
            manifest.machine_id.clone(),
            manifest.machine_version.clone(),
            SchemaRef::new("test.frame", SchemaVersion::V1),
            sequence,
            sequence,
            sequence,
            value.as_bytes().to_vec(),
        ))
    }

    fn input_evidence(value: &str) -> InputValueEvidence {
        let input_id = InputId::new("test.input").unwrap();
        let payload = TypedInputPayload::new(
            SchemaRef::new("test.input.value", SchemaVersion::V1),
            StructuredValue::Text(value.into()),
        )
        .unwrap();
        InputValueEvidence::new(
            input_id,
            SchemaRef::new("test.input.value", SchemaVersion::V1),
            InputSource::Scheduled { ordinal: 0 },
            InputCoordinate::frame(0),
            payload,
        )
    }

    fn input_value(_manifest: &ReferenceRunManifest, value: &str) -> RecordedRecord {
        RecordedRecord::InputValueEvidence(input_evidence(value))
    }

    fn limits() -> RecorderLimits {
        RecorderLimits {
            max_block_logical_bytes: 4096,
            max_segment_records: 2,
            max_segment_logical_bytes: 8192,
            max_buffered_bytes_per_channel: 4096,
        }
    }

    fn temporary_segment_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "glassvm-recorder-{label}-{}-{}.gvmseg",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn temporary_run_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "glassvm-recorder-run-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn capability_output(id: &str) -> RecordedRecord {
        let capability_id = CapabilityId::new(id).unwrap();
        RecordedRecord::CapabilityOutput(CapabilityOutputRecord {
            capability_id: capability_id.clone(),
            output_schema: capability_id.schema(),
            payload: serde_json::json!({"score": 1}),
        })
    }

    #[test]
    fn hard_segment_limits_are_validated_as_bounds() {
        let header = SegmentHeader {
            schema: SchemaRef::new(
                "glassvm.reference_segment",
                REFERENCE_SEGMENT_SCHEMA_VERSION,
            ),
            channel: ReferenceChannel::NormalizedEvents,
            segment_index: 0,
            record_count: 3,
            first_sequence: Some(0),
            last_sequence: Some(2),
            logical_bytes: 1024,
        };
        assert!(header.validate(&limits()).is_err());

        let mut bounded = header;
        bounded.record_count = 2;
        assert!(bounded.validate(&limits()).is_ok());
    }

    #[test]
    fn durable_capability_output_requires_canonical_id() {
        let id = CapabilityId::new("test.capability.v1").unwrap();
        let output = CapabilityOutputRecord {
            capability_id: id.clone(),
            output_schema: id.schema(),
            payload: serde_json::json!({"value": 1}),
        };
        assert!(output.validate().is_ok());

        let mut mismatched = output;
        mismatched.output_schema.id = CapabilityId::new("other.capability.v1").unwrap();
        assert!(mismatched.validate().is_err());
    }

    #[test]
    fn evidence_failure_does_not_make_a_persisted_recorder_failure() {
        let receipt = RecorderReceipt {
            schema: SchemaRef::new("glassvm.recorder_receipt", RECORDER_RECEIPT_SCHEMA_VERSION),
            status: RecorderStatus::Complete,
            finalized: true,
            evidence_receipt_published: true,
            evidence_status: EvidenceStatus::Failed,
            channels: vec![RecorderChannelReceipt {
                channel: ReferenceChannel::Frames,
                stats: ChannelRecorderStats {
                    segment_count: 1,
                    record_count: 1,
                    block_count: 1,
                    logical_bytes: 10,
                    encoded_bytes: 8,
                },
            }],
            segment_count: 1,
            record_count: 1,
            block_count: 1,
            logical_bytes: 10,
            encoded_bytes: Some(8),
            file_count: 4,
            failure: None,
        };
        assert!(receipt.validate().is_ok());
        assert!(receipt.is_publishable());
    }

    #[test]
    fn finalization_requires_published_evidence_receipt() {
        let receipt = RecorderReceipt {
            schema: SchemaRef::new("glassvm.recorder_receipt", RECORDER_RECEIPT_SCHEMA_VERSION),
            status: RecorderStatus::Complete,
            finalized: true,
            evidence_receipt_published: false,
            evidence_status: EvidenceStatus::Complete,
            channels: Vec::new(),
            segment_count: 0,
            record_count: 0,
            block_count: 0,
            logical_bytes: 0,
            encoded_bytes: None,
            file_count: 3,
            failure: None,
        };
        assert!(receipt.validate().is_err());
        assert!(!receipt.is_publishable());
    }

    #[test]
    fn file_segment_round_trip_streams_records_and_footer() {
        let path = temporary_segment_path("round-trip");
        let manifest = manifest(limits(), &[ReferenceChannel::Frames]);
        let first = frame(&manifest, 0, "a");
        let second = frame(&manifest, 1, "b");
        let mut writer =
            FileSegmentWriter::create(&path, ReferenceChannel::Frames, 0, manifest.limits.clone())
                .unwrap();
        writer.record(first.clone()).unwrap();
        writer.record(second.clone()).unwrap();
        let footer = writer.finish().unwrap();
        assert_eq!(footer.header.record_count, 2);
        assert!(footer.block_count >= 1);

        let mut reader = FileSegmentReader::open(&path).unwrap();
        assert_eq!(reader.prelude().channel, ReferenceChannel::Frames);
        assert_eq!(reader.next_record().unwrap(), Some(first));
        assert_eq!(reader.next_record().unwrap(), Some(second));
        assert_eq!(reader.next_record().unwrap(), None);
        assert_eq!(reader.footer(), Some(&footer));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn file_segment_round_trip_preserves_input_value_payloads() {
        let path = temporary_segment_path("input-values");
        let manifest = manifest(limits(), &[ReferenceChannel::InputValueEvidence]);
        let first = input_value(&manifest, "a");
        let second = input_value(&manifest, "b");
        let mut writer = FileSegmentWriter::create(
            &path,
            ReferenceChannel::InputValueEvidence,
            0,
            manifest.limits.clone(),
        )
        .unwrap();
        writer.record(first.clone()).unwrap();
        writer.record(second.clone()).unwrap();
        writer.finish().unwrap();

        let mut reader = FileSegmentReader::open(&path).unwrap();
        assert_eq!(
            reader.prelude().channel,
            ReferenceChannel::InputValueEvidence
        );
        assert_eq!(reader.next_record().unwrap(), Some(first));
        assert_eq!(reader.next_record().unwrap(), Some(second));
        assert_eq!(reader.next_record().unwrap(), None);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn file_segment_rejects_record_larger_than_block_limit() {
        let path = temporary_segment_path("oversized");
        let limits = RecorderLimits {
            max_block_logical_bytes: 32,
            max_segment_records: 4,
            max_segment_logical_bytes: 64,
            max_buffered_bytes_per_channel: 32,
        };
        let manifest = manifest(limits.clone(), &[ReferenceChannel::Frames]);
        let mut writer =
            FileSegmentWriter::create(&path, ReferenceChannel::Frames, 0, limits).unwrap();
        let error = writer
            .record(frame(&manifest, 0, "this record is larger than the block"))
            .unwrap_err();
        assert!(error.to_string().contains("exceeds block hard limit"));
        drop(writer);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn input_value_evidence_obeys_hard_recorder_limits() {
        let path = temporary_segment_path("input-values-oversized");
        let limits = RecorderLimits {
            max_block_logical_bytes: 32,
            max_segment_records: 4,
            max_segment_logical_bytes: 64,
            max_buffered_bytes_per_channel: 32,
        };
        let manifest = manifest(limits.clone(), &[ReferenceChannel::InputValueEvidence]);
        let mut writer =
            FileSegmentWriter::create(&path, ReferenceChannel::InputValueEvidence, 0, limits)
                .unwrap();
        let error = writer
            .record(input_value(
                &manifest,
                "this input value is deliberately too large",
            ))
            .unwrap_err();
        assert!(error.to_string().contains("exceeds block hard limit"));
        drop(writer);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn file_segment_without_footer_is_not_read_as_complete() {
        let path = temporary_segment_path("incomplete");
        let manifest = manifest(limits(), &[ReferenceChannel::Frames]);
        let mut writer =
            FileSegmentWriter::create(&path, ReferenceChannel::Frames, 0, manifest.limits.clone())
                .unwrap();
        writer.record(frame(&manifest, 0, "partial")).unwrap();
        drop(writer);

        let mut reader = FileSegmentReader::open(&path).unwrap();
        let error = reader.next_record().unwrap_err();
        assert!(error.to_string().contains("without a footer"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn file_run_publishes_multichannel_segments_by_atomic_rename() {
        let root = temporary_run_root("publish");
        let published_path = root.join("run-1");
        let mut run_limits = limits();
        run_limits.max_segment_records = 1;
        let manifest = manifest(
            run_limits,
            &[
                ReferenceChannel::Frames,
                ReferenceChannel::CapabilityOutputs,
            ],
        );
        let mut recorder = FileRunRecorder::create(&published_path, manifest.clone()).unwrap();
        let staging_path = recorder.staging_path().to_path_buf();
        assert!(staging_path.join("manifest.json").is_file());
        recorder.record(frame(&manifest, 0, "a")).unwrap();
        recorder.record(frame(&manifest, 1, "b")).unwrap();
        recorder
            .record(capability_output("test.persisted_output.v1"))
            .unwrap();
        recorder
            .publish_evidence_receipt(evidence_receipt(&manifest, EvidenceStatus::Complete))
            .unwrap();

        let published = recorder.finalize().unwrap();
        assert!(published_path.is_dir());
        assert!(!staging_path.exists());
        assert_eq!(
            published
                .segment_paths(ReferenceChannel::Frames)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            published
                .segment_paths(ReferenceChannel::CapabilityOutputs)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(published.recorder_receipt().segment_count, 3);
        assert_eq!(published.recorder_receipt().file_count, 6);
        let recorder_receipt = published.recorder_receipt();
        assert_eq!(recorder_receipt.channels.len(), 2);
        let frame_stats = recorder_receipt
            .channels
            .iter()
            .find(|entry| entry.channel == ReferenceChannel::Frames)
            .unwrap();
        assert_eq!(frame_stats.stats.segment_count, 2);
        assert_eq!(frame_stats.stats.record_count, 2);
        assert!(frame_stats.stats.block_count >= 2);
        assert!(frame_stats.stats.logical_bytes > 0);
        assert!(frame_stats.stats.encoded_bytes > 0);
        let capability_stats = recorder_receipt
            .channels
            .iter()
            .find(|entry| entry.channel == ReferenceChannel::CapabilityOutputs)
            .unwrap();
        assert_eq!(capability_stats.stats.segment_count, 1);
        assert_eq!(capability_stats.stats.record_count, 1);
        assert_eq!(
            serde_json::from_value::<RecorderReceipt>(
                serde_json::to_value(recorder_receipt).unwrap()
            )
            .unwrap(),
            *recorder_receipt
        );
        assert!(recorder_receipt.channels.len() <= ReferenceChannel::all().len());
        let mut inconsistent_receipt = recorder_receipt.clone();
        inconsistent_receipt.channels[0].stats.record_count += 1;
        assert!(inconsistent_receipt.validate().is_err());
        let mut reader = published.open_segment(ReferenceChannel::Frames, 1).unwrap();
        assert_eq!(
            reader.next_record().unwrap(),
            Some(frame(&manifest, 1, "b"))
        );
        assert_eq!(reader.next_record().unwrap(), None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn file_run_publishes_input_value_channel_and_round_trips_it() {
        let root = temporary_run_root("input-values-publish");
        let published_path = root.join("run-1");
        let manifest = manifest(limits(), &[ReferenceChannel::InputValueEvidence]);
        let mut recorder = FileRunRecorder::create(&published_path, manifest.clone()).unwrap();
        recorder.record(input_value(&manifest, "accepted")).unwrap();
        recorder
            .publish_evidence_receipt(evidence_receipt(&manifest, EvidenceStatus::Complete))
            .unwrap();

        let published = recorder.finalize().unwrap();
        assert_eq!(
            published
                .segment_paths(ReferenceChannel::InputValueEvidence)
                .unwrap()
                .len(),
            1
        );
        let reopened = PublishedFileRun::open(&published_path).unwrap();
        let mut reader = reopened
            .open_segment(ReferenceChannel::InputValueEvidence, 0)
            .unwrap();
        assert_eq!(
            reader.next_record().unwrap(),
            Some(input_value(&manifest, "accepted"))
        );
        assert_eq!(reader.next_record().unwrap(), None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn interrupted_file_run_stays_staged_and_is_not_resumed() {
        let root = temporary_run_root("abandon");
        let published_path = root.join("run-1");
        let manifest = manifest(limits(), &[ReferenceChannel::Frames]);
        let mut recorder = FileRunRecorder::create(&published_path, manifest.clone()).unwrap();
        let staging_path = recorder.staging_path().to_path_buf();
        recorder.record(frame(&manifest, 0, "partial")).unwrap();
        drop(recorder);

        assert!(staging_path.is_dir());
        assert!(!published_path.exists());
        assert!(PublishedFileRun::open(&published_path).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn file_run_without_evidence_receipt_cannot_publish() {
        let root = temporary_run_root("missing-receipt");
        let published_path = root.join("run-1");
        let manifest = manifest(limits(), &[ReferenceChannel::Frames]);
        let recorder = FileRunRecorder::create(&published_path, manifest).unwrap();
        let staging_path = recorder.staging_path().to_path_buf();
        let error = recorder.finalize().unwrap_err();
        assert!(error.to_string().contains("publishing evidence receipt"));
        assert!(staging_path.is_dir());
        assert!(!published_path.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn file_run_can_publish_complete_storage_for_failed_evidence() {
        let root = temporary_run_root("evidence-failed");
        let published_path = root.join("run-1");
        let manifest = manifest(limits(), &[ReferenceChannel::Frames]);
        let mut recorder = FileRunRecorder::create(&published_path, manifest.clone()).unwrap();
        recorder.record(frame(&manifest, 0, "recorded")).unwrap();
        recorder
            .publish_evidence_receipt(evidence_receipt(&manifest, EvidenceStatus::Failed))
            .unwrap();

        let published = recorder.finalize().unwrap();
        assert_eq!(
            published.recorder_receipt().status,
            RecorderStatus::Complete
        );
        assert_eq!(
            published.recorder_receipt().evidence_status,
            EvidenceStatus::Failed
        );
        assert_eq!(published.evidence_receipt().status, EvidenceStatus::Failed);
        std::fs::remove_dir_all(root).unwrap();
    }
}
