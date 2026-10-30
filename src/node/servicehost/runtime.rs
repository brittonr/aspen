pub struct Publisher {
    pub values: crate::system_extension::SharedCallbackValuePort,
}

impl crate::fabric_execution::ExecutionOutputPublisher for Publisher {
    fn publish(
        &mut self,
        _operation_ref: &str,
        stream: &crate::fabric_execution::RetainedStream,
    ) -> Result<crate::fabric_execution::PublishedStream, crate::fabric_execution::OutputPublicationError> {
        let value = crate::system_extension::NativeCallbackValue {
            value_ref: crate::preserves_rail::content_ref_from_bytes(&stream.retained_bytes),
            bytes: stream.retained_bytes.clone(),
        };
        let receipt = self
            .values
            .lock()
            .map_err(|_| crate::fabric_execution::OutputPublicationError::Unavailable)?
            .publish(&value, 1_048_576)
            .map_err(|_| crate::fabric_execution::OutputPublicationError::Storage)?;
        Ok(crate::fabric_execution::PublishedStream {
            content_ref: receipt.value_ref,
            publication_receipt_ref: receipt.publication_ref,
        })
    }
}

pub type Port = crate::fabric_execution::Adapter<Publisher>;
pub type Native = crate::node_service_hosting::native::Runtime<Port, crate::system_extension::Durable>;

/// Held capabilities keep execution bound to measured inode and selected workspace.
/// This local-live pilot does not assert executable trust or sandboxing.
pub struct Materialized {
    pub runtime: Native,
    pub groups: std::collections::BTreeMap<String, crate::system_extension::RecoveryGroupDeclaration>,
    pub files: Vec<crate::node_state::Executable>,
    pub directories: Vec<crate::node_state::Workspace>,
}

struct Prepared {
    admission: crate::system_extension::AdmissionSet,
    port: Port,
    journal: Option<std::sync::Arc<std::sync::Mutex<crate::system_extension::Durable>>>,
    values: crate::system_extension::SharedCallbackValuePort,
    template: crate::system_extension::ExecutionTemplate,
}

pub fn materialize(
    root: &crate::node_state::DirectoryRoot,
    bundle: super::codec::Bundle,
    gate: &crate::node_runtime::ValidatedNodeSourceGate,
) -> crate::error::Result<Materialized> {
    let mut output = Materialized {
        runtime: Native::new(std::collections::BTreeMap::new())?,
        groups: std::collections::BTreeMap::new(),
        files: Vec::new(),
        directories: Vec::new(),
    };
    let mut services = std::collections::BTreeMap::new();
    let replacements = &bundle.replacements;
    for input in bundle.services {
        let service = prepare(root, input, gate, &mut output, true)?;
        let service = select_recovered(root, service, replacements, gate, &mut output)?;
        if let Some(group) = &service.admission.admitted.manifest().recovery_group
            && let Some(prior) = output.groups.insert(group.group_id.clone(), group.clone())
            && prior != *group
        {
            return Err(super::invalid("conflicting admitted recovery declarations"));
        }
        let (member, supervisor) = install_or_restore(service)?;
        if services.insert(member, supervisor).is_some() {
            return Err(super::invalid("duplicate native service identity"));
        }
    }
    output.runtime = Native::new(services)?;
    for replacement in bundle.replacements {
        let service = prepare(root, replacement.service, gate, &mut output, false)?;
        output.runtime.register_replacement(crate::system_extension::Replacement {
            admission: service.admission,
            port: service.port,
            values: service.values,
            template: service.template,
        })?;
    }
    Ok(output)
}

