//! Ephemeral, architecture-aware queries over normalized GlassVM traces.
//!
//! This crate owns no database, path, retention policy, or global run state.
//! A [`TraceQueryEngine`] owns one in-memory trace and is dropped when the
//! consumer has obtained the requested projections.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use glassvm_core::{
    Address, CapabilityId, CapabilityOutput, CapabilityReceipt, CausalRelation, ControlFlowKind,
    EMISSION_SCHEMA_VERSION, EVENT_SCHEMA_VERSION, EventKind, EvidenceReceipt, ExecutionEvent,
    IoObservation, SchemaVersion, StateLocation, StateSpace,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryError {
    message: String,
}

impl QueryError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl std::error::Error for QueryError {}

pub const CAPABILITY_EVIDENCE_VIEW_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

/// Ephemeral, versioned consumer view for bundle capability evidence.
///
/// Capability output blobs and retrospective capability receipts remain
/// separate because a receipt describes fulfillment, while a blob is the
/// payload a downstream analysis may consume. This is a consumer view, not a
/// durable recorder format or a replacement for the core evidence receipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityEvidenceView {
    pub schema_version: SchemaVersion,
    pub capability_outputs: Vec<CapabilityOutput>,
    pub evidence_receipt: EvidenceReceipt,
}

impl CapabilityEvidenceView {
    pub fn from_projection(
        capability_outputs: Vec<CapabilityOutput>,
        evidence_receipt: EvidenceReceipt,
    ) -> Result<Self, QueryError> {
        let view = Self {
            schema_version: CAPABILITY_EVIDENCE_VIEW_SCHEMA_VERSION,
            capability_outputs,
            evidence_receipt,
        };
        view.validate()?;
        Ok(view)
    }

    pub fn validate(&self) -> Result<(), QueryError> {
        if self.schema_version != CAPABILITY_EVIDENCE_VIEW_SCHEMA_VERSION {
            return Err(QueryError::new(format!(
                "unsupported capability evidence view schema {}",
                self.schema_version
            )));
        }
        if self.evidence_receipt.schema_version != EMISSION_SCHEMA_VERSION {
            return Err(QueryError::new(format!(
                "unsupported evidence receipt schema {}",
                self.evidence_receipt.schema_version
            )));
        }
        let mut ids = BTreeSet::new();
        for receipt in &self.evidence_receipt.capabilities {
            if !ids.insert(receipt.id.clone()) {
                return Err(QueryError::new(format!(
                    "evidence receipt contains duplicate capability {}",
                    receipt.id.as_str()
                )));
            }
        }
        Ok(())
    }

    pub fn capability_outputs(&self) -> &[CapabilityOutput] {
        &self.capability_outputs
    }

    pub fn capability_receipts(&self) -> &[CapabilityReceipt] {
        &self.evidence_receipt.capabilities
    }

    pub fn capability_receipt(&self, id: &CapabilityId) -> Option<&CapabilityReceipt> {
        self.evidence_receipt
            .capabilities
            .iter()
            .find(|receipt| &receipt.id == id)
    }
}

/// Whether adjacency in the supplied event stream is known to be complete.
///
/// Complete streams permit instruction-anchor projection. Filtered or unknown
/// streams retain explicit evidence but never fabricate adjacency across
/// potentially omitted events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceCoverage {
    Complete,
    Filtered,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EventFilter {
    /// Empty means all event kinds.
    pub kinds: BTreeSet<EventKind>,
    pub first_sequence: Option<u64>,
    pub last_sequence: Option<u64>,
    pub first_step: Option<u64>,
    pub last_step: Option<u64>,
    pub first_tick: Option<u64>,
    pub last_tick: Option<u64>,
    pub first_frame: Option<u64>,
    pub last_frame: Option<u64>,
    /// Required when either numeric PC bound is present.
    pub pc_space: Option<String>,
    pub pc_start: Option<u64>,
    pub pc_end: Option<u64>,
}

impl EventFilter {
    pub fn validate(&self) -> Result<(), QueryError> {
        validate_bounds("sequence", self.first_sequence, self.last_sequence)?;
        validate_bounds("step", self.first_step, self.last_step)?;
        validate_bounds("tick", self.first_tick, self.last_tick)?;
        validate_bounds("frame", self.first_frame, self.last_frame)?;
        validate_bounds("program counter", self.pc_start, self.pc_end)?;
        if (self.pc_start.is_some() || self.pc_end.is_some()) && self.pc_space.is_none() {
            return Err(QueryError::new(
                "program-counter bounds require an address space",
            ));
        }
        if self.pc_space.as_deref().is_some_and(str::is_empty) {
            return Err(QueryError::new(
                "program-counter address space cannot be empty",
            ));
        }
        Ok(())
    }

