use std::marker::PhantomData;
use std::ops::Deref;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

const MAX_WORKSPACE_LABEL_BYTES: usize = 64;
const MAX_WORKSPACE_PATH_COMPONENTS: usize = 32;
const WORKSPACE_ROLE_COUNT: usize = 7;
const MAX_REASONABLE_WORKSPACE_BOUND: usize = 1_024;
const TEST_LIST_ARGUMENT: &str = "--list";

const _: () = assert!(MAX_WORKSPACE_LABEL_BYTES <= MAX_REASONABLE_WORKSPACE_BOUND);
const _: () = assert!(MAX_WORKSPACE_PATH_COMPONENTS <= MAX_REASONABLE_WORKSPACE_BOUND);

pub(crate) type TestSupportResult<T> = std::io::Result<T>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceRole {
    State,
    Input,
    Output,
    Transport,
    Ledger,
    Cache,
    Adversarial,
}

impl WorkspaceRole {
    const ALL: [Self; WORKSPACE_ROLE_COUNT] = [
        Self::State,
        Self::Input,
        Self::Output,
        Self::Transport,
        Self::Ledger,
        Self::Cache,
        Self::Adversarial,
    ];

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::State => "state",
            Self::Input => "input",
            Self::Output => "output",
            Self::Transport => "transport",
            Self::Ledger => "ledger",
            Self::Cache => "cache",
            Self::Adversarial => "adversarial",
        }
    }
}

pub(crate) trait WorkspaceRoleMarker {
    const ROLE: WorkspaceRole;
}

macro_rules! workspace_role_marker {
    ($name:ident, $role:expr) => {
        #[derive(Debug)]
        pub(crate) enum $name {}

        impl WorkspaceRoleMarker for $name {
            const ROLE: WorkspaceRole = $role;
        }
    };
}

workspace_role_marker!(StateRole, WorkspaceRole::State);
workspace_role_marker!(InputRole, WorkspaceRole::Input);
workspace_role_marker!(OutputRole, WorkspaceRole::Output);
workspace_role_marker!(TransportRole, WorkspaceRole::Transport);
workspace_role_marker!(LedgerRole, WorkspaceRole::Ledger);
workspace_role_marker!(CacheRole, WorkspaceRole::Cache);
workspace_role_marker!(AdversarialRole, WorkspaceRole::Adversarial);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspacePath {
    relative: PathBuf,
}

impl WorkspacePath {
    pub(crate) fn parse(value: &str) -> TestSupportResult<Self> {
        validate_workspace_path(value)?;
        Ok(Self {
            relative: PathBuf::from(value),
        })
    }

    fn as_path(&self) -> &Path {
        &self.relative
    }

    pub(crate) fn logical_name(&self) -> String {
        self.relative.to_string_lossy().into_owned()
    }
}

struct WorkspaceInner {
    temp_dir: cap_tempfile::TempDir,
    workspace_id: blake3::Hash,
    logical_label: String,
    diagnostic_host_path: PathBuf,
}

impl std::fmt::Debug for WorkspaceInner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkspaceInner")
            .field("workspace_id", &self.workspace_id.to_hex())
            .field("logical_label", &self.logical_label)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub(crate) struct TestWorkspace {
    inner: Arc<WorkspaceInner>,
}

impl std::fmt::Debug for TestWorkspace {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TestWorkspace")
            .field("workspace_id", &self.inner.workspace_id.to_hex())
            .field("logical_label", &self.inner.logical_label)
            .finish_non_exhaustive()
    }
}

impl TestWorkspace {
    pub(crate) fn new(logical_label: &str) -> TestSupportResult<Self> {
        // r[impl molten.testing.cap_std_workspace]
        validate_logical_label(logical_label)?;
        let temp_dir = cap_tempfile::tempdir(cap_tempfile::ambient_authority())?;
        let diagnostic_host_path = diagnostic_host_path(&temp_dir)?;
        let workspace_id = workspace_identity(logical_label, &diagnostic_host_path);
        for role in WorkspaceRole::ALL {
            temp_dir.create_dir_all(role.as_str())?;
        }
        Ok(Self {
            inner: Arc::new(WorkspaceInner {
                temp_dir,
                workspace_id,
                logical_label: logical_label.to_string(),
                diagnostic_host_path,
            }),
        })
    }

    pub(crate) fn logical_label(&self) -> &str {
        &self.inner.logical_label
    }

