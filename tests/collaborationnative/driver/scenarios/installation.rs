//! Refused installations: a mismatched selected configuration and a denied worker startup.

const SOURCE_TRUST: &str = "MOLTEN_TEST_SOURCE_TRUST";
const EVIDENCE_ROOT: &str = "MOLTEN_TEST_EVIDENCE_ROOT";
/// The operator scope authority trust whose absence denies scope activation.
const SCOPE_TRUST: &str = "operator-scope-authority/current.preserves";

// r[verify molten.system_extension.native_effect_installation]
#[test]
#[ignore = "requires independently provisioned MOLTEN_TEST_SOURCE_TRUST signed producer output"]
fn installed_native_effect_rejects_a_mismatched_configuration() -> crate::producer::Result<()> {
    let trust = std::path::PathBuf::from(std::env::var_os(SOURCE_TRUST).ok_or("signed source trust")?);
    let output = crate::driver::output_of(&trust)?;
    let evidence = std::env::var_os(EVIDENCE_ROOT)
        .map(std::path::PathBuf::from)
        .map(|path| molten::node_state::DirectoryRoot::open(&path))
        .transpose()?;
    let workspace = crate::test_support::process_workspace("collaboration-native-mismatched-installation")?;
    let mut cohort = crate::producer::prepare(
        workspace.as_ref(),
        &output.join("bin/molten"),
        &output.join("bin/molten-collaboration-scope"),
        &trust,
        None,
    )?;
    cohort.release_endpoints();
    let fixture = &cohort.nodes[0];
    let original = fixture.effects.clone();
    let original_ref = molten::node_service_host::effects::config::reference(&original)?;
    // A distinct installation identity: canonical content of a fresh record, not the admitted one.
    let installation_ref = molten::preserves_rail::canonical_hash(&molten::preserves_rail::record(
        "collaboration-native-mismatched-installation-v1",
        vec![molten::preserves_rail::string(&original_ref)],
    ))?;
    let mut mismatched = original.clone();
    let binding = mismatched.bindings.first_mut().ok_or("admitted effect binding missing")?;
    binding.installation_ref = installation_ref;
    let mismatched_bytes = molten::node_service_host::effects::config::encode(&mismatched)?;
    let mismatched_ref = molten::node_service_host::effects::config::reference(&mismatched)?;
    if original_ref == mismatched_ref {
        return Err("mismatched installation retained the admitted configuration reference".into());
    }
    let issuer = crate::authority::Issuer::new(crate::driver::operations::seed(0)?)?;
    crate::driver::setup::prepare(&cohort.signed_main, fixture, &issuer, 0, 1)?;
    fixture
        .root
        .write(&molten::node_state::RelativePath::parse(crate::producer::EFFECTS)?, &mismatched_bytes)?;
    let mut node = crate::driver::start(&cohort.signed_main, fixture, 0, issuer)?;
    if crate::driver::await_ready(&mut node).is_ok() {
        return Err("mismatched installation published hosting readiness".into());
    }
    let exit = refused_exit(&mut node)?;
    let absence = molten::preserves_rail::sequence(vec![
        namespace_absent(fixture, "scope-application", &fixture.runtime.scope_store.storage_namespace)?,
        namespace_absent(fixture, "raft", &fixture.runtime.raft_store.storage_namespace)?,
    ]);
    let observation =
        molten::preserves_rail::record("collaboration-native-mismatched-installation-observation-v1", vec![
            molten::preserves_rail::string(cohort.signed_main.to_str().ok_or("source executable encoding")?),
            molten::preserves_rail::u64_value(0),
            molten::preserves_rail::string(&original_ref),
            molten::preserves_rail::string(&mismatched_ref),
            molten::preserves_rail::u64_value(exit),
            absence,
            molten::preserves_rail::string("mismatched-config-refusal-is-not-a-callback-route-or-completion-proof"),
        ]);
    eprintln!(
        "mismatched installation observations={}",
        crate::driver::scenarios::retain(&observation, evidence.as_ref())?
    );
    Ok(())
}

