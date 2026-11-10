mod activation;
mod execution;
mod mechanisms;
mod runtime;
mod service;
mod transport;

pub use transport::sizing::uncommitted_frame_budget;

#[allow(dead_code)]
pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
pub const ADMISSION: &str = "admission.preserves";
pub const RUNTIME: &str = "collaboration-runtime.preserves";
/// The node reads its effect configuration from this exact selector path; a different name is
/// silently ignored and every approved effect stays unrouted.
pub const EFFECTS: &str = "service-host-effects";
pub const SERVICE: &str = "collaboration-native";
/// The admitted recovery group activated before effect selection and hosted readiness.
pub const GROUP_ID: &str = "collaboration-native-group";
/// The admitted cohort size bound: three voters, so each node observes at most two peers.
pub const MAX_PEERS: usize = 2;

pub struct Node {
    pub path: std::path::PathBuf,
    pub root: molten::node_state::DirectoryRoot,
    pub bundle: molten::node_service_host::codec::Bundle,
    pub bundle_ref: String,
    pub runtime: molten::collaboration_scope::native_runtime::Input,
    pub effects: molten::node_service_host::effects::config::Configuration,
}

pub struct Cohort {
    pub nodes: Vec<Node>,
    pub signed_main: std::path::PathBuf,
    reservations: Vec<std::net::UdpSocket>,
}

impl Cohort {
    /// Release immediately before starting the three independent native host processes.
    pub fn release_endpoints(&mut self) {
        self.reservations.clear();
    }
}

/// Authors data only. Native lifecycle callbacks, checkpointing and effects activation
/// must run through the externally signed main executable in the owning harness.
pub fn prepare(
    workspace: &std::path::Path,
    signed_main: &std::path::Path,
    signed_scope: &std::path::Path,
    source_trust: &std::path::Path,
    max_frame_bytes: Option<u64>,
) -> Result<Cohort> {
    let cohort = transport::Cohort::reserve(max_frame_bytes)?;
    let mut nodes = Vec::with_capacity(3);
    for index in 0..3 {
        let source = admit(Input {
            workspace,
            signed_main,
            signed_scope,
            source_trust,
            index,
        })?;
        let service = service::build(&source, &cohort.mechanisms)?;
        let bundle = molten::node_service_host::codec::Bundle {
            services: vec![service],
            replacements: Vec::new(),
        };
        let bytes = molten::node_service_host::codec::encode(&bundle)?;
        let bundle_ref = molten::preserves_rail::content_ref_from_bytes(&bytes);
        source.root.write_new(&molten::node_state::RelativePath::parse(ADMISSION)?, &bytes)?;
        let (runtime, effects) = runtime::build(&source, &bundle.services[0], &cohort, index)?;
        nodes.push(Node {
            path: source.path,
            root: source.root,
            bundle,
            bundle_ref,
            runtime,
            effects,
        });
    }
    Ok(Cohort {
        nodes,
        signed_main: signed_main.to_owned(),
        reservations: cohort.reservations,
    })
}

pub(super) fn reference(label: &str) -> String {
    molten::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

use std::io::Read;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;

pub(super) struct Input<'a> {
    pub workspace: &'a std::path::Path,
    pub signed_main: &'a std::path::Path,
    pub signed_scope: &'a std::path::Path,
    pub source_trust: &'a std::path::Path,
    pub index: usize,
}

pub(super) struct Source {
    pub path: std::path::PathBuf,
    pub root: molten::node_state::DirectoryRoot,
    pub validation: molten::node_daemon::SelectedNodeOutputValidation,
    pub bytes_ref: String,
    pub artifact_ref: String,
    pub provenance_ref: String,
    pub closure_ref: String,
    pub kind_ref: String,
    pub target_ref: String,
    pub materialization_ref: String,
}

pub(super) fn admit(input: Input<'_>) -> Result<Source> {
    let name = format!("collaboration-node-{}", input.index);
    let path = input.workspace.join(&name);
    std::fs::create_dir(&path)?;
    init(input.signed_main, &path, input.index)?;
    let root = molten::node_state::DirectoryRoot::open_existing(&path)?;
    // Initial startup verifies the independently signed selection before it admits anything, so
    // the trust record must be installed after init creates the root and before run reads it.
    let (output, trust) = selection(input.source_trust)?;
    root.write_new(&molten::node_state::RelativePath::parse("source-gate-trust.json")?, &trust)?;
    run(input.signed_main, &path, input.index)?;
    root.create_layout()?;
    for name in ["artifacts", "evidence"] {
        root.create_dir_all(&molten::node_state::RelativePath::parse(name)?)?;
    }
    if input.signed_main != std::path::Path::new(&output).join("bin/molten")
        || input.signed_scope != std::path::Path::new(&output).join("bin/molten-collaboration-scope")
    {
        return Err("both actual executables must belong to the independently signed selected output".into());
    }
    let validation = molten::node_daemon::verify_selected_node_output_with_root(&root)?;
    let (binding, bytes_ref, closure_ref) = artifact(&root, &output, &validation, input.signed_scope)?;
    let provenance_ref = record(&root, "signed-collaboration-native-provenance-v1", &[
        &output,
        &validation.binding_ref,
        &validation.selected_output_validation_ref,
        "bin/molten-collaboration-scope",
        &bytes_ref,
        &closure_ref,
    ])?;
    let kind_ref = record(&root, "observed-native-kind-v1", &["ELF64-little-endian"])?;
    let target_ref = record(&root, "observed-native-target-v1", &["linux", "x86-64"])?;
    let materialization_ref = record(&root, "owned-native-materialization-v1", &[
        &bytes_ref,
        "artifacts/collaboration-scope",
        &binding,
        &provenance_ref,
    ])?;
    let artifact_ref = record(&root, "collaboration-native-artifact-v1", &[
        &bytes_ref,
        &kind_ref,
        &target_ref,
        &closure_ref,
        &provenance_ref,
    ])?;
    Ok(Source {
        path: input.workspace.join(name),
        root,
        validation,
        bytes_ref,
        artifact_ref,
        provenance_ref,
        closure_ref,
        kind_ref,
        target_ref,
        materialization_ref,
    })
}