    pub(crate) fn state(&self) -> TestSupportResult<TestRoot<StateRole>> {
        self.root()
    }

    pub(crate) fn input(&self) -> TestSupportResult<TestRoot<InputRole>> {
        self.root()
    }

    pub(crate) fn output(&self) -> TestSupportResult<TestRoot<OutputRole>> {
        self.root()
    }

    pub(crate) fn transport(&self) -> TestSupportResult<TestRoot<TransportRole>> {
        self.root()
    }

    pub(crate) fn ledger(&self) -> TestSupportResult<TestRoot<LedgerRole>> {
        self.root()
    }

    pub(crate) fn cache(&self) -> TestSupportResult<TestRoot<CacheRole>> {
        self.root()
    }

    pub(crate) fn adversarial(&self) -> TestSupportResult<TestRoot<AdversarialRole>> {
        self.root()
    }

    pub(crate) fn process_bridge(&self) -> ProcessPathBridge<'_> {
        ProcessPathBridge { workspace: self }
    }

    pub(crate) fn adversarial_setup(&self) -> AdversarialSetup<'_> {
        AdversarialSetup { workspace: self }
    }

    pub(crate) fn export_selected<R: WorkspaceRoleMarker>(
        &self,
        source_root: &TestRoot<R>,
        destination_root: &TestRoot<OutputRole>,
        plan: &ArtifactExportPlan,
    ) -> TestSupportResult<ArtifactExportReceipt> {
        // r[impl molten.testing.cap_std_cleanup]
        ensure_workspace_owns_root(self, source_root)?;
        let bytes = source_root.read(&plan.source)?;
        destination_root.write(&plan.destination, &bytes)?;
        Ok(ArtifactExportReceipt {
            artifact_label: plan.artifact_label.clone(),
            source_logical_path: format!("{}/{}", R::ROLE.as_str(), plan.source.logical_name()),
            destination_logical_path: format!("{}/{}", OutputRole::ROLE.as_str(), plan.destination.logical_name()),
            content_ref: format!("blake3:{}", blake3::hash(&bytes).to_hex()),
        })
    }

    fn root<R: WorkspaceRoleMarker>(&self) -> TestSupportResult<TestRoot<R>> {
        // r[impl molten.testing.cap_std_subroots]
        let dir = self.inner.temp_dir.open_dir(R::ROLE.as_str())?;
        Ok(TestRoot {
            dir,
            inner: Arc::clone(&self.inner),
            marker: PhantomData,
        })
    }
}

pub(crate) struct TestRoot<R: WorkspaceRoleMarker> {
    dir: cap_std::fs::Dir,
    inner: Arc<WorkspaceInner>,
    marker: PhantomData<R>,
}

impl<R: WorkspaceRoleMarker> std::fmt::Debug for TestRoot<R> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TestRoot")
            .field("workspace_id", &self.inner.workspace_id.to_hex())
            .field("role", &R::ROLE)
            .finish_non_exhaustive()
    }
}

impl<R: WorkspaceRoleMarker> TestRoot<R> {
    pub(crate) fn dir(&self) -> &cap_std::fs::Dir {
        &self.dir
    }

    pub(crate) fn logical_label(&self) -> String {
        format!("{}/{}", self.inner.logical_label, R::ROLE.as_str())
    }

    pub(crate) fn write(&self, path: &WorkspacePath, bytes: &[u8]) -> TestSupportResult<()> {
        create_parent(&self.dir, path.as_path())?;
        self.dir.write(path.as_path(), bytes)
    }

    pub(crate) fn read(&self, path: &WorkspacePath) -> TestSupportResult<Vec<u8>> {
        self.dir.read(path.as_path())
    }