fn select_recovered(
    root: &crate::node_state::DirectoryRoot,
    service: Prepared,
    replacements: &[super::codec::ReplacementInput],
    gate: &crate::node_runtime::ValidatedNodeSourceGate,
    output: &mut Materialized,
) -> crate::error::Result<Prepared> {
    let journal = service.journal.as_ref().ok_or_else(|| super::invalid("missing native recovery journal"))?;
    let record = crate::system_extension::NativeHostJournal::latest_instance(
        &*journal.lock().map_err(|_| super::invalid("native recovery lock poisoned"))?,
        &service.template.context.instance_id,
    )
    .map_err(|error| super::invalid(format!("native admission recovery: {error:?}")))?;
    let Some(record) = record else {
        return Ok(service);
    };
    if record.manifest_ref == service.admission.admitted.manifest_ref()
        && record.executable_ref == service.admission.executable.executable.executable_ref
    {
        return Ok(service);
    }
    for replacement in replacements {
        let admitted = replacement.service.admit_manifest()?;
        if admitted.manifest_ref() != record.manifest_ref
            || replacement.service.executable.executable_ref != record.executable_ref
        {
            continue;
        }
        let mut current = prepare(root, replacement.service.clone(), gate, output, false)?;
        current.journal = service.journal;
        return Ok(current);
    }
    Err(super::invalid("durable native generation has no exact admitted replacement in bundle"))
}

fn prepare(
    root: &crate::node_state::DirectoryRoot,
    input: super::codec::ExtensionInput,
    gate: &crate::node_runtime::ValidatedNodeSourceGate,
    output: &mut Materialized,
    open_journal: bool,
) -> crate::error::Result<Prepared> {
    if input.executable.source_gate_ref != gate.validation_ref() {
        return Err(super::invalid("native executable source gate differs from validated startup gate"));
    }
    let admitted = input.admit_manifest()?;
    let profile = crate::system_extension::admit_native_host_profile(&input.host_profile)
        .map_err(|issues| super::invalid(format!("native profile denied: {issues:?}")))?;
    let executable = crate::system_extension::admit_native_executable(&profile, &input.executable)
        .map_err(|issues| super::invalid(format!("native executable denied: {issues:?}")))?;
    let execution_profile = crate::fabric_execution::admit_profile(&input.execution_profile)?;
    let values_scope = root.ledger()?.open_subdir(&crate::node_state::RelativePath::parse(&input.cache_namespace)?)?;
    let workspace = values_scope.process_workspace()?;
    let values = crate::system_extension::shared_callback_value_port(
        crate::system_extension::CapabilityCallbackValuePort::new(values_scope)?,
    );
    let port = crate::fabric_execution::Adapter::new(execution_profile, Publisher { values: values.clone() })
        .map_err(|_| super::invalid("native host requires live bounded process execution"))?;
    let mut resolved = input.resolved;
    resolved.workspace_path = workspace.process_path()?;
    output.directories.push(workspace);
    bind_executable(root, &input.artifact_path, &input.executable.executable_bytes_ref, &mut resolved, output)?;
    let journal = if open_journal {
        let ledger = root.ledger_store()?;
        let durable_root = crate::local_store::DurableStoreRoot::from_ledger(
            &ledger,
            &crate::local_store::Locator::parse(&input.storage_namespace)?,
        )?;
        let durability = crate::fabric_durability::durable_profile(&input.durability_profile)?;
        Some(std::sync::Arc::new(std::sync::Mutex::new(crate::system_extension::Durable::new(
            crate::fabric_durability::RedbDurableStateAdapter::open_root(durable_root, durability, input.namespace)?,
        ))))
    } else {
        None
    };
    let template = crate::system_extension::ExecutionTemplate {
        host_profile: profile.clone(),
        executable: executable.clone(),
        admitted: admitted.clone(),
        request: input.request,
        authority: input.authority,
        resources: input.resources,
        resolved,
        context: input.context,
    };
    Ok(Prepared {
        admission: crate::system_extension::AdmissionSet {
            profile,
            executable,
            admitted,
        },
        port,
        journal,
        values,
        template,
    })
}

fn bind_executable(
    root: &crate::node_state::DirectoryRoot,
    path: &str,
    expected: &str,
    resolved: &mut crate::fabric_execution::ResolvedContext,
    output: &mut Materialized,
) -> crate::error::Result<()> {
    let path = crate::node_state::RelativePath::parse(path)?;
    let file = root.open_executable(&path)?;
    let bytes = file.read_bounded(crate::node_state::MAX_EXECUTABLE_BYTES)?;
    if crate::preserves_rail::content_ref_from_bytes(&bytes) != expected {
        return Err(super::invalid("materialized executable byte identity differs from admitted evidence"));
    }
    resolved.executable_path = file.process_path()?;
    output.files.push(file);
    Ok(())
}

