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
    relative: std::path::PathBuf,
}

impl WorkspacePath {
    pub(crate) fn parse(value: &str) -> TestSupportResult<Self> {
        validate_workspace_path(value)?;
        Ok(Self {
            relative: std::path::PathBuf::from(value),
        })
    }

    fn as_path(&self) -> &std::path::Path {
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
    diagnostic_host_path: std::path::PathBuf,
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
    inner: std::sync::Arc<WorkspaceInner>,
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
            inner: std::sync::Arc::new(WorkspaceInner {
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
            inner: std::sync::Arc::clone(&self.inner),
            marker: std::marker::PhantomData,
        })
    }
}

pub(crate) struct TestRoot<R: WorkspaceRoleMarker> {
    dir: cap_std::fs::Dir,
    inner: std::sync::Arc<WorkspaceInner>,
    marker: std::marker::PhantomData<R>,
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