/// The signed binary owns node identity; the harness never authors it.
fn init(signed_main: &std::path::Path, path: &std::path::Path, index: usize) -> Result<()> {
    let id = format!("node:collaboration-{index}");
    let init = std::process::Command::new(signed_main)
        .args(["node", "init", "--state-root"])
        .arg(path)
        .args(["--node-id", &id])
        .output()?;
    if !init.status.success() {
        return Err(format!(
            "node {index} init denied: {}\n{}",
            String::from_utf8_lossy(&init.stdout),
            String::from_utf8_lossy(&init.stderr)
        )
        .into());
    }
    Ok(())
}

/// Initial startup admits the installed signed selection before any service-host operation.
fn run(signed_main: &std::path::Path, path: &std::path::Path, index: usize) -> Result<()> {
    let run = std::process::Command::new(signed_main).args(["node", "run", "--state-root"]).arg(path).output()?;
    if !run.status.success() {
        return Err(format!(
            "node {index} initial startup denied: {}\n{}",
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        )
        .into());
    }
    Ok(())
}

/// The selected output and its trust record, read from the independently provisioned file.
fn selection(trust_path: &std::path::Path) -> Result<(String, Vec<u8>)> {
    let trust = read(trust_path, 16_384)?;
    let value: serde_json::Value = serde_json::from_slice(&trust)?;
    let output = value.get("output").and_then(serde_json::Value::as_str).ok_or("source trust output missing")?;
    Ok((output.to_owned(), trust))
}

/// The measured artifact bytes and the retained dependency closure the binding names.
fn artifact(
    root: &molten::node_state::DirectoryRoot,
    output: &str,
    validation: &molten::node_daemon::SelectedNodeOutputValidation,
    signed_scope: &std::path::Path,
) -> Result<(String, String, String)> {
    let binding_bytes = read(&std::path::Path::new(output).join("binding.json"), 65_536)?;
    if molten::preserves_rail::content_ref_from_bytes(&binding_bytes) != validation.binding_ref {
        return Err("independently validated source binding changed".into());
    }
    let binding: molten::core_api::startup_binding::Binding = serde_json::from_slice(&binding_bytes)?;
    let bytes = read(signed_scope, 536_870_912)?;
    if bytes.len() < 20
        || &bytes[..4] != b"\x7fELF"
        || bytes[4] != 2
        || bytes[5] != 1
        || u16::from_le_bytes([bytes[18], bytes[19]]) != 62
    {
        return Err("actual scope executable must be an ELF64 little-endian x86-64 artifact".into());
    }
    let bytes_ref = molten::preserves_rail::content_ref_from_bytes(&bytes);
    let mut target =
        root.open_database_file(&molten::node_state::RelativePath::parse("artifacts/collaboration-scope")?)?;
    target.write_all(&bytes)?;
    target.set_permissions(std::fs::Permissions::from_mode(0o500))?;
    target.sync_all()?;
    Ok((bytes.len().to_string(), bytes_ref, binding.dependencies_ref))
}

pub(super) fn record(root: &molten::node_state::DirectoryRoot, tag: &'static str, fields: &[&str]) -> Result<String> {
    retain(
        root,
        &molten::preserves_rail::record(tag, fields.iter().map(molten::preserves_rail::string).collect()),
    )
}

pub(super) fn retain(root: &molten::node_state::DirectoryRoot, value: &preserves::IOValue) -> Result<String> {
    let bytes = molten::preserves_rail::canonical_bytes(value)?;
    let reference = molten::preserves_rail::content_ref_from_bytes(&bytes);
    let path = molten::node_state::RelativePath::parse(&format!("evidence/{}.preserves", &reference[7..]))?;
    if root.entry_kind(&path)?.is_none() {
        root.write_new(&path, &bytes)?;
    } else if root.read(&path, 1_048_576)? != bytes {
        return Err("retained evidence collision".into());
    }
    Ok(reference)
}

fn read(path: &std::path::Path, maximum: u64) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err("producer input is not a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err("producer input exceeds bounded size".into());
    }
    Ok(bytes)
}