    pub(crate) fn try_exists(&self, path: &WorkspacePath) -> TestSupportResult<bool> {
        self.dir.try_exists(path.as_path())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactExportPlan {
    artifact_label: String,
    source: WorkspacePath,
    destination: WorkspacePath,
}

impl ArtifactExportPlan {
    pub(crate) fn new(artifact_label: &str, source: &str, destination: &str) -> TestSupportResult<Self> {
        plan_artifact_export(artifact_label, source, destination)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactExportReceipt {
    pub(crate) artifact_label: String,
    pub(crate) source_logical_path: String,
    pub(crate) destination_logical_path: String,
    pub(crate) content_ref: String,
}

pub(crate) struct ProcessPathBridge<'a> {
    workspace: &'a TestWorkspace,
}

impl ProcessPathBridge<'_> {
    pub(crate) fn plan<R: WorkspaceRoleMarker>(&self, root: &TestRoot<R>) -> TestSupportResult<ChildProcessPlan> {
        // r[impl molten.testing.cap_std_process_bridge]
        ensure_workspace_owns_root(self.workspace, root)?;
        Ok(ChildProcessPlan {
            diagnostic_path: self.workspace.inner.diagnostic_host_path.join(R::ROLE.as_str()),
            logical_root: root.logical_label(),
        })
    }
}

#[derive(Clone)]
pub(crate) struct ChildProcessPlan {
    diagnostic_path: PathBuf,
    logical_root: String,
}

impl ChildProcessPlan {
    pub(crate) fn path(&self) -> &Path {
        &self.diagnostic_path
    }

    pub(crate) fn logical_root(&self) -> &str {
        &self.logical_root
    }
}

impl std::fmt::Debug for ChildProcessPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChildProcessPlan")
            .field("logical_root", &self.logical_root)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub(crate) struct ProcessWorkspace {
    _workspace: TestWorkspace,
    plan: ChildProcessPlan,
}

impl ProcessWorkspace {
    pub(crate) fn new(logical_label: &str) -> TestSupportResult<Self> {
        let workspace = TestWorkspace::new(logical_label)?;
        let state = workspace.state()?;
        let plan = workspace.process_bridge().plan(&state)?;
        Ok(Self {
            _workspace: workspace,
            plan,
        })
    }

    pub(crate) fn logical_root(&self) -> &str {
        self.plan.logical_root()
    }
}

impl std::fmt::Debug for ProcessWorkspace {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProcessWorkspace")
            .field("logical_root", &self.plan.logical_root)
            .finish_non_exhaustive()
    }
}

impl Deref for ProcessWorkspace {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.plan.path()
    }
}

impl AsRef<Path> for ProcessWorkspace {
    fn as_ref(&self) -> &Path {
        self.plan.path()
    }
}

pub(crate) fn process_workspace(logical_label: &str) -> TestSupportResult<ProcessWorkspace> {
    ProcessWorkspace::new(logical_label)
}

pub(crate) fn cleanup_stale_molten_temp_dirs() {
    // Compatibility no-op for suites not yet migrated. Broad ambient-prefix cleanup is
    // intentionally removed.
}

impl AsRef<std::ffi::OsStr> for ProcessWorkspace {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.plan.path().as_os_str()
    }
}

pub(crate) struct AdversarialSetup<'a> {
    workspace: &'a TestWorkspace,
}

impl AdversarialSetup<'_> {
    pub(crate) fn corrupt<R: WorkspaceRoleMarker>(
        &self,
        target: &TestRoot<R>,
        path: &WorkspacePath,
        bytes: &[u8],
    ) -> TestSupportResult<()> {
        ensure_workspace_owns_root(self.workspace, target)?;
        target.write(path, bytes)
    }

    pub(crate) fn remove<R: WorkspaceRoleMarker>(
        &self,
        target: &TestRoot<R>,
        path: &WorkspacePath,
    ) -> TestSupportResult<()> {
        ensure_workspace_owns_root(self.workspace, target)?;
        target.dir.remove_file(path.as_path())
    }

    pub(crate) fn replace<R: WorkspaceRoleMarker>(
        &self,
        target: &TestRoot<R>,
        path: &WorkspacePath,
        bytes: &[u8],
    ) -> TestSupportResult<()> {
        ensure_workspace_owns_root(self.workspace, target)?;
        match target.dir.symlink_metadata(path.as_path()) {
            Ok(metadata) if metadata.is_dir() => target.dir.remove_dir_all(path.as_path())?,
            Ok(_) => target.dir.remove_file(path.as_path())?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        target.write(path, bytes)
    }

    #[cfg(unix)]
    pub(crate) fn set_mode<R: WorkspaceRoleMarker>(
        &self,
        target: &TestRoot<R>,
        path: &WorkspacePath,
        mode: u32,
    ) -> TestSupportResult<()> {
        use cap_std::fs::PermissionsExt;

        ensure_workspace_owns_root(self.workspace, target)?;
        target.dir.set_permissions(path.as_path(), cap_std::fs::Permissions::from_mode(mode))
    }

    #[cfg(unix)]
    pub(crate) fn symlink_to_host<R: WorkspaceRoleMarker>(
        &self,
        target: &TestRoot<R>,
        link: &WorkspacePath,
        destination: &Path,
    ) -> TestSupportResult<()> {
        ensure_workspace_owns_root(self.workspace, target)?;
        create_parent(&target.dir, link.as_path())?;
        let target_root = self.workspace.process_bridge().plan(target)?;
        std::os::unix::fs::symlink(destination, target_root.path().join(link.as_path()))
    }
}

