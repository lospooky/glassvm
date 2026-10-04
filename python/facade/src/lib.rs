//! Generic Python facade for entry-point-discovered GlassVM machine bundles.
//!
//! The base wheel contains no machine implementation. Bundle wheels register
//! provider entry points in the `glassvm.machine_bundles` group and own the
//! machine-specific preparation and execution implementation.

use std::sync::atomic::{AtomicU64, Ordering};

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use serde_json::Value;

const PYTHON_BUNDLE_ENTRY_POINT_GROUP: &str = "glassvm.machine_bundles";
const PYTHON_BUNDLE_PROVIDER_CONTRACT: &str = "glassvm.python_bundle";
const PYTHON_BUNDLE_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion {
    major: 1,
    minor: 0,
    patch: 0,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProtocolVersion {
    major: u16,
    minor: u16,
    patch: u16,
}

fn parse_protocol_version(value: &Value, entry_point: &str) -> Result<ProtocolVersion, String> {
    let object = value.as_object().ok_or_else(|| {
        format!(
            "entry point {entry_point:?} provider has invalid protocol_version: expected an object"
        )
    })?;
    for field in object.keys() {
        if !matches!(field.as_str(), "major" | "minor" | "patch") {
            return Err(format!(
                "entry point {entry_point:?} provider has invalid protocol_version: unsupported field {field:?}"
            ));
        }
    }
    let component = |field: &str| -> Result<u16, String> {
        let value = object.get(field).and_then(Value::as_u64).ok_or_else(|| {
            format!(
                "entry point {entry_point:?} provider has invalid protocol_version: {field:?} must be an unsigned integer"
            )
        })?;
        u16::try_from(value).map_err(|_| {
            format!(
                "entry point {entry_point:?} provider has invalid protocol_version: {field:?} is out of range"
            )
        })
    };
    Ok(ProtocolVersion {
        major: component("major")?,
        minor: component("minor")?,
        patch: component("patch")?,
    })
}

fn validate_bundle_provider(
    mut provider: Value,
    entry_point: &str,
) -> Result<(String, Value), String> {
    let object = provider
        .as_object_mut()
        .ok_or_else(|| "bundle provider must return a JSON object".to_owned())?;
    let protocol = object
        .get("protocol")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!("entry point {entry_point:?} provider is missing string field \"protocol\"")
        })?;
    if protocol != PYTHON_BUNDLE_PROVIDER_CONTRACT {
        return Err(format!(
            "entry point {entry_point:?} returned unsupported provider protocol {protocol:?}"
        ));
    }
    let protocol_version = object.get("protocol_version").ok_or_else(|| {
        format!("entry point {entry_point:?} provider is missing object field \"protocol_version\"")
    })?;
    let protocol_version = parse_protocol_version(protocol_version, entry_point)?;
    if protocol_version != PYTHON_BUNDLE_PROTOCOL_VERSION {
        return Err(format!(
            "entry point {entry_point:?} returned unsupported provider protocol version {}.{}.{}",
            protocol_version.major, protocol_version.minor, protocol_version.patch
        ));
    }

    const REQUIRED_STRING_FIELDS: &[&str] = &[
        "machine_id",
        "distribution",
        "module",
        "prepare_function",
        "execute_function",
        "bundle_version",
    ];
    for field in REQUIRED_STRING_FIELDS {
        if object
            .get(*field)
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err(format!(
                "entry point {entry_point:?} provider is missing string field {field:?}"
            ));
        }
    }

    for field in object.keys() {
        if !matches!(
            field.as_str(),
            "protocol"
                | "protocol_version"
                | "machine_id"
                | "distribution"
                | "module"
                | "prepare_function"
                | "execute_function"
                | "bundle_version"
        ) {
            return Err(format!(
                "entry point {entry_point:?} provider contains unsupported metadata field {field:?}"
            ));
        }
    }
    let machine_id = object
        .get("machine_id")
        .and_then(Value::as_str)
        .expect("machine_id was validated")
        .to_owned();
    object.insert(
        "entry_point".to_owned(),
        Value::String(entry_point.to_owned()),
    );
    Ok((machine_id, provider))
}

fn provider_string(value: &Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("bundle provider field {field:?} is not a string"))
}

fn discover_bundle_values(py: Python<'_>) -> PyResult<Vec<Value>> {
    let metadata = py.import("importlib.metadata")?;
    let all_entries = metadata.call_method0("entry_points")?;
    let kwargs = PyDict::new(py);
    kwargs.set_item("group", PYTHON_BUNDLE_ENTRY_POINT_GROUP)?;
    let entries = if all_entries.hasattr("select")? {
        all_entries.call_method("select", (), Some(&kwargs))?
    } else {
        all_entries
    };

    let mut providers = Vec::new();
    let mut machine_ids = std::collections::BTreeSet::new();
    for item in entries.try_iter()? {
        let entry = item?;
        let entry_point = entry.getattr("name")?.extract::<String>()?;
        let provider = entry.call_method0("load")?;
        let encoded = provider.call0()?.extract::<String>()?;
        let value: Value = serde_json::from_str(&encoded).map_err(|error| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "bundle entry point {entry_point:?} returned invalid JSON: {error}"
            ))
        })?;
        let (machine_id, value) = validate_bundle_provider(value, &entry_point)
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
        if !machine_ids.insert(machine_id.clone()) {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                "multiple bundle entry points advertise machine_id {machine_id:?}"
            )));
        }
        providers.push((machine_id, value));
    }
    providers.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(providers
        .into_iter()
        .map(|(_, provider)| provider)
        .collect())
}