// r[verify molten.system_extension.native_effect_lifetime]
#[test]
#[ignore = "requires independently provisioned MOLTEN_TEST_SOURCE_TRUST signed producer output"]
fn installed_native_effect_startup_failure_publishes_no_readiness() -> crate::producer::Result<()> {
    let trust = std::path::PathBuf::from(std::env::var_os(SOURCE_TRUST).ok_or("signed source trust")?);
    let output = crate::driver::output_of(&trust)?;
    let evidence = std::env::var_os(EVIDENCE_ROOT)
        .map(std::path::PathBuf::from)
        .map(|path| molten::node_state::DirectoryRoot::open(&path))
        .transpose()?;
    let workspace = crate::test_support::process_workspace("collaboration-native-worker-startup-failure")?;
    let mut cohort = crate::producer::prepare(
        workspace.as_ref(),
        &output.join("bin/molten"),
        &output.join("bin/molten-collaboration-scope"),
        &trust,
        None,
    )?;
    cohort.release_endpoints();
    let fixture = &cohort.nodes[0];
    let issuer = crate::authority::Issuer::new(crate::driver::operations::seed(0)?)?;
    crate::driver::setup::prepare(&cohort.signed_main, fixture, &issuer, 0, 1)?;
    let trust_path = molten::node_state::RelativePath::parse(SCOPE_TRUST)?;
    fixture.root.remove_regular_file(&trust_path)?;
    if fixture.root.entry_kind(&trust_path)?.is_some() {
        return Err("removed operator scope authority trust remained present".into());
    }
    let mut node = crate::driver::start(&cohort.signed_main, fixture, 0, issuer)?;
    if crate::driver::await_ready(&mut node).is_ok() {
        return Err("denied worker startup published hosting readiness".into());
    }
    let exit = refused_exit(&mut node)?;
    let absence = molten::preserves_rail::sequence(vec![
        namespace_absent(fixture, "scope-application", &fixture.runtime.scope_store.storage_namespace)?,
        native_journal(fixture)?,
    ]);
    let observation =
        molten::preserves_rail::record("collaboration-native-worker-startup-failure-observation-v1", vec![
            molten::preserves_rail::u64_value(0),
            molten::preserves_rail::string(SCOPE_TRUST),
            molten::preserves_rail::u64_value(exit),
            absence,
            molten::preserves_rail::string("startup-denial-is-not-a-route-or-completion-observation"),
        ]);
    eprintln!(
        "worker startup failure observations={}",
        crate::driver::scenarios::retain(&observation, evidence.as_ref())?
    );
    Ok(())
}

/// A refused startup is an ordinary CLI error that never published readiness.
fn refused_exit(node: &mut crate::driver::Node) -> crate::producer::Result<u64> {
    let status = node
        .child
        .try_wait()?
        .ok_or_else(|| format!("node {} did not exit after its refusal", node.index))?;
    let control = node.control.as_mut().ok_or("retained control missing")?;
    control.close();
    control.join();
    let code = status
        .code()
        .ok_or_else(|| format!("node {} terminated without an ordinary exit: {status}", node.index))?;
    if code != 1 {
        return Err(format!("node {} refusal was not an ordinary CLI error: {status}", node.index).into());
    }
    let log = std::fs::read_to_string(&node.log)?;
    if log.lines().any(|line| line.trim_start().starts_with("<node-service-host-ready-v1")) {
        return Err(format!("node {} published readiness before refusing", node.index).into());
    }
    Ok(u64::try_from(code).map_err(|_| "negative ordinary exit code")?)
}

/// Capability metadata only: deriving or opening a durable root would create the namespace.
fn namespace_absent(
    fixture: &crate::producer::Node,
    role: &str,
    namespace: &str,
) -> crate::producer::Result<preserves::IOValue> {
    let ledger = fixture.root.ledger_store()?;
    if ledger.root().entry_kind_optional(&molten::local_store::Locator::parse(namespace)?)?.is_some() {
        return Err(format!("refused native startup created its {role} namespace").into());
    }
    Ok(molten::preserves_rail::record("capability-namespace-observation-v1", vec![
        molten::preserves_rail::string(role),
        molten::preserves_rail::string(namespace),
        molten::preserves_rail::string("absent"),
    ]))
}

/// The journal namespace is checked before any read so an absent store is evidence, not an error.
/// Operator preparation may already have installed an idle instance, but startup must have
/// completed no protected effect and left no uncertain retry behind.
fn native_journal(fixture: &crate::producer::Node) -> crate::producer::Result<preserves::IOValue> {
    let service = fixture.bundle.services.first().ok_or("native service")?;
    let ledger = fixture.root.ledger_store()?;
    let locator = molten::local_store::Locator::parse(&service.storage_namespace)?;
    if ledger.root().entry_kind_optional(&locator)?.is_none() {
        return Ok(journal_record(&service.storage_namespace, "namespace-absent", "journal-absent"));
    }
    let root = molten::local_store::DurableStoreRoot::from_ledger(&ledger, &locator)?;
    let adapter = molten::fabric_durability::RedbDurableStateAdapter::open_root(
        root,
        molten::fabric_durability::durable_profile(&service.durability_profile)?,
        service.namespace.clone(),
    )?;
    let journal = molten::system_extension::Durable::new(adapter);
    let record = molten::system_extension::NativeHostJournal::latest_instance(&journal, &service.context.instance_id)
        .map_err(|error| format!("actual native journal unavailable: {error:?}"))?;
    if let Some(record) = &record
        && (record
            .completed_operations
            .iter()
            .any(|operation| operation.kind == molten::system_extension::NativeOperationKind::Effect)
            || record.unresolved.iter().any(|operation| operation.is_retry_permitted))
    {
        return Err("failed worker startup completed a protected effect or permitted an uncertain retry".into());
    }
    let journal_state = if record.is_some() {
        "journal-present"
    } else {
        "journal-absent"
    };
    Ok(journal_record(&service.storage_namespace, "namespace-present", journal_state))
}

fn journal_record(namespace: &str, namespace_state: &str, journal_state: &str) -> preserves::IOValue {
    molten::preserves_rail::record("collaboration-native-journal-observation-v1", vec![
        molten::preserves_rail::string(namespace),
        molten::preserves_rail::string(namespace_state),
        molten::preserves_rail::string(journal_state),
        molten::preserves_rail::string("completed-effects-absent"),
    ])
}