pub(crate) fn validate_portable_evidence(fields: &[&str], diagnostic_paths: &[&Path]) -> TestSupportResult<()> {
    for field in fields {
        for diagnostic_path in diagnostic_paths {
            let rendered = diagnostic_path.to_string_lossy();
            if !rendered.is_empty() && field.contains(rendered.as_ref()) {
                return Err(invalid_input("canonical test evidence contains a temporary host path"));
            }
        }
    }
    Ok(())
}

fn plan_artifact_export(
    artifact_label: &str,
    source: &str,
    destination: &str,
) -> TestSupportResult<ArtifactExportPlan> {
    validate_logical_label(artifact_label)?;
    Ok(ArtifactExportPlan {
        artifact_label: artifact_label.to_string(),
        source: WorkspacePath::parse(source)?,
        destination: WorkspacePath::parse(destination)?,
    })
}

fn ensure_workspace_owns_root<R: WorkspaceRoleMarker>(
    workspace: &TestWorkspace,
    root: &TestRoot<R>,
) -> TestSupportResult<()> {
    if workspace.inner.workspace_id != root.inner.workspace_id {
        return Err(permission_denied("test root belongs to a different workspace"));
    }
    Ok(())
}

fn validate_logical_label(label: &str) -> TestSupportResult<()> {
    if label.is_empty() {
        return Err(invalid_input("test workspace logical label cannot be empty"));
    }
    if label.len() > MAX_WORKSPACE_LABEL_BYTES {
        return Err(invalid_input("test workspace logical label is too long"));
    }
    if !label.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_') {
        return Err(invalid_input("test workspace logical label contains unsupported characters"));
    }
    Ok(())
}

fn validate_workspace_path(value: &str) -> TestSupportResult<()> {
    if value.is_empty() {
        return Err(invalid_input("test workspace path cannot be empty"));
    }
    if value.contains('\\') || value.contains("://") {
        return Err(invalid_input("test workspace path must be a portable relative path"));
    }
    let mut component_count = 0usize;
    for component in Path::new(value).components() {
        match component {
            Component::Normal(_) => {
                component_count = component_count
                    .checked_add(1)
                    .ok_or_else(|| invalid_input("test workspace path component count overflow"))?;
                if component_count > MAX_WORKSPACE_PATH_COMPONENTS {
                    return Err(invalid_input("test workspace path has too many components"));
                }
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(invalid_input("test workspace path must not escape its typed root"));
            }
        }
    }
    if component_count == 0 {
        return Err(invalid_input("test workspace path must contain a logical component"));
    }
    Ok(())
}

fn create_parent(dir: &cap_std::fs::Dir, path: &Path) -> TestSupportResult<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    dir.create_dir_all(parent)
}

fn workspace_identity(logical_label: &str, diagnostic_host_path: &Path) -> blake3::Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(logical_label.as_bytes());
    hasher.update(&[0]);
    hasher.update(diagnostic_host_path.as_os_str().as_encoded_bytes());
    hasher.finalize()
}

#[cfg(unix)]
fn diagnostic_host_path(dir: &cap_std::fs::Dir) -> TestSupportResult<PathBuf> {
    use std::os::fd::AsRawFd;

    let descriptor_path = PathBuf::from(format!("/proc/self/fd/{}", dir.as_raw_fd()));
    std::fs::read_link(descriptor_path)
}

#[cfg(not(unix))]
fn diagnostic_host_path(_dir: &cap_std::fs::Dir) -> TestSupportResult<PathBuf> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "test child-process path bridge is not implemented for this host",
    ))
}

fn invalid_input(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message)
}

