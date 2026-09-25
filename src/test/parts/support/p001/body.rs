
#[derive(Clone)]
pub(crate) struct ChildProcessPlan {
    diagnostic_path: std::path::PathBuf,
    logical_root: String,
}

impl ChildProcessPlan {
    pub(crate) fn path(&self) -> &std::path::Path {
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

impl std::ops::Deref for ProcessWorkspace {
    type Target = std::path::Path;

    fn deref(&self) -> &Self::Target {
        self.plan.path()
    }
}

impl AsRef<std::path::Path> for ProcessWorkspace {
    fn as_ref(&self) -> &std::path::Path {
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
        destination: &std::path::Path,
    ) -> TestSupportResult<()> {
        ensure_workspace_owns_root(self.workspace, target)?;
        create_parent(&target.dir, link.as_path())?;
        let target_root = self.workspace.process_bridge().plan(target)?;
        std::os::unix::fs::symlink(destination, target_root.path().join(link.as_path()))
    }
}

pub(crate) fn validate_portable_evidence(
    fields: &[&str],
    diagnostic_paths: &[&std::path::Path],
) -> TestSupportResult<()> {
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
    for component in std::path::Path::new(value).components() {
        match component {
            std::path::Component::Normal(_) => {
                component_count = component_count
                    .checked_add(1)
                    .ok_or_else(|| invalid_input("test workspace path component count overflow"))?;
                if component_count > MAX_WORKSPACE_PATH_COMPONENTS {
                    return Err(invalid_input("test workspace path has too many components"));
                }
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err(invalid_input("test workspace path must not escape its typed root"));
            }
        }
    }
    if component_count == 0 {
        return Err(invalid_input("test workspace path must contain a logical component"));
    }
    Ok(())
}

fn create_parent(dir: &cap_std::fs::Dir, path: &std::path::Path) -> TestSupportResult<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    dir.create_dir_all(parent)
}

fn workspace_identity(logical_label: &str, diagnostic_host_path: &std::path::Path) -> blake3::Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(logical_label.as_bytes());
    hasher.update(&[0]);
    hasher.update(diagnostic_host_path.as_os_str().as_encoded_bytes());
    hasher.finalize()
}

#[cfg(unix)]
fn diagnostic_host_path(dir: &cap_std::fs::Dir) -> TestSupportResult<std::path::PathBuf> {
    use std::os::fd::AsRawFd;

    let descriptor_path = std::path::PathBuf::from(format!("/proc/self/fd/{}", dir.as_raw_fd()));
    std::fs::read_link(descriptor_path)
}

#[cfg(not(unix))]
fn diagnostic_host_path(_dir: &cap_std::fs::Dir) -> TestSupportResult<std::path::PathBuf> {
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