/// Discover separately installed machine-bundle wheels through Python entry
/// points. Providers return metadata only; this boundary does not exchange
/// Rust trait objects or invoke Python during machine execution.
#[pyfunction]
fn discover_bundles(py: Python<'_>) -> PyResult<String> {
    serde_json::to_string(&discover_bundle_values(py)?).map_err(|error| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("cannot encode bundle metadata: {error}"))
    })
}

#[derive(Debug, Clone)]
struct PythonBundleProvider {
    protocol: String,
    protocol_version: ProtocolVersion,
    entry_point: String,
    machine_id: String,
    distribution: String,
    module: String,
    prepare_function: String,
    execute_function: String,
    bundle_version: String,
}

fn parse_bundle_providers(values: Vec<Value>) -> Result<Vec<PythonBundleProvider>, String> {
    values
        .into_iter()
        .map(|value| {
            Ok(PythonBundleProvider {
                protocol: provider_string(&value, "protocol")?,
                protocol_version: parse_protocol_version(
                    value
                        .get("protocol_version")
                        .ok_or_else(|| "bundle provider is missing protocol_version".to_owned())?,
                    "parsed provider",
                )?,
                entry_point: provider_string(&value, "entry_point")?,
                machine_id: provider_string(&value, "machine_id")?,
                distribution: provider_string(&value, "distribution")?,
                module: provider_string(&value, "module")?,
                prepare_function: provider_string(&value, "prepare_function")?,
                execute_function: provider_string(&value, "execute_function")?,
                bundle_version: provider_string(&value, "bundle_version")?,
            })
        })
        .collect()
}

fn json_argument(
    py: Python<'_>,
    value: Option<&Bound<'_, PyAny>>,
    field: &str,
) -> PyResult<String> {
    let Some(value) = value else {
        return Ok("null".to_owned());
    };
    py.import("json")?
        .call_method1("dumps", (value,))?
        .extract::<String>()
        .map_err(|error| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "{field} must be JSON-compatible: {error}"
            ))
        })
}

#[pyclass]
struct Runtime {
    providers: Vec<PythonBundleProvider>,
}

#[pyclass]
struct PreparedRun {
    provider: PythonBundleProvider,
    /// Provider-owned preparation state. This is deliberately an in-process
    /// Python object, not a serialization, persistence, or replay format.
    prepared: Py<PyAny>,
}

static NEXT_PYTHON_RUN_ID: AtomicU64 = AtomicU64::new(1);

