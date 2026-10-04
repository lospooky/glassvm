use std::{
    fs,
    path::{Path, PathBuf},
};

const FORBIDDEN_SURFACES: &[&str] = &[
    "ObservationPlan",
    "RunConfig",
    "TraceChunk",
    "ReplayManifest",
    "ReplayPackage",
    "ReplayReceipt",
    "ObservableAdapter",
    "native_event_adapter",
    "observable_adapter",
    "record_function",
    "record_run",
    "body_provider",
    "CapabilityBlob",
    "glassvm_episode",
    "create_execution_with_prepared_observation",
    "glassvm.python_bundle.v1",
    "fn create_execution(\n",
];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("core test must be nested beneath crates/core")
        .to_path_buf()
}

fn source_files(root: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            source_files(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "rs" || extension == "toml" || extension == "py")
        {
            files.push(path);
        }
    }
}

#[test]
fn publication_sources_exclude_removed_contract_surfaces() {
    let root = repository_root();
    let mut files = Vec::new();
    for source_root in [
        root.join("crates").join("core").join("src"),
        root.join("crates").join("normalizer-contract").join("src"),
        root.join("crates").join("query").join("src"),
        root.join("crates").join("recorder").join("src"),
        root.join("crates").join("registry").join("src"),
        root.join("python").join("facade").join("src"),
    ] {
        source_files(&source_root, &mut files);
    }
    files.extend([
        root.join("Cargo.toml"),
        root.join("crates/core/Cargo.toml"),
        root.join("crates/normalizer-contract/Cargo.toml"),
        root.join("crates/query/Cargo.toml"),
        root.join("crates/recorder/Cargo.toml"),
        root.join("crates/registry/Cargo.toml"),
        root.join("python/facade/Cargo.toml"),
        root.join("python/facade/pyproject.toml"),
    ]);

    let mut violations = Vec::new();
    for path in files {
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        for forbidden in FORBIDDEN_SURFACES {
            if contents.contains(forbidden) {
                violations.push(format!("{} contains {forbidden:?}", path.display()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "publication source contains removed contract surfaces:\n{}",
        violations.join("\n")
    );
}
