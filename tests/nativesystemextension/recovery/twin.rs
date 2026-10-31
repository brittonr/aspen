use crate::support::*;

pub(super) struct Live {
    pub root: molten::node_state::DirectoryRoot,
    pub bundle: molten::node_service_host::codec::Bundle,
    pub bundle_ref: String,
    pub definitions: molten::harness::composition::GroupDefinitions,
    workspace: crate::test_support::ProcessWorkspace,
    node: std::path::PathBuf,
    is_initialized: bool,
    operator: super::authority::Operator,
}

impl Live {
    /// `trust_path` and `cohort_path` are captured by the calling test shell from its explicit
    /// environment.
    pub(super) fn open(label: &str, trust_path: &std::ffi::OsStr, cohort_path: &std::ffi::OsStr) -> TestResult<Self> {
        let workspace = crate::test_support::process_workspace(label).or_fail("owned process workspace")?;
        let root = molten::node_state::DirectoryRoot::open_existing(workspace.as_ref())?;
        root.create_layout()?;
        let trust_bytes = std::fs::read(trust_path)?;
        let trust: serde_json::Value = serde_json::from_slice(&trust_bytes)?;
        let output = trust.get("output").and_then(serde_json::Value::as_str).or_fail("exact signed Nix output")?;
        let node = std::path::PathBuf::from(output).join("bin/molten");
        root.write(&molten::node_state::RelativePath::parse("source-gate-trust.json")?, &trust_bytes)?;
        let cohort = molten::node_state::DirectoryRoot::open_existing(std::path::Path::new(cohort_path))?;
        let admission = cohort.read(&molten::node_state::RelativePath::parse("admission.preserves")?, 16_777_216)?;
        let bundle_ref = molten::preserves_rail::content_ref_from_bytes(&admission);
        let bundle = molten::node_service_host::codec::decode(&admission, &bundle_ref)?;
        root.write(&molten::node_state::RelativePath::parse("admission.preserves")?, &admission)?;
        copy_artifacts(std::path::Path::new(cohort_path), &root, &bundle)?;
        let definitions = molten::harness::composition::group_definitions()?;
        let declared = bundle
            .services
            .iter()
            .find_map(|service| {
                service
                    .manifest
                    .recovery_group
                    .as_ref()
                    .filter(|group| group.group_id == definitions.group.group_id)
            })
            .or_fail("shared recovery declaration")?;
        let mut expected = definitions.group.clone();
        expected.elapsed_source_ref = declared.elapsed_source_ref.clone();
        assert_eq!(
            *declared, expected,
            "live cohort must consume the shared composition, with an actual elapsed source"
        );
        for service in &bundle.services {
            assert!(
                service.manifest.resources.max_restart_attempts >= 8,
                "native budget must not mask lost group history"
            );
        }
        let operator = super::authority::Operator::provision(&root)?;
        Ok(Self {
            root,
            bundle,
            bundle_ref,
            definitions,
            workspace,
            node,
            operator,
            is_initialized: false,
        })
    }

    pub(super) fn execute(
        &mut self,
        name: &str,
        input: preserves::IOValue,
        tick: u64,
    ) -> TestResult<(bool, Option<preserves::IOValue>)> {
        self.authorize(name, &input, tick)?;
        let (success, output) = self.invoke(name, tick, None)?;
        let result = if success {
            Some(value(&output, "node-service-host-receipt-v1")?)
        } else {
            None
        };
        Ok((success, result))
    }

    pub(super) fn require(
        &mut self,
        name: &str,
        input: preserves::IOValue,
        tick: u64,
    ) -> TestResult<preserves::IOValue> {
        let (success, result) = self.execute(name, input, tick)?;
        if !success {
            return Err(format!("actual public node {name} denied").into());
        }
        result.or_fail("actual canonical operation receipt")
    }

    pub(super) fn authorize(&mut self, name: &str, input: &preserves::IOValue, tick: u64) -> TestResult<()> {
        self.root.write(
            &molten::node_state::RelativePath::parse("operation.preserves")?,
            &molten::preserves_rail::canonical_bytes(input)?,
        )?;
        let (success, output) = self.invoke(name, tick, Some("--preview-load"))?;
        if !success {
            return Err(format!("genuine source/bundle load preview denied: {output}").into());
        }
        let load = request(&value(&output, "node-service-host-operation-request-v1")?)?;
        self.operator.authorize(&self.root, &self.bundle_ref, &load)?;
        let (success, output) = self.invoke(name, tick, Some("--preview"))?;
        if !success {
            return Err(format!("load-authorized operation preview denied: {output}").into());
        }
        let operation = request(&value(&output, "node-service-host-operation-request-v1")?)?;
        self.operator.authorize(&self.root, &self.bundle_ref, &operation)?;
        // The preview consumed its genuine load nonce; actual execution needs a fresh invocation.
        self.operator.authorize(&self.root, &self.bundle_ref, &load)
    }

    fn invoke(&self, name: &str, tick: u64, mode: Option<&str>) -> TestResult<(bool, String)> {
        let mut command = std::process::Command::new(&self.node);
        command
            .args(["node", "service-host", name, "--state-root"])
            .arg(AsRef::<std::path::Path>::as_ref(&self.workspace))
            .args([
                "--admission",
                "admission.preserves",
                "--logical-tick",
                &tick.to_string(),
                "--input",
                "operation.preserves",
            ]);
        if let Some(mode) = mode {
            command.arg(mode);
        }
        super::processes::run(command)
    }

