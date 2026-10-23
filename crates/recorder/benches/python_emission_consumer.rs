use std::collections::BTreeMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use glassvm_core::{
    AccessDetail, BudgetedSink, CommonMetrics, Emission, EmissionSink, EmulatorSession,
    EventContext, EventKind, EventSelection, ExecutionControls, ExecutionEvent, ExecutionRequest,
    InputSchedule, MachineConfiguration, NullSink, ObservationRequest, RunResult, SchemaRef,
    SchemaVersion, StructuredValue, VersionStamp,
};
use glassvm_recorder::{
    FileRecorderRequest, FileRunSession, PythonBatchPolicy, PythonCallbackConsumer,
    PythonEmissionConsumer, RecorderLimits,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;

const MACHINE_ID: &str = "bench.machine";
const MACHINE_VERSION: &str = "bench-v1";
const EVENT_PAYLOAD_BYTES: usize = 16 * 1024;
struct SyntheticSession {
    request: ExecutionRequest,
    event_count: usize,
}

impl EmulatorSession for SyntheticSession {
    fn request(&self) -> &ExecutionRequest {
        &self.request
    }

    fn execute(&mut self, sink: &mut dyn EmissionSink) -> Result<RunResult, String> {
        sink.emit(Emission::RunStarted(&self.request))
            .map_err(|error| error.to_string())?;
        for sequence in 0..self.event_count {
            let context = EventContext {
                arch: self.request.machine_id.clone(),
                machine_version: VersionStamp::from(MACHINE_VERSION),
                run_id: self.request.run_id.clone(),
                sequence: sequence as u64,
                step: sequence as u64,
                cycle_or_tick: Some(sequence as u64),
                frame: None,
                pc: None,
                instruction: None,
            };
            let mut event = ExecutionEvent::from_context(&context, EventKind::StepStarted);
            event.extensions.insert(
                "benchmark_payload".into(),
                serde_json::Value::String("x".repeat(EVENT_PAYLOAD_BYTES)),
            );
            sink.emit(Emission::Event(&event))
                .map_err(|error| error.to_string())?;
        }
        let result = RunResult {
            common: CommonMetrics {
                cycles: self.event_count as u64,
                frames: 0,
                termination: "benchmark_complete".into(),
                boot_success: true,
            },
            capabilities: Vec::new(),
        };
        sink.emit(Emission::RunFinished(&result))
            .map_err(|error| error.to_string())?;
        Ok(result)
    }

    fn step_frame(&mut self) -> Result<(), String> {
        unreachable!("benchmark executes as one bounded synthetic run")
    }

    fn reset(&mut self) -> Result<(), String> {
        unreachable!("benchmark session is not reset")
    }

    fn snapshot(&self) -> Result<Vec<u8>, String> {
        unreachable!("benchmark session does not snapshot")
    }

    fn restore_snapshot(&mut self, _bytes: &[u8]) -> Result<(), String> {
        unreachable!("benchmark session does not restore")
    }
}

#[derive(Default)]
struct BenchRow {
    mode: String,
    policy: String,
    elapsed: Duration,
    batches: u64,
    items: u64,
    logical_bytes: u64,
    peak_buffer_items: usize,
    peak_buffer_logical_bytes: usize,
    peak_rss_kib: Option<u64>,
}

fn request(observation_enabled: bool, run_id: String) -> ExecutionRequest {
    let mut observation = ObservationRequest::summary();
    if observation_enabled {
        observation.normalized_events.events = EventSelection::All;
        observation.normalized_events.access_detail = AccessDetail::None;
    }
    ExecutionRequest::new(
        run_id,
        MACHINE_ID,
        vec![1; 32],
        MachineConfiguration::new(
            SchemaRef::new("bench.configuration", SchemaVersion::V1),
            StructuredValue::Map(BTreeMap::new()),
        )
        .expect("valid empty benchmark configuration"),
        InputSchedule::empty(),
        observation,
        ExecutionControls::default(),
    )
    .expect("valid benchmark execution request")
}

fn make_session(event_count: usize, enabled: bool) -> SyntheticSession {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    SyntheticSession {
        request: request(enabled, format!("bench-{id}")),
        event_count,
    }
}

fn measure_disabled(event_count: usize) -> BenchRow {
    let mut session = make_session(event_count, false);
    let start = Instant::now();
    session.execute(&mut NullSink).expect("synthetic execution");
    row("observation_disabled", "none", start.elapsed(), None, 0, 0)
}

fn measure_null_sink(event_count: usize) -> BenchRow {
    let mut session = make_session(event_count, true);
    let request = session.request.clone();
    let mut sink = BudgetedSink::new(
        NullSink,
        request.observation.clone(),
        request.run_id.clone(),
        request.machine_id.clone(),
        VersionStamp::from(MACHINE_VERSION),
    )
    .expect("valid budgeted sink");
    let start = Instant::now();
    session.execute(&mut sink).expect("synthetic execution");
    let receipt = sink.emit_receipt().expect("evidence receipt");
    let event_channel = receipt
        .channels
        .iter()
        .find(|channel| channel.channel == glassvm_core::EmissionChannel::NormalizedEvents)
        .expect("normalized event receipt channel");
    let mut row = row("observation_null_sink", "none", start.elapsed(), None, 0, 0);
    row.items = event_channel.item_count;
    row.logical_bytes = event_channel.logical_bytes;
    row
}

fn python_sink() -> (PythonCallbackConsumer, Py<PyAny>) {
    Python::attach(|py| {
        let module = PyModule::from_code(
            py,
            c"class Sink:\n    def __init__(self): self.calls = 0; self.items = 0; self.max_batch_items = 0\n    def emit_batch(self, batch):\n        self.calls += 1\n        self.items += len(batch)\n        self.max_batch_items = max(self.max_batch_items, len(batch))\n    def record_capability_outputs(self, values): pass\n    def record_capability_receipts(self, values): pass\n",
            c"python_emission_bench.py",
            c"python_emission_bench",
        )
        .expect("load benchmark Python consumer");
        let instance = module
            .getattr("Sink")
            .expect("benchmark Sink class")
            .call0()
            .expect("construct benchmark Sink")
            .unbind();
        let consumer = PythonCallbackConsumer::new(vec![(
            Some("benchmark-consumer".into()),
            instance.clone_ref(py),
        )]);
        (consumer, instance)
    })
}

fn measure_python(
    event_count: usize,
    policy_name: &str,
    policy: PythonBatchPolicy,
    with_recorder: bool,
) -> BenchRow {
    let mut session = make_session(event_count, true);
    let (consumer, python_instance) = python_sink();
    let recorder = with_recorder.then(|| {
        let path = unique_temp_path("python-plus-recorder");
        FileRecorderRequest {
            published_path: path,
            limits: recorder_limits(),
        }
    });
    let recorder_path = recorder.as_ref().map(|value| value.published_path.clone());
    let consumers: Vec<(Option<String>, Box<dyn PythonEmissionConsumer + Send>)> =
        vec![(Some("benchmark-consumer".into()), Box::new(consumer))];
    let request = session.request.clone();
    let start = Instant::now();
    let result = FileRunSession::run_with_sinks(
        &mut session,
        &request,
        recorder,
        VersionStamp::from(MACHINE_VERSION),
        consumers,
        policy,
    )
    .expect("shared file-run lifecycle");
    let elapsed = start.elapsed();
    let outcome = result.python_sink_delivery;
    assert_eq!(
        outcome.status,
        glassvm_recorder::PythonSinkDeliveryStatus::Complete,
        "benchmark Python consumer should receive the full run"
    );
    if with_recorder {
        assert!(result.published.is_some(), "combined run must publish");
    }
    let peak_callback_items = Python::attach(|py| {
        python_instance
            .bind(py)
            .getattr("max_batch_items")
            .expect("max batch field")
            .extract::<usize>()
            .expect("integer max batch field")
    });
    debug_assert!(peak_callback_items <= outcome.peak_batch_items);
    if let Some(path) = recorder_path {
        std::fs::remove_dir_all(path).expect("remove this benchmark's published run only");
    }
    BenchRow {
        mode: if with_recorder {
            "python_batched_plus_rust_recorder".into()
        } else {
            "python_batched_sink".into()
        },
        policy: policy_name.into(),
        elapsed,
        batches: outcome.batches_delivered,
        items: outcome.items_delivered,
        logical_bytes: outcome.logical_bytes_delivered,
        peak_buffer_items: outcome.peak_batch_items,
        peak_buffer_logical_bytes: outcome.peak_batch_logical_bytes,
        peak_rss_kib: process_peak_rss_kib(),
    }
}

fn measure_recorder(event_count: usize) -> BenchRow {
    let mut session = make_session(event_count, true);
    let path = unique_temp_path("rust-recorder");
    let request = session.request.clone();
    let start = Instant::now();
    let result = FileRunSession::run_with_sinks(
        &mut session,
        &request,
        Some(FileRecorderRequest {
            published_path: path.clone(),
            limits: recorder_limits(),
        }),
        VersionStamp::from(MACHINE_VERSION),
        Vec::new(),
        PythonBatchPolicy::default(),
    )
    .expect("shared file-run lifecycle");
    let elapsed = start.elapsed();
    assert!(
        result.published.is_some(),
        "reference recorder must publish"
    );
    let finalized = match result.recorder_finalization {
        glassvm_recorder::RecorderFinalizationOutcome::Finalized(receipt) => receipt,
        other => panic!("recorder did not finalize: {other:?}"),
    };
    let row = BenchRow {
        mode: "rust_reference_recorder".into(),
        policy: "none".into(),
        elapsed,
        batches: 0,
        items: finalized.record_count,
        logical_bytes: finalized.logical_bytes,
        peak_buffer_items: 0,
        peak_buffer_logical_bytes: 0,
        peak_rss_kib: process_peak_rss_kib(),
    };
    std::fs::remove_dir_all(path).expect("remove this benchmark's published run only");
    row
}

fn recorder_limits() -> RecorderLimits {
    RecorderLimits {
        max_block_logical_bytes: 8 * 1024 * 1024,
        max_segment_records: 100_000,
        max_segment_logical_bytes: 16 * 1024 * 1024,
        max_buffered_bytes_per_channel: 8 * 1024 * 1024,
    }
}

fn unique_temp_path(label: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "glassvm-python-consumer-bench-{label}-{}-{nonce}",
        std::process::id()
    ))
}

