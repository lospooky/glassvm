//! Stable capability key constants used across all machine adapters.
//!
//! Key naming convention: `<domain>.<name>`
//! All strings are lowercase, dot-separated.

// ── Display ──────────────────────────────────────────────────────────────────

/// Final framebuffer as a flat byte array with associated geometry.
/// Payload: `{ width: u32, height: u32, planes: u32, bytes: [u8] }`
pub const DISPLAY_FRAMEBUFFER_FLAT: &str = "display.framebuffer_flat";

/// Display dimensions only (no pixel data).
/// Payload: `{ width: u32, height: u32, planes: u32 }`
pub const DISPLAY_DIMS: &str = "display.dims";

// ── Events ────────────────────────────────────────────────────────────────────

/// Ordered event stream recorded during a run.
/// Payload: `[ { frame: u64, kind: String, data: Value }, ... ]`
pub const EVENTS_STREAM: &str = "events.stream";

// ── Coverage ─────────────────────────────────────────────────────────────────

/// Per-address execution counts.
/// Payload: `{ addresses: [u16], counts: [u64] }`
pub const COVERAGE_MAP: &str = "coverage.map";

/// Aggregate coverage summary.
/// Payload: `{ instructions_reached: u32, total_instructions: u32, coverage_ratio: f64 }`
pub const COVERAGE_SUMMARY: &str = "coverage.summary";

// ── CHIP-8 execution metrics ──────────────────────────────────────────────────

/// High-level execution summary for a CHIP-8 run.
/// Payload: `{ rom_hash: u64, draw_count: u32, input_opcode_count: u32, unique_frame_count: u32 }`
pub const CHIP8_EXECUTION_SUMMARY: &str = "chip8.execution_summary";

/// Interestingness metrics for a CHIP-8 run.
/// Payload includes frame entropy/change, opcode and coverage metrics; bounded
/// aggregate display activity, delta, locality, edge-density, and recurrence
/// metrics; plus clear, timer-set, sound-set, and scroll instruction counts.
pub const CHIP8_INTERESTINGNESS: &str = "chip8.interestingness";

/// Coverage metrics for a CHIP-8 run.
/// Payload: `{ unique_pcs: usize, unique_edges: usize, memory_written_bytes: usize, max_stack_depth: usize }`
pub const CHIP8_COVERAGE: &str = "chip8.coverage";

/// Per-frame FNV-1a hashes (only when `record_frames` was set).
/// Payload: `[u64, ...]`
pub const CHIP8_FRAME_HASHES: &str = "chip8.frame_hashes";

/// Exact SHA-256 identity of every completed physical composite frame.
/// Payload: `{ definition: String, digest: String, frame_count: u64 }`
pub const CHIP8_TRAJECTORY_IDENTITY: &str = "chip8.trajectory_identity";

// ── Soft CHIP-8 execution metrics ─────────────────────────────────────────────────

/// High-level execution summary for a projected Soft CHIP-8 run.
pub const SOFT_CHIP8_EXECUTION_SUMMARY: &str = "soft_chip8.execution_summary";

/// Metrics produced by exact execution of projected hard program bytes.
pub const SOFT_CHIP8_PROJECTED_HARD_METRICS: &str = "soft_chip8.projected_hard_metrics";

/// Non-proof evidence produced by an explicit detached autograd run.
pub const SOFT_CHIP8_SURROGATE_DIAGNOSTICS: &str = "soft_chip8.surrogate_diagnostics";

/// Coverage metrics for a projected Soft CHIP-8 run.
pub const SOFT_CHIP8_COVERAGE: &str = "soft_chip8.coverage";

/// Per-frame hashes emitted by the projected-byte compatibility runtime.
pub const SOFT_CHIP8_FRAME_HASHES: &str = "soft_chip8.frame_hashes";

/// Exact identity of every completed projected-byte frame.
pub const SOFT_CHIP8_TRAJECTORY_IDENTITY: &str = "soft_chip8.trajectory_identity";

// ── Verifier ─────────────────────────────────────────────────────────────────

/// Detected ISA extension level.
/// Payload: `"chip8" | "superchip" | "xochip"`
pub const VERIFIER_EXTENSION_LEVEL: &str = "verifier.extension_level";

/// Structural CFG metrics.
/// Payload: `{ reachable_instruction_count, basic_block_count, loop_count, max_cfg_depth, reachable_ratio, estimated_code_bytes, estimated_data_bytes }`
pub const VERIFIER_STRUCTURAL: &str = "verifier.structural";

/// Behavioral feature flags detected by static analysis.
/// Payload: `{ contains_draw, contains_key_input, contains_timers, contains_sound, contains_collision_detection, contains_randomness }`
pub const VERIFIER_BEHAVIORAL: &str = "verifier.behavioral";

// ── Typed helper structs ──────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};

/// Typed wrapper for `DISPLAY_DIMS` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayDims {
    pub width: u32,
    pub height: u32,
    pub planes: u32,
}

/// Typed wrapper for `COVERAGE_SUMMARY` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageSummary {
    pub instructions_reached: u32,
    pub total_instructions: u32,
    pub coverage_ratio: f64,
}

/// A single event entry within the `EVENTS_STREAM` capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEntry {
    pub frame: u64,
    pub kind: String,
    pub data: serde_json::Value,
}