    pub(super) fn initialize_node(&mut self) -> TestResult<()> {
        if !self.is_initialized {
            let mut init = std::process::Command::new(&self.node);
            init.args(["node", "init", "--state-root"])
                .arg(AsRef::<std::path::Path>::as_ref(&self.workspace))
                .args(["--node-id", "node:service-recovery"]);
            let (success, output) = super::processes::run(init)?;
            if !success {
                return Err(format!("actual node init denied: {output}").into());
            }
            let mut run = std::process::Command::new(&self.node);
            run.args(["node", "run", "--state-root"]).arg(AsRef::<std::path::Path>::as_ref(&self.workspace));
            let (success, output) = super::processes::run(run)?;
            if !success {
                return Err(format!("actual initial node startup denied: {output}").into());
            }
            self.is_initialized = true;
        }
        Ok(())
    }

    pub(super) fn crash_serving_node(&mut self, tick: u64) -> TestResult<preserves::IOValue> {
        self.initialize_node()?;
        self.authorize("status", &super::wire::operation("status", Vec::new()), tick)?;
        self.root.write(
            &molten::node_state::RelativePath::parse("service-host-startup")?,
            &molten::preserves_rail::canonical_bytes(&molten::preserves_rail::record(
                "node-service-host-startup-v1",
                vec![
                    molten::preserves_rail::string("admission.preserves"),
                    molten::preserves_rail::u64_value(tick),
                ],
            ))?,
        )?;
        let mut command = std::process::Command::new(&self.node);
        command
            .args(["node", "serve", "--state-root"])
            .arg(AsRef::<std::path::Path>::as_ref(&self.workspace))
            .args([
                "--live-iroh",
                "--live-max-events",
                "1",
                "--live-event-timeout-ms",
                "60000",
                "--max-requests-per-tick",
                "1",
                "--live-ticket-out",
            ])
            .arg(self.workspace.join("live-ticket.preserves"));
        let mut child = super::processes::Child::spawn(command)?;
        let ready = child.await_record("node-service-host-ready-v1")?;
        child.kill()?;
        Ok(ready)
    }
}

/// The authoring bound for one callback ELF; node-state reads cap far lower, so artifacts stream.
const MAX_ARTIFACT_BYTES: u64 = 536_870_912;

fn copy_artifacts(
    source: &std::path::Path,
    target: &molten::node_state::DirectoryRoot,
    bundle: &molten::node_service_host::codec::Bundle,
) -> TestResult<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut paths = std::collections::BTreeSet::new();
    for service in bundle.services.iter().chain(bundle.replacements.iter().map(|replacement| &replacement.service)) {
        if !paths.insert(&service.artifact_path) {
            continue;
        }
        let path = molten::node_state::RelativePath::parse(&service.artifact_path)?;
        if let Some(parent) = path.as_path().parent().filter(|parent| !parent.as_os_str().is_empty()) {
            target.create_dir_all(&molten::node_state::RelativePath::parse(
                parent.to_str().or_fail("artifact parent UTF-8")?,
            )?)?;
        }
        let mut input = std::fs::File::open(source.join(path.as_path()))?;
        let mut file = target.open_database_file(&path)?;
        file.set_len(0)?;
        if stream_hashed(&mut input, &mut file)? != service.executable.executable_bytes_ref {
            return Err("external cohort native ELF differs from its actual admitted identity".into());
        }
        file.set_permissions(std::fs::Permissions::from_mode(0o500))?;
        file.sync_all()?;
    }
    Ok(())
}

/// Copies at most `MAX_ARTIFACT_BYTES`, returning the canonical BLAKE3 content ref of what was
/// copied.
fn stream_hashed(input: &mut std::fs::File, output: &mut std::fs::File) -> TestResult<String> {
    use std::io::Read;
    use std::io::Write;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0_u8; 65_536];
    let mut total = 0_u64;
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            return Ok(molten::preserves_rail::content_ref_from_blake3_hash(hasher.finalize()));
        }
        total = total.checked_add(u64::try_from(count)?).or_fail("artifact size overflow")?;
        if total > MAX_ARTIFACT_BYTES {
            return Err("external cohort native ELF exceeds its authoring bound".into());
        }
        hasher.update(&buffer[..count]);
        output.write_all(&buffer[..count])?;
    }
}

pub(super) fn value(output: &str, tag: &str) -> TestResult<preserves::IOValue> {
    output
        .lines()
        .filter_map(|line| molten::preserves_rail::parse_text(line).ok())
        .find(|value| value.collect_simple_record(tag, None).is_some())
        .ok_or_else(|| format!("actual process did not emit canonical {tag}: {output}").into())
}

fn request(value: &preserves::IOValue) -> TestResult<molten::node_service_hosting::OperationRequest> {
    let fields = molten::preserves_rail::simple_record_fields(value, "node-service-host-operation-request-v1", 5)?;
    Ok(molten::node_service_hosting::OperationRequest {
        operation: super::wire::text(&fields[0])?,
        subject: super::wire::text(&fields[1])?,
        logical_tick: super::wire::number(&fields[2])?,
        state_ref: super::wire::text(&fields[3])?,
        input_ref: super::wire::text(&fields[4])?,
    })
}
