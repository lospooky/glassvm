use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{MachineId, RunId, SchemaRef, SchemaVersion, VersionStamp};

/// Current architecture-neutral execution-event schema.
pub const EVENT_SCHEMA_VERSION: SchemaVersion = SchemaVersion::V1;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Address {
    pub space: String,
    pub value: u64,
}

impl Address {
    pub fn new(space: impl Into<String>, value: u64) -> Self {
        Self {
            space: space.into(),
            value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstructionRef {
    pub encoding: String,
    pub bytes: Vec<u8>,
    pub decoded: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateSpace {
    Register,
    Memory,
    Stack,
    Timer,
    Display,
    Input,
    Output,
    Randomness,
    Extension(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateLocation {
    pub space: StateSpace,
    pub name: Option<String>,
    pub address: Option<Address>,
    pub width_bits: Option<u16>,
}

impl StateLocation {
    pub fn named(space: StateSpace, name: impl Into<String>) -> Self {
        Self {
            space,
            name: Some(name.into()),
            address: None,
            width_bits: None,
        }
    }

    pub fn addressed(space: StateSpace, address: Address) -> Self {
        Self {
            space,
            name: None,
            address: Some(address),
            width_bits: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateRead {
    pub location: StateLocation,
    pub value: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateWrite {
    pub location: StateLocation,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlFlowKind {
    Next,
    Branch,
    Call,
    Return,
    Interrupt,
    Trap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlFlow {
    pub kind: ControlFlowKind,
    pub from: Option<Address>,
    pub to: Option<Address>,
    pub taken: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoChannel {
    Display,
    Keypad,
    Sound,
    Timer,
    Collision,
    Randomness,
    Extension(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IoObservation {
    pub port: String,
    pub direction: IoDirection,
    pub channel: IoChannel,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrapInfo {
    pub code: String,
    pub message: String,
    pub fatal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CausalRelation {
    Data,
    Control,
    Input,
    Extension(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CausalLink {
    pub source_sequence: u64,
    pub relation: CausalRelation,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    RunStarted,
    StepStarted,
    InstructionDecoded,
    RegisterRead,
    RegisterWrite,
    MemoryRead,
    MemoryWrite,
    StackChanged,
    BranchTaken,
    Call,
    Return,
    Interrupt,
    Trap,
    DisplayWrite,
    InputApplied,
    InputSampled,
    SoundEmitted,
    TimerChanged,
    FrameCompleted,
    SnapshotCaptured,
    RunHalted,
    RunCrashed,
    Extension(String),
}

/// Universal event envelope emitted by all machine bundles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionEvent {
    pub schema_version: SchemaVersion,
    pub arch: MachineId,
    pub machine_version: VersionStamp,
    pub run_id: RunId,
    pub sequence: u64,
    pub step: u64,
    pub cycle_or_tick: Option<u64>,
    pub frame: Option<u64>,
    pub pc: Option<Address>,
    pub instruction: Option<InstructionRef>,
    pub kind: EventKind,
    pub reads: Vec<StateRead>,
    pub writes: Vec<StateWrite>,
    pub control_flow: Option<ControlFlow>,
    pub io: Vec<IoObservation>,
    pub trap: Option<TrapInfo>,
    pub causes: Vec<CausalLink>,
    pub extensions: BTreeMap<String, Value>,
}

impl ExecutionEvent {
    pub fn from_context(context: &EventContext, kind: EventKind) -> Self {
        Self {
            schema_version: EVENT_SCHEMA_VERSION,
            arch: context.arch.clone(),
            machine_version: context.machine_version.clone(),
            run_id: context.run_id.clone(),
            sequence: context.sequence,
            step: context.step,
            cycle_or_tick: context.cycle_or_tick,
            frame: context.frame,
            pc: context.pc.clone(),
            instruction: context.instruction.clone(),
            kind,
            reads: Vec::new(),
            writes: Vec::new(),
            control_flow: None,
            io: Vec::new(),
            trap: None,
            causes: Vec::new(),
            extensions: BTreeMap::new(),
        }
    }
}

/// Metadata supplied to a bundle's native-event adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventContext {
    pub arch: MachineId,
    pub machine_version: VersionStamp,
    pub run_id: RunId,
    pub sequence: u64,
    pub step: u64,
    pub cycle_or_tick: Option<u64>,
    pub frame: Option<u64>,
    pub pc: Option<Address>,
    pub instruction: Option<InstructionRef>,
}

/// Opaque machine-native event passed through the bundle-owned adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEvent {
    pub schema: SchemaRef,
    pub kind: String,
    pub payload: Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_round_trip_preserves_envelope_and_extensions() {
        let context = EventContext {
            arch: MachineId::from("test"),
            machine_version: VersionStamp::from("test-semantics-1"),
            run_id: RunId::from("run-1"),
            sequence: 7,
            step: 5,
            cycle_or_tick: Some(9),
            frame: Some(1),
            pc: Some(Address::new("program", 0x200)),
            instruction: Some(InstructionRef {
                encoding: "test.word16".into(),
                bytes: vec![0x12, 0x00],
                decoded: Some("jump 0x200".into()),
            }),
        };
        let mut event = ExecutionEvent::from_context(&context, EventKind::BranchTaken);
        event
            .extensions
            .insert("test.native".into(), serde_json::json!({"opcode": 0x1200}));

        let encoded = serde_json::to_vec(&event).expect("serialize");
        let decoded: ExecutionEvent = serde_json::from_slice(&encoded).expect("deserialize");

        assert_eq!(decoded, event);
        assert_eq!(decoded.schema_version, EVENT_SCHEMA_VERSION);
    }
}
