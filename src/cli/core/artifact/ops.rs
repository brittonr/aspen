pub(super) fn install(command: super::ArtifactCommand) -> molten::error::Result<()> {
    let super::ArtifactCommand::Install {
        payload,
        registry,
        kind,
        dependencies,
        schema_refs,
        effect_manifest_ref,
        artifact_out,
        receipt_out,
    } = command
    else {
        return dispatch_mismatch("install");
    };
    let payload = super::io::read_preserves_file(&payload)?;
    let schemas = if schema_refs.is_empty() {
        vec![super::io::cli_artifact_ref("schema", &kind)?]
    } else {
        schema_refs
    };
    let install = molten::artifacts::install_artifact(&registry, &molten::artifacts::ArtifactInstallInput {
        kind: kind.clone(),
        payload,
        schema_refs: schemas,
        dependency_refs: dependencies,
        effect_manifest_ref,
        policy_refs: vec![super::io::cli_artifact_ref("policy", &kind)?],
        evidence_refs: vec![super::io::cli_artifact_ref("evidence", &kind)?],
        installer_ref: super::io::cli_artifact_ref("installer", &kind)?,
        capability_refs: vec![super::io::cli_artifact_ref("capability", &kind)?],
    })?;
    if let Some(path) = artifact_out.as_ref() {
        super::io::write_file(path, &molten::preserves_rail::to_text(&install.artifact.value)?)?;
    }
    super::io::emit_named_receipt(receipt_out.as_ref(), "artifact receipt", &install.receipt_value)?;
    println!(
        "artifact install {} artifact={} kind={} registry={}",
        install.decision,
        install.artifact_ref,
        install.artifact.kind,
        registry.display()
    );
    Ok(())
}

pub(super) fn list(command: super::ArtifactCommand) -> molten::error::Result<()> {
    let super::ArtifactCommand::List { registry, kind } = command else {
        return dispatch_mismatch("list");
    };
    for artifact in molten::artifacts::list_artifacts(&registry, kind.as_deref())? {
        println!("{} {}", artifact.artifact_ref, artifact.kind);
    }
    Ok(())
}

pub(super) fn view(command: super::ArtifactCommand) -> molten::error::Result<()> {
    let super::ArtifactCommand::View {
        artifact_ref,
        registry,
        payload,
    } = command
    else {
        return dispatch_mismatch("view");
    };
    if payload {
        println!("{}", molten::preserves_rail::to_text(&molten::artifacts::read_payload(&registry, &artifact_ref)?)?);
    } else {
        let artifact = molten::artifacts::read_artifact(&registry, &artifact_ref)?;
        println!("{}", molten::preserves_rail::to_text(&artifact.value)?);
    }
    Ok(())
}

pub(super) fn name_set(command: super::ArtifactCommand) -> molten::error::Result<()> {
    let super::ArtifactCommand::NameSet {
        registry,
        kind,
        name,
        artifact_ref,
        receipt_out,
    } = command
    else {
        return dispatch_mismatch("name-set");
    };
    let policy_refs = [super::io::cli_artifact_ref("policy", &name)?];
    let evidence_refs = [super::io::cli_artifact_ref("evidence", &name)?];
    let pointer = molten::artifacts::set_name_pointer(&registry, &molten::artifacts::SetNamePointerInput {
        pointer_kind: &kind,
        name: &name,
        artifact_ref: &artifact_ref,
        policy_refs: &policy_refs,
        evidence_refs: &evidence_refs,
    })?;
    let receipt = molten::artifacts::read_receipt(&registry, &pointer.receipt_ref)?;
    super::io::emit_named_receipt(receipt_out.as_ref(), "artifact receipt", &receipt.value)?;
    println!(
        "artifact name-set ok kind={} name={} artifact={} pointer={}",
        pointer.pointer_kind, pointer.name, pointer.artifact_ref, pointer.pointer_ref
    );
    Ok(())
}

pub(super) fn name_show(command: super::ArtifactCommand) -> molten::error::Result<()> {
    let super::ArtifactCommand::NameShow { registry, kind, name } = command else {
        return dispatch_mismatch("name-show");
    };
    let pointer = molten::artifacts::read_name_pointer(&registry, &kind, &name)?.ok_or_else(|| {
        molten::error::MoltenError::invalid_harness(format!("artifact pointer {kind}:{name} not found"))
    })?;
    println!("{} {} {}", pointer.pointer_kind, pointer.name, pointer.artifact_ref);
    Ok(())
}

pub(super) fn deps(command: super::ArtifactCommand) -> molten::error::Result<()> {
    let super::ArtifactCommand::Deps { artifact_ref, registry } = command else {
        return dispatch_mismatch("deps");
    };
    for dependency in molten::artifacts::direct_dependencies(&registry, &artifact_ref)? {
        println!("{dependency}");
    }
    Ok(())
}

pub(super) fn closure(command: super::ArtifactCommand) -> molten::error::Result<()> {
    let super::ArtifactCommand::Closure {
        artifact_ref,
        registry,
        receipt_out,
    } = command
    else {
        return dispatch_mismatch("closure");
    };
    let closure = molten::artifacts::dependency_closure(&registry, &[artifact_ref])?;
    super::io::emit_named_receipt(receipt_out.as_ref(), "artifact receipt", &closure.receipt_value)?;
    for reference in &closure.closure_refs {
        println!("{reference}");
    }
    if !closure.missing_refs.is_empty() {
        eprintln!("missing dependencies: {}", closure.missing_refs.join(","));
    }
    eprintln!("artifact closure {} refs={}", closure.closure_hash, closure.closure_refs.len());
    Ok(())
}

pub(super) fn impact(command: super::ArtifactCommand) -> molten::error::Result<()> {
    let super::ArtifactCommand::Impact {
        artifact_ref,
        registry,
        receipt_out,
    } = command
    else {
        return dispatch_mismatch("impact");
    };
    let impact = molten::artifacts::impact(&registry, &[artifact_ref])?;
    super::io::emit_named_receipt(receipt_out.as_ref(), "artifact receipt", &impact.receipt_value)?;
    for reference in &impact.impacted_refs {
        println!("{reference}");
    }
    eprintln!("artifact impact {} refs={}", impact.impact_hash, impact.impacted_refs.len());
    Ok(())
}

pub(super) fn index_rebuild(command: super::ArtifactCommand) -> molten::error::Result<()> {
    let super::ArtifactCommand::IndexRebuild { registry, receipt_out } = command else {
        return dispatch_mismatch("index-rebuild");
    };
    let rebuild = molten::artifacts::rebuild_index(&registry)?;
    super::io::emit_named_receipt(receipt_out.as_ref(), "artifact receipt", &rebuild.receipt_value)?;
    println!(
        "artifact index-rebuild ok artifacts={} names={} registry={}",
        rebuild.artifacts,
        rebuild.names,
        registry.display()
    );
    Ok(())
}

fn dispatch_mismatch(command: &str) -> molten::error::Result<()> {
    Err(molten::error::MoltenError::invalid_harness(format!("artifact {command} dispatch mismatch")))
}
