use std::path::Path;

const MAX_BOUNDARY_FILES: usize = 1_024;
const MAX_BOUNDARY_FILE_BYTES: u64 = 1_048_576;
const NODE_ROOT: &str = "src/node";
const CORE_ROOT: &str = "crates/molten-core/src/fabric_simulation";
const RUST_EXTENSION: &str = "rs";
const FORBIDDEN_NODE_TERMS: [&str; 4] = [
    "TransactionalKeyValue",
    "ReplicatedLog",
    "DistributedScheduler",
    "fabric_simulation::reference",
];
const FORBIDDEN_CORE_TERMS: [&str; 8] = [
    "std::fs",
    "std::net",
    "std::process",
    "std::env",
    "SystemTime",
    "Instant::now",
    "TcpStream",
    "UdpSocket",
];

type BoundaryResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceFile {
    path: String,
    source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BoundaryViolation {
    path: String,
    term: String,
}

fn find_boundary_violations(files: &[SourceFile], forbidden_terms: &[&str]) -> Vec<BoundaryViolation> {
    let mut violations = Vec::new();
    for file in files {
        for term in forbidden_terms {
            if file.source.contains(term) {
                violations.push(BoundaryViolation {
                    path: file.path.clone(),
                    term: (*term).to_string(),
                });
            }
        }
    }
    violations
}

fn read_bounded_rust_sources(root: &Path) -> BoundaryResult<Vec<SourceFile>> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = pending.pop() {
        let mut entries = std::fs::read_dir(&path)?.collect::<std::result::Result<Vec<_>, _>>()?;
        entries.sort_by_key(std::fs::DirEntry::path);
        for entry in entries {
            let entry_path = entry.path();
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                pending.push(entry_path);
                continue;
            }
            if entry_path.extension().and_then(|extension| extension.to_str()) != Some(RUST_EXTENSION) {
                continue;
            }
            if metadata.len() > MAX_BOUNDARY_FILE_BYTES {
                return Err(format!(
                    "boundary source {} is {} bytes; maximum is {MAX_BOUNDARY_FILE_BYTES}",
                    entry_path.display(),
                    metadata.len()
                )
                .into());
            }
            if files.len() >= MAX_BOUNDARY_FILES {
                return Err(format!("boundary source count exceeds {MAX_BOUNDARY_FILES}").into());
            }
            files.push(SourceFile {
                path: entry_path.display().to_string(),
                source: std::fs::read_to_string(entry_path)?,
            });
        }
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

// r[verify molten.fabric_simulation.fabric_sufficiency]
// r[verify molten.fabric_simulation.final_validation]
#[test]
fn node_core_has_no_reference_service_domain_branch() -> BoundaryResult<()> {
    let sources = read_bounded_rust_sources(Path::new(NODE_ROOT))?;
    let violations = find_boundary_violations(&sources, &FORBIDDEN_NODE_TERMS);

    assert!(!sources.is_empty());
    assert!(sources.len() <= MAX_BOUNDARY_FILES);
    assert!(violations.is_empty(), "node-core reference-service leakage: {violations:?}");
    Ok(())
}

// r[verify molten.fabric_simulation.same_core]
// r[verify molten.fabric_simulation.final_validation]
#[test]
fn pure_simulation_core_has_no_ambient_io_dependency() -> BoundaryResult<()> {
    let sources = read_bounded_rust_sources(Path::new(CORE_ROOT))?;
    let violations = find_boundary_violations(&sources, &FORBIDDEN_CORE_TERMS);

    assert!(!sources.is_empty());
    assert!(sources.len() <= MAX_BOUNDARY_FILES);
    assert!(violations.is_empty(), "pure simulation core ambient dependency: {violations:?}");
    Ok(())
}

// r[verify molten.fabric_simulation.fabric_sufficiency]
// r[verify molten.fabric_simulation.final_validation]
#[test]
fn boundary_checker_reports_domain_and_ambient_negative_fixtures() {
    let fixtures = vec![
        SourceFile {
            path: "src/node/leak.rs".to_string(),
            source: "match service { TransactionalKeyValue => run() }".to_string(),
        },
        SourceFile {
            path: "crates/molten-core/src/fabric_simulation/leak.rs".to_string(),
            source: "std::fs::read(path)".to_string(),
        },
    ];

    let node_violations = find_boundary_violations(&fixtures, &FORBIDDEN_NODE_TERMS);
    let core_violations = find_boundary_violations(&fixtures, &FORBIDDEN_CORE_TERMS);

    assert_eq!(node_violations.len(), 1);
    assert_eq!(node_violations[0].term, "TransactionalKeyValue");
    assert_eq!(core_violations.len(), 1);
    assert_eq!(core_violations[0].term, "std::fs");
}