fn permission_denied(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::PermissionDenied, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONCURRENT_WORKSPACE_COUNT: usize = 8;
    const NO_ACCESS_MODE: u32 = 0o000;
    const OWNER_READ_WRITE_MODE: u32 = 0o600;

    #[test]
    fn concurrent_workspaces_and_typed_subroots_are_isolated() {
        // r[verify molten.testing.cap_std_workspace]
        // r[verify molten.testing.cap_std_subroots]
        cleanup_stale_molten_temp_dirs();
        let handles = (0..CONCURRENT_WORKSPACE_COUNT)
            .map(|index| {
                std::thread::spawn(move || {
                    let workspace = TestWorkspace::new(&format!("concurrent_{index}")).expect("workspace");
                    assert_eq!(workspace.logical_label(), format!("concurrent_{index}"));
                    let state = workspace.state().expect("state root");
                    let input = workspace.input().expect("input root");
                    let output = workspace.output().expect("output root");
                    let transport = workspace.transport().expect("transport root");
                    let ledger = workspace.ledger().expect("ledger root");
                    let cache = workspace.cache().expect("cache root");
                    let adversarial = workspace.adversarial().expect("adversarial root");
                    for root in [
                        input.dir(),
                        transport.dir(),
                        ledger.dir(),
                        cache.dir(),
                        adversarial.dir(),
                    ] {
                        assert!(root.metadata(".").expect("role root metadata").is_dir());
                    }
                    let path = WorkspacePath::parse("shared/value.bin").expect("workspace path");
                    let bytes = format!("state-{index}").into_bytes();
                    state.write(&path, &bytes).expect("state write");
                    output.write(&path, b"output").expect("output write");
                    assert_eq!(state.read(&path).expect("state read"), bytes);
                    assert_eq!(output.read(&path).expect("output read"), b"output");
                    workspace.inner.workspace_id
                })
            })
            .collect::<Vec<_>>();
        let mut ids = handles.into_iter().map(|handle| handle.join().expect("workspace thread")).collect::<Vec<_>>();
        ids.sort_by_key(blake3::Hash::to_hex);
        ids.dedup();
        assert_eq!(ids.len(), CONCURRENT_WORKSPACE_COUNT);
    }

    #[tokio::test]
    async fn async_workspace_survives_yield_and_exports_selected_artifact() {
        // r[verify molten.testing.cap_std_validation]
        let source_workspace = TestWorkspace::new("async_source").expect("source workspace");
        let destination_workspace = TestWorkspace::new("async_destination").expect("destination workspace");
        let state = source_workspace.state().expect("state root");
        let output = destination_workspace.output().expect("output root");
        let source_path = WorkspacePath::parse("receipts/run.preserves").expect("source path");
        state.write(&source_path, b"receipt").expect("source write");
        tokio::task::yield_now().await;
        let plan = ArtifactExportPlan::new("run_receipt", "receipts/run.preserves", "selected/run.preserves")
            .expect("export plan");
        let receipt = source_workspace.export_selected(&state, &output, &plan).expect("selected export");
        assert_eq!(receipt.artifact_label, "run_receipt");
        assert!(receipt.content_ref.starts_with("blake3:"));
        assert_eq!(
            output
                .read(&WorkspacePath::parse("selected/run.preserves").expect("destination path"))
                .expect("exported bytes"),
            b"receipt"
        );
    }

    #[test]
    fn process_bridge_runs_child_without_putting_host_path_in_evidence() {
        // r[verify molten.testing.cap_std_process_bridge]
        let workspace = TestWorkspace::new("child_process").expect("workspace");
        let state = workspace.state().expect("state root");
        let plan = workspace.process_bridge().plan(&state).expect("process plan");
        let output = std::process::Command::new(std::env::current_exe().expect("current test executable"))
            .arg(TEST_LIST_ARGUMENT)
            .current_dir(plan.path())
            .output()
            .expect("run child test executable");
        assert!(output.status.success());
        validate_portable_evidence(&[plan.logical_root()], &[plan.path()]).expect("portable logical evidence");
        assert!(!format!("{plan:?}").contains(&plan.path().to_string_lossy().into_owned()));
    }

    #[test]
    fn workspace_drop_cleans_only_its_owned_root() {
        // r[verify molten.testing.cap_std_cleanup]
        let process_workspace = ProcessWorkspace::new("cleanup_owned").expect("process workspace");
        let state_path = process_workspace.to_path_buf();
        std::fs::write(state_path.join("owned.bin"), b"owned").expect("owned fixture");
        assert!(state_path.exists());
        drop(process_workspace);
        assert!(!state_path.exists());
    }

    #[test]
    fn wrong_workspace_and_invalid_export_are_denied() {
        // r[verify molten.testing.cap_std_validation]
        let first = TestWorkspace::new("wrong_root_first").expect("first workspace");
        let second = TestWorkspace::new("wrong_root_second").expect("second workspace");
        let second_state = second.state().expect("second state");
        let first_output = first.output().expect("first output");
        let bridge_error = first.process_bridge().plan(&second_state).expect_err("cross-workspace bridge denied");
        assert_eq!(bridge_error.kind(), std::io::ErrorKind::PermissionDenied);
        let plan = ArtifactExportPlan::new("missing", "missing.bin", "selected/missing.bin").expect("export plan");
        let export_error = first
            .export_selected(&second_state, &first_output, &plan)
            .expect_err("cross-workspace export denied");
        assert_eq!(export_error.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(ArtifactExportPlan::new("escape", "../outside", "selected/outside").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn adversarial_symlink_corruption_replacement_and_mode_stay_in_test_shell() {
        // r[verify molten.testing.cap_std_validation]
        let target_workspace = TestWorkspace::new("adversarial_target").expect("target workspace");
        let outside_workspace = ProcessWorkspace::new("adversarial_outside").expect("outside workspace");
        let state = target_workspace.state().expect("target state");
        let setup = target_workspace.adversarial_setup();
        let value_path = WorkspacePath::parse("values/value.bin").expect("value path");
        state.write(&value_path, b"original").expect("initial write");
        setup.corrupt(&state, &value_path, b"corrupt").expect("corrupt fixture");
        assert_eq!(state.read(&value_path).expect("corrupt read"), b"corrupt");
        setup.replace(&state, &value_path, b"replacement").expect("replace fixture");
        assert_eq!(state.read(&value_path).expect("replacement read"), b"replacement");
        setup.set_mode(&state, &value_path, NO_ACCESS_MODE).expect("deny mode");
        setup.set_mode(&state, &value_path, OWNER_READ_WRITE_MODE).expect("restore mode");
        setup.remove(&state, &value_path).expect("remove fixture");
        assert!(!state.try_exists(&value_path).expect("removed state"));
        let link_path = WorkspacePath::parse("values/outside-link").expect("link path");
        setup
            .symlink_to_host(&state, &link_path, outside_workspace.as_ref())
            .expect("outside symlink fixture");
        assert!(state.read(&link_path).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn cleanup_does_not_follow_replaced_symlink_or_remove_external_workspace() {
        // r[verify molten.testing.cap_std_validation]
        let outside = ProcessWorkspace::new("cleanup_external").expect("outside workspace");
        std::fs::write(outside.join("marker.bin"), b"external").expect("outside marker");
        let target = ProcessWorkspace::new("cleanup_replaced").expect("target workspace");
        let target_path = target.to_path_buf();
        std::fs::remove_dir(&target_path).expect("remove empty target state");
        std::os::unix::fs::symlink(&*outside, &target_path).expect("replace state with symlink");
        drop(target);
        assert_eq!(std::fs::read(outside.join("marker.bin")).expect("outside marker survives"), b"external");
    }

    #[test]
    fn canonical_evidence_rejects_temporary_host_path_leakage() {
        // r[verify molten.testing.cap_std_validation]
        let process_workspace = ProcessWorkspace::new("portable_evidence").expect("process workspace");
        validate_portable_evidence(&[process_workspace.logical_root()], &[process_workspace.as_ref()])
            .expect("logical evidence passes");
        let leaked = format!("state-root={}", process_workspace.display());
        let error = validate_portable_evidence(&[&leaked], &[process_workspace.as_ref()])
            .expect_err("host path leakage denied");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(!format!("{process_workspace:?}").contains(&process_workspace.display().to_string()));
    }

    #[test]
    fn logical_labels_and_paths_reject_ambient_or_traversing_inputs() {
        assert!(TestWorkspace::new("../escape").is_err());
        assert!(WorkspacePath::parse("/absolute").is_err());
        assert!(WorkspacePath::parse("../parent").is_err());
        assert!(WorkspacePath::parse(r"C:\outside").is_err());
        assert!(WorkspacePath::parse("https://example.invalid/value").is_err());
    }
}
