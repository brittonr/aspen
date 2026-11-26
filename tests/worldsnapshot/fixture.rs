fn reference(label: &str) -> String {
    molten::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

fn command(args: &[&str]) -> FixtureResult<std::process::Output> {
    Ok(std::process::Command::new(env!("CARGO_BIN_EXE_molten")).args(args).output()?)
}

fn text(path: &std::path::Path) -> FixtureResult<&str> {
    path.to_str().ok_or_else(|| "fixture path is not UTF8".into())
}

fn succeeded(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

struct Fixture {
    workspace: test_support::ProcessWorkspace,
    root: molten::node_state::DirectoryRoot,
    signer: ed25519_dalek::SigningKey,
    key: String,
    image: molten::world_snapshot::LogicalRuntimeImage,
    descriptor: molten_core::world_snapshot::SnapshotDescriptor,
    destination_ref: String,
    current: molten::world_snapshot::LocalCurrentAdmission,
}

impl Fixture {
    fn new(label: &str) -> FixtureResult<Self> {
        let workspace = test_support::process_workspace(label)?;
        let node = workspace.join("node");
        let root = molten::node_state::DirectoryRoot::open(&node)?;
        root.create_layout()?;
        provision_identity(&root)?;
        let signer = ed25519_dalek::SigningKey::from_bytes(&[17; 32]); // disposable experimental fixture only
        let key = hex::encode(signer.verifying_key().to_bytes());
        let image = initial_image()?;
        let image_path = workspace.join("image.preserves");
        std::fs::write(&image_path, image.canonical_bytes()?)?;
        let descriptor_path = workspace.join("descriptor.preserves");
        let output = command(&[
            "world-snapshot", "capture", "--image", text(&image_path)?,
            "--state-root", text(&node)?, "--trusted-signer", &key,
            "--commit-ref", &reference("commit"), "--descriptor-out", text(&descriptor_path)?,
        ])?;
        succeeded(&output);
        let stdout = String::from_utf8(output.stdout)?;
        let destination_ref = stdout.lines()
            .find_map(|line| line.strip_prefix("destination_ref="))
            .ok_or("capture omitted observed destination")?.to_string();
        let (descriptor, _) = molten::world_snapshot::parse_canonical_snapshot_descriptor(&std::fs::read(descriptor_path)?)?;
        let current = molten::world_snapshot::LocalCurrentAdmission {
            policy_ref: image.policy_ref.clone(),
            capability_ref: reference("capability"),
            revocation_ref: reference("current revocation"),
            resource_ref: reference("fixed bounded resources"),
            adapter_ref: reference("local runtime adapter"),
            generation: 1,
            allowed: true,
            expires_unix_seconds: u64::MAX,
        };
        let fixture = Self { workspace, root, signer, key, image, descriptor, destination_ref, current };
        fixture.admit(&fixture.current)?;
        Ok(fixture)
    }

    fn admit(&self, current: &molten::world_snapshot::LocalCurrentAdmission) -> FixtureResult<()> {
        let descriptor_ref = molten::world_snapshot::canonical_snapshot_descriptor(&self.descriptor)?.artifact_ref;
        let grant = molten::world_snapshot::sign_local_admission(molten::world_snapshot::LocalAdmissionInput {
            signer: &self.signer,
            descriptor_ref: &descriptor_ref,
            profile_ref: self.descriptor.profile_ref.as_str(),
            cohort_ref: self.descriptor.cohort.cohort_ref.as_str(),
            destination_ref: &self.destination_ref,
            current: current.clone(),
        })?;
        self.root.write(
            &molten::node_state::RelativePath::parse("storage/world-snapshot/current.json")?,
            &serde_json::to_vec(current)?,
        )?;
        self.root.write(&molten::node_state::RelativePath::parse("storage/world-snapshot/admission.json")?, &grant)?;
        Ok(())
    }

    fn restore(&self, receipt: &std::path::Path) -> FixtureResult<std::process::Output> {
        command(&[
            "world-snapshot", "restore", "--descriptor",
            text(&self.workspace.join("descriptor.preserves"))?, "--destination",
            text(&self.workspace.join("descriptor.preserves"))?, "--state-root",
            text(&self.workspace.join("node"))?, "--trusted-signer", &self.key,
            "--receipt-out", text(receipt)?,
        ])
    }
}

fn provision_identity(root: &molten::node_state::DirectoryRoot) -> FixtureResult<()> {
    for name in ["config.preserves", "identity.preserves"] {
        let bytes = molten::preserves_rail::canonical_bytes(&molten::preserves_rail::record(
            "explicit-experimental-snapshot-node-v1",
            vec![molten::preserves_rail::string(name)],
        ))?;
        root.write(&molten::node_state::RelativePath::parse(name)?, &bytes)?;
    }
    Ok(())
}

fn initial_image() -> FixtureResult<molten::world_snapshot::LogicalRuntimeImage> {
    let mut runtime = molten::runtime::RuntimeState::new(91);
    let value = molten::runtime::RuntimeValue::string("pending")?;
    let mut history = Vec::with_capacity(4096);
    for step in [
        molten::runtime::RuntimeStep::Send { from: "producer".into(), to: "consumer".into(), body: value.clone() },
        molten::runtime::RuntimeStep::Assert { actor: "producer".into(), value: value.clone() },
        molten::runtime::RuntimeStep::Observe { actor: "consumer".into(), pattern: value },
        molten::runtime::RuntimeStep::Clock { actor: "producer".into() },
        molten::runtime::RuntimeStep::Random { actor: "producer".into(), upper: 97 },
    ] {
        let events = runtime.apply_step(&step);
        if events.len() > 4095 - history.len() {
            return Err("fixture history exceeds runtime image entry bound".into());
        }
        history.extend(events.iter().map(|event| event.to_value()));
    }
    let unresolved = runtime.begin_effect_for_step(&molten::runtime::RuntimeStep::Clock { actor: "producer".into() })?
        .ok_or("fixture clock effect was not prepared")?;
    history.push(unresolved.to_value());
    Ok(molten::world_snapshot::LogicalRuntimeImage {
        snapshot: runtime.snapshot(), history, unresolved_effects: vec![unresolved],
        policy_ref: reference("selected policy"),
    })
}