#[pymethods]
impl Runtime {
    #[staticmethod]
    fn discover(py: Python<'_>) -> PyResult<Self> {
        let providers = parse_bundle_providers(discover_bundle_values(py)?).map_err(|error| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("invalid bundle provider: {error}"))
        })?;
        Ok(Self { providers })
    }

    fn bundles(&self) -> PyResult<String> {
        let values = self
            .providers
            .iter()
            .map(|provider| {
                serde_json::json!({
                    "protocol": provider.protocol,
                    "protocol_version": {
                        "major": provider.protocol_version.major,
                        "minor": provider.protocol_version.minor,
                        "patch": provider.protocol_version.patch,
                    },
                    "entry_point": provider.entry_point,
                    "machine_id": provider.machine_id,
                    "distribution": provider.distribution,
                    "module": provider.module,
                    "prepare_function": provider.prepare_function,
                    "execute_function": provider.execute_function,
                    "bundle_version": provider.bundle_version,
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_string(&values)
            .map_err(|error| pyo3::exceptions::PyRuntimeError::new_err(error.to_string()))
    }

    #[pyo3(signature = (machine_id, artifact, configuration = None, input_schedule = None, execution_controls = None, observation = None))]
    fn prepare(
        &self,
        py: Python<'_>,
        machine_id: &str,
        artifact: &[u8],
        configuration: Option<&Bound<'_, PyAny>>,
        input_schedule: Option<&Bound<'_, PyAny>>,
        execution_controls: Option<&Bound<'_, PyAny>>,
        observation: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PreparedRun> {
        let provider = self
            .providers
            .iter()
            .find(|provider| provider.machine_id == machine_id)
            .cloned()
            .ok_or_else(|| {
                pyo3::exceptions::PyLookupError::new_err(format!(
                    "machine bundle {machine_id:?} is not installed; install with `uv add \"glassvm[{machine_id}]\"`"
                ))
            })?;
        let module = py.import(&provider.module)?;
        let prepare = module.getattr(&provider.prepare_function)?;
        let run_id = format!(
            "python-{}-{}",
            provider.machine_id,
            NEXT_PYTHON_RUN_ID.fetch_add(1, Ordering::Relaxed)
        );
        let prepared = prepare.call1((
            PyBytes::new(py, artifact),
            run_id,
            json_argument(py, configuration, "configuration")?,
            json_argument(py, input_schedule, "input_schedule")?,
            json_argument(py, execution_controls, "execution_controls")?,
            json_argument(py, observation, "observation")?,
        ))?;
        Ok(PreparedRun {
            provider,
            prepared: prepared.unbind(),
        })
    }
}

#[pymethods]
impl PreparedRun {
    #[pyo3(signature = (output_path))]
    fn execute(&self, py: Python<'_>, output_path: &str) -> PyResult<Py<PyAny>> {
        let module = py.import(&self.provider.module)?;
        let execute = module.getattr(&self.provider.execute_function)?;
        let result = execute.call1((self.prepared.bind(py), output_path))?;
        if !result.is_instance_of::<PyDict>() {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "bundle execute_prepared must return a structured mapping",
            ));
        }
        Ok(result.unbind())
    }
}

#[pymodule]
fn glassvm_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Runtime>()?;
    m.add_class::<PreparedRun>()?;
    m.add_function(wrap_pyfunction!(discover_bundles, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        PYTHON_BUNDLE_PROTOCOL_VERSION, PYTHON_BUNDLE_PROVIDER_CONTRACT, parse_bundle_providers,
        validate_bundle_provider,
    };

    #[test]
    fn canonical_provider_requires_both_canonical_operations() {
        let (_, provider) = validate_bundle_provider(
            json!({
                "protocol": "glassvm.python_bundle",
                "protocol_version": {"major": 1, "minor": 0, "patch": 0},
                "machine_id": "chip8",
                "distribution": "glassvm-machine-chip8",
                "module": "glassvm_py_chip8",
                "prepare_function": "prepare_run",
                "execute_function": "execute_prepared",
                "bundle_version": "0.1.0"
            }),
            "chip8",
        )
        .expect("valid canonical provider");
        let parsed = parse_bundle_providers(vec![provider]).expect("parse provider");
        assert_eq!(parsed[0].protocol, PYTHON_BUNDLE_PROVIDER_CONTRACT);
        assert_eq!(parsed[0].protocol_version, PYTHON_BUNDLE_PROTOCOL_VERSION);
        assert_eq!(parsed[0].prepare_function, "prepare_run");
        assert_eq!(parsed[0].execute_function, "execute_prepared");
    }

    #[test]
    fn zero_providers_is_valid() {
        assert!(parse_bundle_providers(Vec::new()).unwrap().is_empty());
    }

    #[test]
    fn wrong_protocol_version_is_rejected() {
        let error = validate_bundle_provider(
            json!({
                "protocol": "glassvm.python_bundle",
                "protocol_version": {"major": 2, "minor": 0, "patch": 0},
                "machine_id": "chip8",
                "distribution": "glassvm-machine-chip8",
                "module": "glassvm_py_chip8",
                "prepare_function": "prepare_run",
                "execute_function": "execute_prepared",
                "bundle_version": "0.1.0"
            }),
            "chip8",
        )
        .expect_err("unsupported protocol version must fail closed");
        assert!(error.contains("unsupported provider protocol version"));
    }

    #[test]
    fn legacy_protocol_name_is_rejected_without_fallback() {
        let error = validate_bundle_provider(
            json!({
                "protocol": format!("{PYTHON_BUNDLE_PROVIDER_CONTRACT}.v1"),
                "protocol_version": {"major": 1, "minor": 0, "patch": 0},
                "machine_id": "chip8",
                "distribution": "glassvm-machine-chip8",
                "module": "glassvm_py_chip8",
                "prepare_function": "prepare_run",
                "execute_function": "execute_prepared",
                "bundle_version": "0.1.0"
            }),
            "chip8",
        )
        .expect_err("legacy protocol names must not dispatch");
        assert!(error.contains("unsupported provider protocol"));
    }

    #[test]
    fn canonical_provider_rejects_missing_execute_operation() {
        let error = validate_bundle_provider(
            json!({
                "protocol": "glassvm.python_bundle",
                "protocol_version": {"major": 1, "minor": 0, "patch": 0},
                "machine_id": "chip8",
                "distribution": "glassvm-machine-chip8",
                "module": "glassvm_py_chip8",
                "prepare_function": "prepare_run",
                "bundle_version": "0.1.0"
            }),
            "chip8",
        )
        .expect_err("missing execute operation must fail");
        assert!(error.contains("execute_function"));
    }
}