fn install_or_restore(
    service: Prepared,
) -> crate::error::Result<(String, crate::system_extension::Supervisor<Port, crate::system_extension::Durable>)> {
    let journal = service.journal.ok_or_else(|| super::invalid("service installation requires a durable journal"))?;
    let id = service.template.context.instance_id.clone();
    let record = crate::system_extension::NativeHostJournal::latest_instance(
        &*journal.lock().map_err(|_| super::invalid("native journal lock poisoned"))?,
        &id,
    )
    .map_err(|error| super::invalid(format!("native journal recovery: {error:?}")))?;
    let supervisor = if let Some(record) = record {
        let instance = std::sync::Arc::new(std::sync::Mutex::new(record));
        let executor = crate::system_extension::Process::new(
            service.port,
            journal.clone(),
            service.values,
            instance.clone(),
            service.template,
        )
        .map_err(|error| super::invalid(format!("native executor recovery: {error:?}")))?;

        crate::system_extension::Supervisor::from_recovered(service.admission, executor, journal, instance)
            .map_err(|error| super::invalid(format!("native supervisor recovery: {error:?}")))?
    } else {
        crate::system_extension::Supervisor::install(
            service.admission,
            service.port,
            journal,
            service.values,
            service.template,
        )
        .map_err(|error| super::invalid(format!("native supervisor installation: {error:?}")))?
    };
    let member = supervisor
        .instance()
        .map_err(|error| super::invalid(format!("native identity: {error:?}")))?
        .service_id;
    Ok((member, supervisor))
}

#[cfg(all(test, target_os = "linux"))]
mod capability_tests {
    #[test]
    fn executable_measurement_retains_inode_across_name_replacement_and_rejects_symlink() {
        let workspace =
            crate::test_support::ProcessWorkspace::new("native-retained-executable").expect("owned workspace");
        let root = crate::node_state::DirectoryRoot::open_existing(&workspace).expect("owned root");
        let path = crate::node_state::RelativePath::parse("callback").expect("artifact locator");
        root.write(&path, b"measured-before-replacement").expect("materialize owned artifact");
        let permissions = <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o700);
        std::fs::set_permissions(workspace.join("callback"), permissions).expect("owned executable mode");
        let executable = root.open_executable(&path).expect("retain regular executable capability");
        std::fs::rename(workspace.join("callback"), workspace.join("retained")).expect("replace artifact name");
        root.write(&path, b"different-replacement").expect("publish replacement name");
        assert_eq!(executable.read_bounded(128).expect("measure retained inode"), b"measured-before-replacement");
        assert_eq!(
            std::fs::read(executable.process_path().expect("retained process path"))
                .expect("read retained process inode"),
            b"measured-before-replacement"
        );
        assert_eq!(root.read(&path, 128).expect("read replacement"), b"different-replacement");
        std::os::unix::fs::symlink("retained", workspace.join("linked")).expect("owned adversarial symlink");
        assert!(
            root.open_executable(&crate::node_state::RelativePath::parse("linked").expect("symlink locator"))
                .is_err()
        );
    }

    #[test]
    fn optional_namespace_acquisition_does_not_create_missing_capabilities() {
        let workspace =
            crate::test_support::ProcessWorkspace::new("native-optional-namespace").expect("owned workspace");
        let root = crate::node_state::DirectoryRoot::open_existing(&workspace).expect("owned root");
        assert!(
            root.existing_namespace(crate::node_state::NamespaceKind::Registry)
                .expect("observe optional namespace")
                .is_none()
        );
        assert!(
            !root
                .try_exists(&crate::node_state::RelativePath::parse("registry").expect("namespace locator"))
                .expect("observe unchanged root")
        );
        let ledger = root.ledger().expect("explicitly prepare authorized ledger");
        assert!(
            ledger
                .existing_subdir(&crate::node_state::RelativePath::parse("optional").expect("child locator"))
                .expect("observe optional child")
                .is_none()
        );
        assert!(
            !root
                .try_exists(&crate::node_state::RelativePath::parse("ledger/optional").expect("child root locator"))
                .expect("observe unchanged ledger")
        );
    }
}