fn process_peak_rss_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        line.strip_prefix("VmHWM:")
            .and_then(|value| value.split_whitespace().next())
            .and_then(|value| value.parse().ok())
    })
}

fn row(
    mode: &str,
    policy: &str,
    elapsed: Duration,
    peak_buffer: Option<(usize, usize)>,
    batches: u64,
    items: u64,
) -> BenchRow {
    let (peak_buffer_items, peak_buffer_logical_bytes) = peak_buffer.unwrap_or_default();
    BenchRow {
        mode: mode.into(),
        policy: policy.into(),
        elapsed,
        batches,
        items,
        logical_bytes: 0,
        peak_buffer_items,
        peak_buffer_logical_bytes,
        peak_rss_kib: process_peak_rss_kib(),
    }
}

fn main() {
    Python::initialize();
    let event_count = std::env::var("GLASSVM_BENCH_EVENTS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1000usize);
    let repetitions = std::env::var("GLASSVM_BENCH_REPETITIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1usize);

    println!(
        "mode,policy,repetition,event_count,wall_ms,relative_to_disabled,batches,items,logical_bytes,peak_buffer_items,peak_buffer_logical_bytes,process_peak_rss_kib"
    );
    let policies = [
        (
            "items_1",
            PythonBatchPolicy {
                max_items: 1,
                max_logical_bytes: 64 * 1024 * 1024,
            },
        ),
        (
            "items_8",
            PythonBatchPolicy {
                max_items: 8,
                max_logical_bytes: 64 * 1024 * 1024,
            },
        ),
        (
            "items_32",
            PythonBatchPolicy {
                max_items: 32,
                max_logical_bytes: 64 * 1024 * 1024,
            },
        ),
        (
            "items_128",
            PythonBatchPolicy {
                max_items: 128,
                max_logical_bytes: 64 * 1024 * 1024,
            },
        ),
        (
            "byte_bounded_default",
            PythonBatchPolicy {
                max_items: usize::MAX,
                max_logical_bytes: glassvm_recorder::DEFAULT_PYTHON_BATCH_MAX_LOGICAL_BYTES,
            },
        ),
    ];

    for repetition in 0..repetitions {
        let baseline = measure_disabled(event_count);
        print_row(&baseline, repetition, event_count, baseline.elapsed);
        let null = measure_null_sink(event_count);
        print_row(&null, repetition, event_count, baseline.elapsed);
        let recorder = measure_recorder(event_count);
        print_row(&recorder, repetition, event_count, baseline.elapsed);
        for (name, policy) in policies {
            let python = measure_python(event_count, name, policy, false);
            print_row(&python, repetition, event_count, baseline.elapsed);
        }
        let combined = measure_python(event_count, "default", PythonBatchPolicy::default(), true);
        print_row(&combined, repetition, event_count, baseline.elapsed);
    }
}

fn print_row(row: &BenchRow, repetition: usize, event_count: usize, baseline: Duration) {
    let relative = if baseline.is_zero() {
        0.0
    } else {
        row.elapsed.as_secs_f64() / baseline.as_secs_f64()
    };
    println!(
        "{},{},{},{},{:.3},{:.3},{},{},{},{},{},{}",
        row.mode,
        row.policy,
        repetition,
        event_count,
        row.elapsed.as_secs_f64() * 1000.0,
        relative,
        row.batches,
        row.items,
        row.logical_bytes,
        row.peak_buffer_items,
        row.peak_buffer_logical_bytes,
        row.peak_rss_kib
            .map(|value| value.to_string())
            .unwrap_or_else(|| "".into())
    );
}