    pub fn matches(&self, event: &ExecutionEvent) -> bool {
        let pc_selected =
            self.pc_space.is_some() || self.pc_start.is_some() || self.pc_end.is_some();
        let pc_matches = !pc_selected
            || event.pc.as_ref().is_some_and(|pc| {
                self.pc_space
                    .as_deref()
                    .is_none_or(|space| pc.space == space)
                    && self.pc_start.is_none_or(|start| pc.value >= start)
                    && self.pc_end.is_none_or(|end| pc.value <= end)
            });

        (self.kinds.is_empty() || self.kinds.contains(&event.kind))
            && self
                .first_sequence
                .is_none_or(|first| event.sequence >= first)
            && self.last_sequence.is_none_or(|last| event.sequence <= last)
            && self.first_step.is_none_or(|first| event.step >= first)
            && self.last_step.is_none_or(|last| event.step <= last)
            && self
                .first_tick
                .is_none_or(|first| event.cycle_or_tick.is_some_and(|tick| tick >= first))
            && self
                .last_tick
                .is_none_or(|last| event.cycle_or_tick.is_some_and(|tick| tick <= last))
            && self
                .first_frame
                .is_none_or(|first| event.frame.is_some_and(|frame| frame >= first))
            && self
                .last_frame
                .is_none_or(|last| event.frame.is_some_and(|frame| frame <= last))
            && pc_matches
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ControlFlowEdge {
    pub from: Address,
    pub to: Address,
    pub kind: ControlFlowKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlFlowBoundaryKind {
    MissingInstructionPc,
    MissingEndpoint,
    SourceMismatch {
        expected: Address,
        observed: Address,
    },
    TargetMismatch {
        expected: Address,
        observed: Address,
        target_sequence: u64,
    },
    AmbiguousTransfer {
        evidence_sequences: Vec<u64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlFlowBoundary {
    pub source_sequence: u64,
    pub kind: ControlFlowBoundaryKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicControlFlowGraph {
    pub coverage: TraceCoverage,
    pub nodes: BTreeSet<Address>,
    pub edges: BTreeMap<ControlFlowEdge, u64>,
    pub boundaries: Vec<ControlFlowBoundary>,
}

impl DynamicControlFlowGraph {
    /// Unique endpoint pairs, independent of their semantic transfer kind.
    pub fn unique_endpoint_edge_count(&self) -> usize {
        self.edges
            .keys()
            .map(|edge| (&edge.from, &edge.to))
            .collect::<BTreeSet<_>>()
            .len()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateDiff {
    pub sequence: u64,
    pub step: u64,
    pub cycle_or_tick: Option<u64>,
    pub frame: Option<u64>,
    pub write_index: u32,
    pub location: StateLocation,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CausalEdgeKind {
    Explicit(CausalRelation),
    LastWriter,
    ControlTransfer,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CausalEdge {
    pub source_sequence: u64,
    pub target_sequence: u64,
    pub kind: CausalEdgeKind,
    pub source_access_index: Option<u32>,
    pub target_access_index: Option<u32>,
    pub location: Option<StateLocation>,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CausalBoundaryKind {
    NoPriorWriter {
        read_index: u32,
        location: StateLocation,
    },
    IncompleteStateContext {
        read_index: u32,
        location: StateLocation,
    },
    MissingExplicitSource {
        source_sequence: u64,
        relation: CausalRelation,
    },
    NonPriorExplicitSource {
        source_sequence: u64,
        relation: CausalRelation,
    },
    MissingControlTarget,
    NoFollowingInstruction {
        expected: Address,
    },
    FollowingInstructionMissingPc {
        expected: Address,
        target_sequence: u64,
    },
    ControlTargetMismatch {
        expected: Address,
        observed: Address,
        target_sequence: u64,
    },
    IncompleteControlContext {
        expected: Address,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalBoundary {
    pub at_sequence: u64,
    pub kind: CausalBoundaryKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalGraph {
    pub coverage: TraceCoverage,
    pub nodes: BTreeSet<u64>,
    pub edges: Vec<CausalEdge>,
    pub boundaries: Vec<CausalBoundary>,
}

impl CausalGraph {
    pub fn ancestors_of(&self, sequence: u64) -> Result<BTreeSet<u64>, QueryError> {
        if !self.nodes.contains(&sequence) {
            return Err(QueryError::new(format!(
                "event sequence {sequence} is not in the causal graph"
            )));
        }

        let mut incoming: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
        for edge in &self.edges {
            incoming
                .entry(edge.target_sequence)
                .or_default()
                .push(edge.source_sequence);
        }
        let mut ancestors = BTreeSet::new();
        let mut pending = VecDeque::from([sequence]);
        while let Some(target) = pending.pop_front() {
            for source in incoming.get(&target).into_iter().flatten() {
                if *source != sequence && ancestors.insert(*source) {
                    pending.push_back(*source);
                }
            }
        }
        Ok(ancestors)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalEdge {
    pub source_sequence: u64,
    pub target_sequence: u64,
    pub sequence_delta: u64,
    pub step_delta: i128,
    pub tick_delta: Option<i128>,
    pub frame_delta: Option<i128>,
    /// False when an event filter omitted one or more source events between
    /// these two selected nodes.
    pub source_adjacent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalGraph {
    pub nodes: BTreeSet<u64>,
    pub edges: Vec<TemporalEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopSignature {
    pub back_edge: ControlFlowEdge,
    pub executions: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "query", rename_all = "snake_case")]
pub enum QuerySpec {
    Events {
        filter: EventFilter,
    },
    CountByKind,
    CountByKindFiltered {
        filter: EventFilter,
    },
    UniqueProgramCounters,
    DynamicControlFlow,
    StateDiffs {
        require_before: bool,
    },
    StateDiffsFiltered {
        filter: EventFilter,
        require_before: bool,
    },
    CausalGraph,
    CausalAncestors {
        sequence: u64,
    },
    TemporalGraph,
    TemporalGraphFiltered {
        filter: EventFilter,
    },
    LoopSignatures,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum QueryResult {
    Events { events: Vec<ExecutionEvent> },
    CountByKind { counts: BTreeMap<EventKind, u64> },
    UniqueProgramCounters { addresses: BTreeSet<Address> },
    DynamicControlFlow { graph: DynamicControlFlowGraph },
    StateDiffs { diffs: Vec<StateDiff> },
    CausalGraph { graph: CausalGraph },
    CausalAncestors { sequences: BTreeSet<u64> },
    TemporalGraph { graph: TemporalGraph },
    LoopSignatures { loops: Vec<LoopSignature> },
}

/// One ephemeral in-memory trace and its architecture-aware projections.
#[derive(Debug, Clone)]
pub struct TraceQueryEngine {
    events: Vec<ExecutionEvent>,
    coverage: TraceCoverage,
}

impl TraceQueryEngine {
    /// Construct from a caller-owned event vector whose source completeness is
    /// unknown. Use [`Self::with_coverage`] when adjacency claims are required.
    pub fn new(events: Vec<ExecutionEvent>) -> Result<Self, QueryError> {
        Self::with_coverage(events, TraceCoverage::Unknown)
    }

    pub fn with_coverage(
        events: Vec<ExecutionEvent>,
        coverage: TraceCoverage,
    ) -> Result<Self, QueryError> {
        validate_events(&events)?;
        Ok(Self { events, coverage })
    }

    pub fn events(&self) -> &[ExecutionEvent] {
        &self.events
    }

    pub fn coverage(&self) -> TraceCoverage {
        self.coverage
    }

    pub fn filter(&self, filter: &EventFilter) -> Result<Vec<&ExecutionEvent>, QueryError> {
        filter.validate()?;
        Ok(self
            .events
            .iter()
            .filter(|event| filter.matches(event))
            .collect())
    }

    pub fn count_by_kind(&self) -> BTreeMap<EventKind, u64> {
        count_events(self.events.iter())
    }

    pub fn count_by_kind_filtered(
        &self,
        filter: &EventFilter,
    ) -> Result<BTreeMap<EventKind, u64>, QueryError> {
        Ok(count_events(self.filter(filter)?))
    }

    pub fn unique_program_counters(&self) -> BTreeSet<Address> {
        self.events
            .iter()
            .filter_map(|event| event.pc.clone())
            .collect()
    }

    pub fn io(&self) -> impl Iterator<Item = (u64, &IoObservation)> {
        self.events.iter().flat_map(|event| {
            event
                .io
                .iter()
                .map(move |observation| (event.sequence, observation))
        })
    }

    /// Project actual observed instruction-to-instruction transitions when the
    /// source is complete. Explicit taken transfers remain usable for partial
    /// sources and for terminal transfers without a following instruction.
    pub fn dynamic_cfg(&self) -> DynamicControlFlowGraph {
        let mut graph = DynamicControlFlowGraph {
            coverage: self.coverage,
            nodes: BTreeSet::new(),
            edges: BTreeMap::new(),
            boundaries: Vec::new(),
        };
        let anchors: Vec<usize> = self
            .events
            .iter()
            .enumerate()
            .filter_map(|(index, event)| {
                (event.kind == EventKind::InstructionDecoded).then_some(index)
            })
            .collect();
        for index in &anchors {
            if let Some(pc) = &self.events[*index].pc {
                graph.nodes.insert(pc.clone());
            }
        }

        let mut consumed_controls = BTreeSet::new();
        if self.coverage == TraceCoverage::Complete {
            for pair in anchors.windows(2) {
                let source_index = pair[0];
                let target_index = pair[1];
                let source = &self.events[source_index];
                let target = &self.events[target_index];
                let (Some(source_pc), Some(target_pc)) = (&source.pc, &target.pc) else {
                    let sequence = if source.pc.is_none() {
                        source.sequence
                    } else {
                        target.sequence
                    };
                    graph.boundaries.push(ControlFlowBoundary {
                        source_sequence: sequence,
                        kind: ControlFlowBoundaryKind::MissingInstructionPc,
                    });
                    continue;
                };

                let mut matching_controls = Vec::new();
                for index in source_index..target_index {
                    let event = &self.events[index];
                    let Some(control) = &event.control_flow else {
                        continue;
                    };
                    if !control.taken || event.step != source.step {
                        continue;
                    }
                    consumed_controls.insert(index);
                    let Some(to) = &control.to else {
                        graph.boundaries.push(ControlFlowBoundary {
                            source_sequence: event.sequence,
                            kind: ControlFlowBoundaryKind::MissingEndpoint,
                        });
                        continue;
                    };
                    if let Some(from) = &control.from
                        && from != source_pc
                    {
                        graph.boundaries.push(ControlFlowBoundary {
                            source_sequence: event.sequence,
                            kind: ControlFlowBoundaryKind::SourceMismatch {
                                expected: source_pc.clone(),
                                observed: from.clone(),
                            },
                        });
                        continue;
                    }
                    if to != target_pc {
                        graph.boundaries.push(ControlFlowBoundary {
                            source_sequence: event.sequence,
                            kind: ControlFlowBoundaryKind::TargetMismatch {
                                expected: to.clone(),
                                observed: target_pc.clone(),
                                target_sequence: target.sequence,
                            },
                        });
                        continue;
                    }
                    matching_controls.push((event.sequence, control.kind.clone()));
                }

                let kind = match matching_controls.as_slice() {
                    [] => ControlFlowKind::Next,
                    [(_, kind)] => kind.clone(),
                    controls => {
                        graph.boundaries.push(ControlFlowBoundary {
                            source_sequence: source.sequence,
                            kind: ControlFlowBoundaryKind::AmbiguousTransfer {
                                evidence_sequences: controls
                                    .iter()
                                    .map(|(sequence, _)| *sequence)
                                    .collect(),
                            },
                        });
                        controls[0].1.clone()
                    }
                };
                add_cfg_edge(&mut graph, source_pc.clone(), target_pc.clone(), kind);
            }
        }

        // Preserve explicit evidence that could not be paired with complete
        // instruction anchors. This is the only CFG source for partial traces.
        for (index, event) in self.events.iter().enumerate() {
            let Some(control) = &event.control_flow else {
                continue;
            };
            if !control.taken || consumed_controls.contains(&index) {
                continue;
            }
            let (Some(from), Some(to)) = (&control.from, &control.to) else {
                graph.boundaries.push(ControlFlowBoundary {
                    source_sequence: event.sequence,
                    kind: ControlFlowBoundaryKind::MissingEndpoint,
                });
                continue;
            };
            add_cfg_edge(&mut graph, from.clone(), to.clone(), control.kind.clone());
        }

        graph
    }

    pub fn state_diffs(&self, require_before: bool) -> Result<Vec<StateDiff>, QueryError> {
        self.state_diffs_for(self.events.iter(), require_before)
    }

    pub fn state_diffs_filtered(
        &self,
        filter: &EventFilter,
        require_before: bool,
    ) -> Result<Vec<StateDiff>, QueryError> {
        self.state_diffs_for(self.filter(filter)?, require_before)
    }

    fn state_diffs_for<'a>(
        &self,
        events: impl IntoIterator<Item = &'a ExecutionEvent>,
        require_before: bool,
    ) -> Result<Vec<StateDiff>, QueryError> {
        let mut diffs = Vec::new();
        for event in events {
            for (index, write) in event.writes.iter().enumerate() {
                if require_before && write.before.is_none() {
                    return Err(QueryError::new(format!(
                        "event {} write {} lacks before-state data for {:?}",
                        event.sequence, index, write.location
                    )));
                }
                diffs.push(StateDiff {
                    sequence: event.sequence,
                    step: event.step,
                    cycle_or_tick: event.cycle_or_tick,
                    frame: event.frame,
                    write_index: u32::try_from(index)
                        .map_err(|_| QueryError::new("event has too many state writes"))?,
                    location: write.location.clone(),
                    before: write.before.clone(),
                    after: write.after.clone(),
                });
            }
        }
        Ok(diffs)
    }

    pub fn causal_graph(&self) -> CausalGraph {
        let nodes = self.events.iter().map(|event| event.sequence).collect();
        let sequences: BTreeSet<u64> = self.events.iter().map(|event| event.sequence).collect();
        let mut edges = BTreeSet::new();
        let mut boundaries = Vec::new();
        let mut last_writers: BTreeMap<StateKey, (u64, u32)> = BTreeMap::new();
        let next_instruction = next_instruction_indices(&self.events);

        for (event_index, event) in self.events.iter().enumerate() {
            for cause in &event.causes {
                if !sequences.contains(&cause.source_sequence) {
                    boundaries.push(CausalBoundary {
                        at_sequence: event.sequence,
                        kind: CausalBoundaryKind::MissingExplicitSource {
                            source_sequence: cause.source_sequence,
                            relation: cause.relation.clone(),
                        },
                    });
                } else if cause.source_sequence >= event.sequence {
                    boundaries.push(CausalBoundary {
                        at_sequence: event.sequence,
                        kind: CausalBoundaryKind::NonPriorExplicitSource {
                            source_sequence: cause.source_sequence,
                            relation: cause.relation.clone(),
                        },
                    });
                } else {
                    edges.insert(CausalEdge {
                        source_sequence: cause.source_sequence,
                        target_sequence: event.sequence,
                        kind: CausalEdgeKind::Explicit(cause.relation.clone()),
                        source_access_index: None,
                        target_access_index: None,
                        location: None,
                        evidence: cause.evidence.clone(),
                    });
                }
            }

            for (read_index, read) in event.reads.iter().enumerate() {
                let read_index = u32::try_from(read_index).unwrap_or(u32::MAX);
                if self.coverage != TraceCoverage::Complete {
                    boundaries.push(CausalBoundary {
                        at_sequence: event.sequence,
                        kind: CausalBoundaryKind::IncompleteStateContext {
                            read_index,
                            location: read.location.clone(),
                        },
                    });
                } else if let Some((writer_sequence, write_index)) =
                    last_writers.get(&StateKey::from(&read.location))
                {
                    edges.insert(CausalEdge {
                        source_sequence: *writer_sequence,
                        target_sequence: event.sequence,
                        kind: CausalEdgeKind::LastWriter,
                        source_access_index: Some(*write_index),
                        target_access_index: Some(read_index),
                        location: Some(read.location.clone()),
                        evidence: None,
                    });
                } else {
                    boundaries.push(CausalBoundary {
                        at_sequence: event.sequence,
                        kind: CausalBoundaryKind::NoPriorWriter {
                            read_index,
                            location: read.location.clone(),
                        },
                    });
                }
            }

            if let Some(control) = &event.control_flow
                && control.taken
            {
                if let Some(expected) = &control.to {
                    if self.coverage != TraceCoverage::Complete {
                        boundaries.push(CausalBoundary {
                            at_sequence: event.sequence,
                            kind: CausalBoundaryKind::IncompleteControlContext {
                                expected: expected.clone(),
                            },
                        });
                    } else if let Some(target_index) = next_instruction[event_index] {
                        let target = &self.events[target_index];
                        match &target.pc {
                            Some(observed) if observed == expected => {
                                edges.insert(CausalEdge {
                                    source_sequence: event.sequence,
                                    target_sequence: target.sequence,
                                    kind: CausalEdgeKind::ControlTransfer,
                                    source_access_index: None,
                                    target_access_index: None,
                                    location: None,
                                    evidence: None,
                                });
                            }
                            Some(observed) => boundaries.push(CausalBoundary {
                                at_sequence: event.sequence,
                                kind: CausalBoundaryKind::ControlTargetMismatch {
                                    expected: expected.clone(),
                                    observed: observed.clone(),
                                    target_sequence: target.sequence,
                                },
                            }),
                            None => boundaries.push(CausalBoundary {
                                at_sequence: event.sequence,
                                kind: CausalBoundaryKind::FollowingInstructionMissingPc {
                                    expected: expected.clone(),
                                    target_sequence: target.sequence,
                                },
                            }),
                        }
                    } else {
                        boundaries.push(CausalBoundary {
                            at_sequence: event.sequence,
                            kind: CausalBoundaryKind::NoFollowingInstruction {
                                expected: expected.clone(),
                            },
                        });
                    }
                } else {
                    boundaries.push(CausalBoundary {
                        at_sequence: event.sequence,
                        kind: CausalBoundaryKind::MissingControlTarget,
                    });
                }
            }

            for (write_index, write) in event.writes.iter().enumerate() {
                let write_index = u32::try_from(write_index).unwrap_or(u32::MAX);
                last_writers.insert(
                    StateKey::from(&write.location),
                    (event.sequence, write_index),
                );
            }
        }

        CausalGraph {
            coverage: self.coverage,
            nodes,
            edges: edges.into_iter().collect(),
            boundaries,
        }
    }

    pub fn temporal_graph(&self) -> TemporalGraph {
        temporal_graph_from(self.events.iter().enumerate(), self.coverage)
    }

    pub fn temporal_graph_filtered(
        &self,
        filter: &EventFilter,
    ) -> Result<TemporalGraph, QueryError> {
        filter.validate()?;
        Ok(temporal_graph_from(
            self.events
                .iter()
                .enumerate()
                .filter(|(_, event)| filter.matches(event)),
            self.coverage,
        ))
    }

    pub fn loop_signatures(&self) -> Vec<LoopSignature> {
        self.dynamic_cfg()
            .edges
            .into_iter()
            .filter(|(edge, _)| {
                edge.from.space == edge.to.space && edge.to.value <= edge.from.value
            })
            .map(|(back_edge, executions)| LoopSignature {
                back_edge,
                executions,
            })
            .collect()
    }

    pub fn execute(&self, query: &QuerySpec) -> Result<QueryResult, QueryError> {
        Ok(match query {
            QuerySpec::Events { filter } => QueryResult::Events {
                events: self.filter(filter)?.into_iter().cloned().collect(),
            },
            QuerySpec::CountByKind => QueryResult::CountByKind {
                counts: self.count_by_kind(),
            },
            QuerySpec::CountByKindFiltered { filter } => QueryResult::CountByKind {
                counts: self.count_by_kind_filtered(filter)?,
            },
            QuerySpec::UniqueProgramCounters => QueryResult::UniqueProgramCounters {
                addresses: self.unique_program_counters(),
            },
            QuerySpec::DynamicControlFlow => QueryResult::DynamicControlFlow {
                graph: self.dynamic_cfg(),
            },
            QuerySpec::StateDiffs { require_before } => QueryResult::StateDiffs {
                diffs: self.state_diffs(*require_before)?,
            },
            QuerySpec::StateDiffsFiltered {
                filter,
                require_before,
            } => QueryResult::StateDiffs {
                diffs: self.state_diffs_filtered(filter, *require_before)?,
            },
            QuerySpec::CausalGraph => QueryResult::CausalGraph {
                graph: self.causal_graph(),
            },
            QuerySpec::CausalAncestors { sequence } => QueryResult::CausalAncestors {
                sequences: self.causal_graph().ancestors_of(*sequence)?,
            },
            QuerySpec::TemporalGraph => QueryResult::TemporalGraph {
                graph: self.temporal_graph(),
            },
            QuerySpec::TemporalGraphFiltered { filter } => QueryResult::TemporalGraph {
                graph: self.temporal_graph_filtered(filter)?,
            },
            QuerySpec::LoopSignatures => QueryResult::LoopSignatures {
                loops: self.loop_signatures(),
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct StateKey {
    // Width describes this access, not the identity of its base location.
    // GlassVM cannot infer overlapping ranges because address units belong to
    // the machine bundle, so v1 causality matches exact base locations only.
    space: StateSpace,
    name: Option<String>,
    address: Option<Address>,
}

impl From<&StateLocation> for StateKey {
    fn from(location: &StateLocation) -> Self {
        Self {
            space: location.space.clone(),
            name: location.name.clone(),
            address: location.address.clone(),
        }
    }
}

fn validate_bounds(
    coordinate: &str,
    first: Option<u64>,
    last: Option<u64>,
) -> Result<(), QueryError> {
    if let (Some(first), Some(last)) = (first, last)
        && first > last
    {
        return Err(QueryError::new(format!(
            "{coordinate} range starts after it ends"
        )));
    }
    Ok(())
}

fn count_events<'a>(
    events: impl IntoIterator<Item = &'a ExecutionEvent>,
) -> BTreeMap<EventKind, u64> {
    let mut counts = BTreeMap::new();
    for event in events {
        *counts.entry(event.kind.clone()).or_default() += 1;
    }
    counts
}

fn add_cfg_edge(
    graph: &mut DynamicControlFlowGraph,
    from: Address,
    to: Address,
    kind: ControlFlowKind,
) {
    graph.nodes.insert(from.clone());
    graph.nodes.insert(to.clone());
    *graph
        .edges
        .entry(ControlFlowEdge { from, to, kind })
        .or_default() += 1;
}

fn next_instruction_indices(events: &[ExecutionEvent]) -> Vec<Option<usize>> {
    let mut result = vec![None; events.len()];
    let mut next = None;
    for index in (0..events.len()).rev() {
        result[index] = next;
        if events[index].kind == EventKind::InstructionDecoded {
            next = Some(index);
        }
    }
    result
}

fn temporal_graph_from<'a>(
    events: impl IntoIterator<Item = (usize, &'a ExecutionEvent)>,
    coverage: TraceCoverage,
) -> TemporalGraph {
    let selected: Vec<_> = events.into_iter().collect();
    TemporalGraph {
        nodes: selected.iter().map(|(_, event)| event.sequence).collect(),
        edges: selected
            .windows(2)
            .map(|pair| {
                let (source_index, source) = pair[0];
                let (target_index, target) = pair[1];
                TemporalEdge {
                    source_sequence: source.sequence,
                    target_sequence: target.sequence,
                    sequence_delta: target.sequence - source.sequence,
                    step_delta: signed_delta(source.step, target.step),
                    tick_delta: optional_signed_delta(source.cycle_or_tick, target.cycle_or_tick),
                    frame_delta: optional_signed_delta(source.frame, target.frame),
                    source_adjacent: coverage == TraceCoverage::Complete
                        && target_index == source_index + 1
                        && target.sequence.checked_sub(source.sequence) == Some(1),
                }
            })
            .collect(),
    }
}

fn signed_delta(first: u64, second: u64) -> i128 {
    i128::from(second) - i128::from(first)
}

fn optional_signed_delta(first: Option<u64>, second: Option<u64>) -> Option<i128> {
    first
        .zip(second)
        .map(|(first, second)| signed_delta(first, second))
}

fn validate_events(events: &[ExecutionEvent]) -> Result<(), QueryError> {
    let Some(first) = events.first() else {
        return Ok(());
    };
    let mut previous_sequence = first.sequence;
    for (index, event) in events.iter().enumerate() {
        if event.schema_version != EVENT_SCHEMA_VERSION {
            return Err(QueryError::new(format!(
                "event at index {index} uses unsupported schema {}",
                event.schema_version
            )));
        }
        if event.run_id != first.run_id
            || event.arch != first.arch
            || event.machine_version != first.machine_version
        {
            return Err(QueryError::new("query trace mixes execution identities"));
        }
        if index > 0 && event.sequence <= previous_sequence {
            return Err(QueryError::new(
                "query trace events must be strictly sequence ordered",
            ));
        }
        previous_sequence = event.sequence;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use glassvm_core::{
        BudgetedSink, CapabilityStatus, CausalLink, ControlFlow, EmissionSink, EventContext,
        InstructionRef, MachineId, NullSink, ObservationRequest, RunId, StateRead, StateWrite,
        VersionStamp,
    };
    use serde_json::json;

    use super::*;

    fn context(sequence: u64, step: u64, pc: u64) -> EventContext {
        EventContext {
            arch: MachineId::from("test"),
            machine_version: VersionStamp::from("test.v1"),
            run_id: RunId::from("run"),
            sequence,
            step,
            cycle_or_tick: Some(step),
            frame: Some(step / 2),
            pc: Some(Address::new("program", pc)),
            instruction: Some(InstructionRef {
                encoding: "test.word".into(),
                bytes: vec![0, 0],
                decoded: None,
            }),
        }
    }

    fn event(sequence: u64, step: u64, pc: u64, kind: EventKind) -> ExecutionEvent {
        ExecutionEvent::from_context(&context(sequence, step, pc), kind)
    }

    fn location(name: &str) -> StateLocation {
        StateLocation::named(StateSpace::Register, name)
    }

    fn control(
        sequence: u64,
        step: u64,
        pc: u64,
        kind: ControlFlowKind,
        from: Option<u64>,
        to: Option<u64>,
        taken: bool,
    ) -> ExecutionEvent {
        let event_kind = match kind {
            ControlFlowKind::Call => EventKind::Call,
            ControlFlowKind::Return => EventKind::Return,
            _ => EventKind::BranchTaken,
        };
        let mut event = event(sequence, step, pc, event_kind);
        event.control_flow = Some(ControlFlow {
            kind,
            from: from.map(|value| Address::new("program", value)),
            to: to.map(|value| Address::new("program", value)),
            taken,
        });
        event
    }

    fn basic_trace() -> Vec<ExecutionEvent> {
        let mut write = event(0, 0, 0x200, EventKind::RegisterWrite);
        write.writes.push(StateWrite {
            location: location("r0"),
            before: Some(json!(0)),
            after: Some(json!(1)),
        });

        let mut branch = control(
            1,
            1,
            0x202,
            ControlFlowKind::Branch,
            Some(0x202),
            Some(0x200),
            true,
        );
        branch.reads.push(StateRead {
            location: location("r0"),
            value: Some(json!(1)),
        });

        let mut next = event(2, 2, 0x200, EventKind::InstructionDecoded);
        next.causes.push(CausalLink {
            source_sequence: 0,
            relation: CausalRelation::Input,
            evidence: Some("fixture".into()),
        });
        vec![write, branch, next]
    }

    #[test]
    fn query_engine_projects_one_trace_consistently() {
        let engine = TraceQueryEngine::with_coverage(basic_trace(), TraceCoverage::Complete)
            .expect("engine");
        assert_eq!(engine.unique_program_counters().len(), 2);
        assert_eq!(engine.count_by_kind()[&EventKind::BranchTaken], 1);

        let cfg = engine.dynamic_cfg();
        assert_eq!(cfg.edges.values().sum::<u64>(), 1);
        assert_eq!(cfg.unique_endpoint_edge_count(), 1);
        let loops = engine.loop_signatures();
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].back_edge.to.value, 0x200);

        let diffs = engine.state_diffs(true).expect("complete diffs");
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].before, Some(json!(0)));
        assert_eq!(diffs[0].write_index, 0);

        let causal = engine.causal_graph();
        assert!(causal.edges.iter().any(|edge| {
            edge.source_sequence == 0
                && edge.target_sequence == 1
                && edge.kind == CausalEdgeKind::LastWriter
        }));
        assert!(causal.edges.iter().any(|edge| {
            edge.source_sequence == 1
                && edge.target_sequence == 2
                && edge.kind == CausalEdgeKind::ControlTransfer
        }));
        assert_eq!(
            causal.ancestors_of(2).expect("known node"),
            BTreeSet::from([0, 1])
        );

        let temporal = engine.temporal_graph();
        assert_eq!(temporal.edges.len(), 2);
        assert_eq!(temporal.edges[0].step_delta, 1);
        assert_eq!(temporal.edges[0].sequence_delta, 1);
    }

    #[test]
    fn filters_validate_ranges_spaces_and_optional_coordinates() {
        let mut events = basic_trace();
        events[0].cycle_or_tick = None;
        events[0].frame = None;
        let engine = TraceQueryEngine::new(events).expect("engine");
        let filter = EventFilter {
            kinds: BTreeSet::from([EventKind::BranchTaken, EventKind::InstructionDecoded]),
            first_sequence: Some(1),
            last_sequence: Some(2),
            first_step: Some(1),
            last_step: Some(2),
            first_tick: Some(1),
            last_tick: Some(2),
            first_frame: Some(0),
            last_frame: Some(1),
            pc_space: Some("program".into()),
            pc_start: Some(0x200),
            pc_end: Some(0x202),
        };
        let selected = engine.filter(&filter).expect("valid filter");
        assert_eq!(
            selected
                .iter()
                .map(|event| event.sequence)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        let counts = engine
            .count_by_kind_filtered(&filter)
            .expect("filtered counts");
        assert_eq!(counts[&EventKind::BranchTaken], 1);
        assert_eq!(counts[&EventKind::InstructionDecoded], 1);

        let wrong_space = EventFilter {
            pc_space: Some("memory".into()),
            ..filter.clone()
        };
        assert!(
            engine
                .filter(&wrong_space)
                .expect("valid filter")
                .is_empty()
        );

        let missing_space = EventFilter {
            pc_start: Some(0x200),
            ..EventFilter::default()
        };
        assert!(engine.filter(&missing_space).is_err());
        let reversed = EventFilter {
            first_tick: Some(3),
            last_tick: Some(2),
            ..EventFilter::default()
        };
        assert!(engine.filter(&reversed).is_err());
    }

    #[test]
    fn state_diffs_preserve_write_order_coordinates_and_null() {
        let mut write = event(0, 7, 0x200, EventKind::RegisterWrite);
        write.cycle_or_tick = Some(9);
        write.frame = Some(3);
        write.writes.push(StateWrite {
            location: location("r0"),
            before: Some(Value::Null),
            after: Some(json!(1)),
        });
        write.writes.push(StateWrite {
            location: location("r0"),
            before: Some(json!(1)),
            after: Some(json!(2)),
        });
        let engine = TraceQueryEngine::new(vec![write]).expect("engine");
        let diffs = engine.state_diffs(true).expect("complete diffs");
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[0].write_index, 0);
        assert_eq!(diffs[1].write_index, 1);
        assert_eq!(diffs[0].before, Some(Value::Null));
        assert_eq!(diffs[0].cycle_or_tick, Some(9));
        assert_eq!(diffs[0].frame, Some(3));

        let mut incomplete = event(1, 8, 0x202, EventKind::RegisterWrite);
        incomplete.writes.push(StateWrite {
            location: location("r1"),
            before: None,
            after: Some(json!(1)),
        });
        let engine = TraceQueryEngine::new(vec![incomplete]).expect("engine");
        assert!(engine.state_diffs(true).is_err());
    }

    #[test]
    fn dynamic_cfg_uses_observed_anchors_and_control_only_as_classification() {
        let events = vec![
            event(0, 0, 0x200, EventKind::InstructionDecoded),
            control(
                1,
                0,
                0x200,
                ControlFlowKind::Branch,
                Some(0x200),
                Some(0x208),
                false,
            ),
            event(2, 1, 0x202, EventKind::InstructionDecoded),
            control(
                3,
                1,
                0x202,
                ControlFlowKind::Branch,
                Some(0x202),
                Some(0x208),
                true,
            ),
            event(4, 1, 0x202, EventKind::DisplayWrite),
            event(5, 2, 0x208, EventKind::InstructionDecoded),
            control(
                6,
                2,
                0x208,
                ControlFlowKind::Branch,
                Some(0x208),
                Some(0x200),
                true,
            ),
            event(7, 2, 0x208, EventKind::DisplayWrite),
            event(8, 3, 0x200, EventKind::InstructionDecoded),
            event(9, 4, 0x202, EventKind::InstructionDecoded),
            control(
                10,
                4,
                0x202,
                ControlFlowKind::Branch,
                Some(0x202),
                Some(0x208),
                true,
            ),
            event(11, 5, 0x208, EventKind::InstructionDecoded),
        ];
        let engine =
            TraceQueryEngine::with_coverage(events, TraceCoverage::Complete).expect("engine");
        let cfg = engine.dynamic_cfg();
        assert_eq!(cfg.edges.values().sum::<u64>(), 5);
        assert_eq!(
            cfg.edges[&ControlFlowEdge {
                from: Address::new("program", 0x200),
                to: Address::new("program", 0x202),
                kind: ControlFlowKind::Next,
            }],
            2
        );
        assert_eq!(
            cfg.edges[&ControlFlowEdge {
                from: Address::new("program", 0x202),
                to: Address::new("program", 0x208),
                kind: ControlFlowKind::Branch,
            }],
            2
        );
        assert!(!cfg.edges.keys().any(|edge| {
            edge.from.value == 0x200
                && edge.to.value == 0x208
                && edge.kind == ControlFlowKind::Branch
        }));

        let causal = engine.causal_graph();
        assert!(causal.edges.iter().any(|edge| {
            edge.source_sequence == 3
                && edge.target_sequence == 5
                && edge.kind == CausalEdgeKind::ControlTransfer
        }));
        assert!(causal.edges.iter().any(|edge| {
            edge.source_sequence == 6
                && edge.target_sequence == 8
                && edge.kind == CausalEdgeKind::ControlTransfer
        }));
        assert!(causal.edges.iter().any(|edge| {
            edge.source_sequence == 10
                && edge.target_sequence == 11
                && edge.kind == CausalEdgeKind::ControlTransfer
        }));
        assert!(!causal.edges.iter().any(|edge| {
            edge.kind == CausalEdgeKind::ControlTransfer && matches!(edge.target_sequence, 4 | 7)
        }));
    }

    #[test]
    fn control_target_mismatch_is_a_boundary_and_never_scans_ahead() {
        let events = vec![
            event(0, 0, 0x200, EventKind::InstructionDecoded),
            control(
                1,
                0,
                0x200,
                ControlFlowKind::Branch,
                Some(0x200),
                Some(0x300),
                true,
            ),
            // A same-step sibling at the expected PC must not satisfy control.
            event(2, 0, 0x300, EventKind::DisplayWrite),
            event(3, 1, 0x202, EventKind::InstructionDecoded),
            // A later loop occurrence must not satisfy the old transfer.
            event(4, 2, 0x300, EventKind::InstructionDecoded),
            control(
                5,
                2,
                0x300,
                ControlFlowKind::Branch,
                Some(0x300),
                Some(0x400),
                true,
            ),
        ];
        let engine =
            TraceQueryEngine::with_coverage(events, TraceCoverage::Complete).expect("engine");
        let causal = engine.causal_graph();
        assert!(!causal.edges.iter().any(|edge| {
            edge.source_sequence == 1 && edge.kind == CausalEdgeKind::ControlTransfer
        }));
        assert!(causal.boundaries.iter().any(|boundary| {
            boundary.at_sequence == 1
                && matches!(
                    boundary.kind,
                    CausalBoundaryKind::ControlTargetMismatch {
                        target_sequence: 3,
                        ..
                    }
                )
        }));
        assert!(causal.boundaries.iter().any(|boundary| {
            boundary.at_sequence == 5
                && matches!(
                    boundary.kind,
                    CausalBoundaryKind::NoFollowingInstruction { .. }
                )
        }));

        let cfg = engine.dynamic_cfg();
        assert!(cfg.boundaries.iter().any(|boundary| {
            boundary.source_sequence == 1
                && matches!(
                    boundary.kind,
                    ControlFlowBoundaryKind::TargetMismatch {
                        target_sequence: 3,
                        ..
                    }
                )
        }));
        assert!(!cfg.edges.keys().any(|edge| {
            edge.from.value == 0x200
                && edge.to.value == 0x300
                && edge.kind == ControlFlowKind::Branch
        }));
    }

    #[test]
    fn last_writer_is_width_insensitive_ordered_and_space_safe() {
        let mut first = event(0, 0, 0x200, EventKind::RegisterWrite);
        let mut wide = location("r0");
        wide.width_bits = Some(8);
        first.writes.push(StateWrite {
            location: wide,
            before: Some(json!(0)),
            after: Some(json!(1)),
        });
        first.writes.push(StateWrite {
            location: StateLocation::addressed(StateSpace::Memory, Address::new("memory", 0)),
            before: Some(json!(0)),
            after: Some(json!(1)),
        });

        let mut read_modify_write = event(1, 1, 0x202, EventKind::RegisterWrite);
        read_modify_write.reads.push(StateRead {
            location: location("r0"),
            value: Some(json!(1)),
        });
        read_modify_write.writes.push(StateWrite {
            location: location("r0"),
            before: Some(json!(1)),
            after: Some(json!(2)),
        });
        read_modify_write.writes.push(StateWrite {
            location: location("r0"),
            before: Some(json!(2)),
            after: Some(json!(3)),
        });

        let mut final_read = event(2, 2, 0x204, EventKind::RegisterRead);
        final_read.reads.push(StateRead {
            location: location("r0"),
            value: Some(json!(3)),
        });
        final_read.reads.push(StateRead {
            location: location("never_written"),
            value: None,
        });
        final_read.reads.push(StateRead {
            location: StateLocation::addressed(StateSpace::Memory, Address::new("other-memory", 0)),
            value: None,
        });

        let graph = TraceQueryEngine::with_coverage(
            vec![first, read_modify_write, final_read],
            TraceCoverage::Complete,
        )
        .expect("engine")
        .causal_graph();
        assert!(graph.edges.iter().any(|edge| {
            edge.source_sequence == 0
                && edge.target_sequence == 1
                && edge.kind == CausalEdgeKind::LastWriter
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.source_sequence == 1
                && edge.target_sequence == 2
                && edge.source_access_index == Some(1)
                && edge.target_access_index == Some(0)
        }));
        assert!(!graph.edges.iter().any(|edge| {
            edge.source_sequence == edge.target_sequence && edge.kind == CausalEdgeKind::LastWriter
        }));
        assert_eq!(
            graph
                .boundaries
                .iter()
                .filter(|boundary| {
                    matches!(boundary.kind, CausalBoundaryKind::NoPriorWriter { .. })
                })
                .count(),
            2
        );
    }

    #[test]
    fn partial_traces_do_not_invent_last_writer_edges() {
        let mut write = event(0, 0, 0x200, EventKind::RegisterWrite);
        write.writes.push(StateWrite {
            location: location("r0"),
            before: Some(json!(0)),
            after: Some(json!(1)),
        });
        let mut read = event(2, 2, 0x204, EventKind::RegisterRead);
        read.reads.push(StateRead {
            location: location("r0"),
            value: Some(json!(2)),
        });

        let graph = TraceQueryEngine::with_coverage(vec![write, read], TraceCoverage::Filtered)
            .expect("engine")
            .causal_graph();
        assert!(
            !graph
                .edges
                .iter()
                .any(|edge| edge.kind == CausalEdgeKind::LastWriter)
        );
        assert!(graph.boundaries.iter().any(|boundary| {
            matches!(
                boundary.kind,
                CausalBoundaryKind::IncompleteStateContext { .. }
            )
        }));
    }

    #[test]
    fn explicit_causes_must_be_prior_and_in_trace() {
        let source = event(0, 0, 0x200, EventKind::InputSampled);
        let mut target = event(1, 1, 0x202, EventKind::DisplayWrite);
        target.causes.extend([
            CausalLink {
                source_sequence: 0,
                relation: CausalRelation::Input,
                evidence: Some("valid".into()),
            },
            CausalLink {
                source_sequence: 1,
                relation: CausalRelation::Control,
                evidence: Some("self".into()),
            },
            CausalLink {
                source_sequence: 99,
                relation: CausalRelation::Data,
                evidence: Some("missing".into()),
            },
        ]);
        let graph = TraceQueryEngine::new(vec![source, target])
            .expect("engine")
            .causal_graph();
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| matches!(edge.kind, CausalEdgeKind::Explicit(_)))
                .count(),
            1
        );
        assert_eq!(graph.boundaries.len(), 2);
        assert_eq!(
            graph.ancestors_of(1).expect("known target"),
            BTreeSet::from([0])
        );
        assert!(graph.ancestors_of(99).is_err());
    }

    #[test]
    fn temporal_graph_preserves_gaps_missing_coordinates_and_regressions() {
        let mut first = event(2, 3, 0x200, EventKind::StepStarted);
        first.cycle_or_tick = Some(5);
        first.frame = Some(2);
        let mut second = event(5, 3, 0x202, EventKind::DisplayWrite);
        second.cycle_or_tick = None;
        second.frame = Some(1);
        let third = event(6, 2, 0x204, EventKind::RunHalted);
        let engine = TraceQueryEngine::new(vec![first, second, third]).expect("engine");
        let graph = engine.temporal_graph();
        assert_eq!(graph.edges[0].sequence_delta, 3);
        assert!(!graph.edges[0].source_adjacent);
        assert_eq!(graph.edges[0].step_delta, 0);
        assert_eq!(graph.edges[0].tick_delta, None);
        assert_eq!(graph.edges[0].frame_delta, Some(-1));
        assert_eq!(graph.edges[1].step_delta, -1);

        let filtered = engine
            .temporal_graph_filtered(&EventFilter {
                kinds: BTreeSet::from([EventKind::StepStarted, EventKind::RunHalted]),
                ..EventFilter::default()
            })
            .expect("filtered graph");
        assert_eq!(filtered.nodes, BTreeSet::from([2, 6]));
        assert_eq!(filtered.edges.len(), 1);
        assert!(!filtered.edges[0].source_adjacent);

        assert!(
            TraceQueryEngine::new(Vec::new())
                .expect("empty")
                .temporal_graph()
                .edges
                .is_empty()
        );
        assert!(
            TraceQueryEngine::new(vec![event(0, 0, 0x200, EventKind::StepStarted)])
                .expect("single")
                .temporal_graph()
                .edges
                .is_empty()
        );
    }

    #[test]
    fn validation_rejects_wrong_schema_identity_and_order_but_accepts_gaps() {
        assert!(TraceQueryEngine::new(Vec::new()).is_ok());
        assert!(
            TraceQueryEngine::new(vec![
                event(0, 0, 0x200, EventKind::StepStarted),
                event(2, 1, 0x202, EventKind::StepStarted),
            ])
            .is_ok()
        );

        let mut wrong_schema = event(0, 0, 0x200, EventKind::StepStarted);
        wrong_schema.schema_version = SchemaVersion::new(2, 0, 0);
        assert!(TraceQueryEngine::new(vec![wrong_schema]).is_err());

        let mut events = basic_trace();
        events.swap(0, 1);
        assert!(TraceQueryEngine::new(events).is_err());

        let mut events = basic_trace();
        events[2].run_id = RunId::from("other");
        assert!(TraceQueryEngine::new(events).is_err());

        let mut events = basic_trace();
        events[2].arch = MachineId::from("other");
        assert!(TraceQueryEngine::new(events).is_err());

        let mut events = basic_trace();
        events[2].machine_version = VersionStamp::from("other.v1");
        assert!(TraceQueryEngine::new(events).is_err());
    }

    #[test]
    fn extension_kinds_remain_distinct_in_filtered_counts() {
        let events = vec![
            event(0, 0, 0x200, EventKind::Extension("test.alpha".into())),
            event(1, 1, 0x202, EventKind::Extension("test.beta".into())),
            event(2, 2, 0x204, EventKind::Extension("test.alpha".into())),
        ];
        let engine = TraceQueryEngine::new(events).expect("engine");
        let filter = EventFilter {
            kinds: BTreeSet::from([EventKind::Extension("test.alpha".into())]),
            ..EventFilter::default()
        };
        let counts = engine
            .count_by_kind_filtered(&filter)
            .expect("filtered counts");
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[&EventKind::Extension("test.alpha".into())], 2);
    }

    #[test]
    fn typed_queries_round_trip_deterministically() {
        let engine = TraceQueryEngine::new(basic_trace()).expect("engine");
        let filter = EventFilter {
            kinds: BTreeSet::from([EventKind::BranchTaken]),
            ..EventFilter::default()
        };
        let first = engine
            .execute(&QuerySpec::CountByKindFiltered {
                filter: filter.clone(),
            })
            .expect("query");
        let second = engine
            .execute(&QuerySpec::CountByKindFiltered { filter })
            .expect("query");
        assert_eq!(first, second);
        let encoded = serde_json::to_vec(&first).expect("serialize");
        let decoded: QueryResult = serde_json::from_slice(&encoded).expect("deserialize");
        assert_eq!(decoded, first);
    }

    fn capability_receipt(id: &str, status: CapabilityStatus) -> CapabilityReceipt {
        CapabilityReceipt {
            id: CapabilityId::new(id).expect("capability ID"),
            status,
            output_schema: None,
            logical_bytes: 0,
            message: None,
        }
    }

    #[test]
    fn capability_evidence_view_keeps_outputs_and_receipts_separate() {
        let mut sink = BudgetedSink::new(
            NullSink,
            ObservationRequest::summary(),
            RunId::from("run"),
            MachineId::from("test"),
            VersionStamp::from("test.v1"),
        )
        .expect("budget sink");
        let output = CapabilityOutput {
            schema: CapabilityId::new("test.summary").unwrap().schema(),
            value: json!({"value": 7}),
        };
        let receipts = vec![
            capability_receipt("test.fulfilled.v1", CapabilityStatus::Fulfilled),
            capability_receipt("test.unavailable.v1", CapabilityStatus::Unavailable),
            capability_receipt("test.failed.v1", CapabilityStatus::Failed),
            capability_receipt("test.incomplete.v1", CapabilityStatus::Incomplete),
        ];
        sink.record_capability_receipts(&receipts);
        let published = sink.emit_receipt().expect("receipt");

        let view = CapabilityEvidenceView::from_projection(vec![output.clone()], published.clone())
            .expect("view");
        assert_eq!(view.capability_outputs(), &[output]);
        assert_eq!(view.capability_receipts(), receipts.as_slice());
        assert_eq!(
            view.capability_receipt(&CapabilityId::new("test.failed.v1").unwrap())
                .expect("failed receipt")
                .status,
            CapabilityStatus::Failed
        );
        assert_eq!(view.evidence_receipt, published);

        let encoded = serde_json::to_vec(&view).expect("serialize view");
        let decoded: CapabilityEvidenceView =
            serde_json::from_slice(&encoded).expect("deserialize view");
        assert_eq!(decoded, view);
    }

    #[test]
    fn capability_evidence_view_rejects_duplicate_receipt_ids() {
        let mut sink = BudgetedSink::new(
            NullSink,
            ObservationRequest::summary(),
            RunId::from("run"),
            MachineId::from("test"),
            VersionStamp::from("test.v1"),
        )
        .expect("budget sink");
        sink.record_capability_receipts(&[capability_receipt(
            "test.duplicate.v1",
            CapabilityStatus::Unavailable,
        )]);
        let receipt = sink.emit_receipt().expect("receipt");
        let mut view = CapabilityEvidenceView::from_projection(Vec::new(), receipt)
            .expect("valid explicit projection");
        view.evidence_receipt.capabilities.push(capability_receipt(
            "test.duplicate.v1",
            CapabilityStatus::Failed,
        ));
        assert!(view.validate().is_err());
    }
}
