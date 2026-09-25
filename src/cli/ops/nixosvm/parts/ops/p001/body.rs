
fn run_shard_run(input: super::command::ShardRunInput) -> Outcome<()> {
    let shard = molten::nixos_vm::evaluate_vm_shard_run(&molten::nixos_vm::NixosVmShardRunInput {
        shard_id: &input.shard_id,
        scenario_fixture_ref: &input.scenario_fixture_ref,
        topology_ref: &input.topology_ref,
        package_ref: &input.package_ref,
        evidence_scope: &input.evidence_scope,
        node_evidence_refs: &input.node_evidence_refs,
        child_receipt_refs: &input.child_receipt_refs,
        diagnostic_log_refs: &input.diagnostic_log_refs,
        unavailable: input.unavailable,
        claimed_decision: &input.claimed_decision,
        caveats: &input.caveats,
    })?;
    let is_written_to_file = super::io::write_optional_preserves(input.out.as_ref(), &shard.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "nixos-vm shard-run ref={} decision={} diagnostics={}",
            shard.shard_ref,
            shard.decision,
            shard.diagnostics.len()
        ),
    );
    if shard.decision == "pass" {
        Ok(())
    } else {
        Err(molten::error::MoltenError::invalid_harness(format!(
            "nixos VM shard denied: {}",
            shard.diagnostics.join(",")
        )))
    }
}

fn run_aggregate(input: super::command::AggregateInput) -> Outcome<()> {
    let default_shard_scopes;
    let shard_scopes = if input.shard_scopes.is_empty() {
        default_shard_scopes = vec![molten::nixos_vm::NIXOS_VM_SCOPE_EXECUTABLE_VM.to_string(); input.shard_refs.len()];
        &default_shard_scopes
    } else {
        &input.shard_scopes
    };
    let aggregate = molten::nixos_vm::evaluate_vm_aggregate(&molten::nixos_vm::NixosVmAggregateInput {
        topology_ref: &input.topology_ref,
        package_ref: &input.package_ref,
        manifest_ref: &input.manifest_ref,
        required_shard_ids: &input.required_shard_ids,
        shard_refs: &input.shard_refs,
        shard_scopes,
        denied_shard_ids: &input.denied_shard_ids,
        unavailable_as_pass_shard_ids: &input.unavailable_as_pass_shard_ids,
        stale_child_refs: &input.stale_child_refs,
        log_only_child_refs: &input.log_only_child_refs,
        caveats: &input.caveats,
    })?;
    let is_written_to_file = super::io::write_optional_preserves(input.out.as_ref(), &aggregate.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "nixos-vm aggregate ref={} decision={} diagnostics={}",
            aggregate.aggregate_ref,
            aggregate.decision,
            aggregate.diagnostics.len()
        ),
    );
    if aggregate.decision == "pass" {
        Ok(())
    } else {
        Err(molten::error::MoltenError::invalid_harness(format!(
            "nixos VM aggregate denied: {}",
            aggregate.diagnostics.join(",")
        )))
    }
}

fn read_preserves_files(paths: &[FilePath]) -> Outcome<Vec<preserves::IOValue>> {
    let mut values = Vec::with_capacity(paths.len());
    for path in paths {
        values.push(super::io::read_preserves_file(path)?);
    }
    Ok(values)
}

fn parse_expected_child_receipts(items: &[String]) -> Outcome<Vec<molten::nixos_vm::NixosVmExpectedChildReceipt>> {
    let mut receipts = Vec::with_capacity(items.len());
    for item in items {
        let fields = parse_key_value_fields(item, "expected child receipt")?;
        receipts.push(molten::nixos_vm::NixosVmExpectedChildReceipt {
            child_ref: required_key(&fields, "ref", "expected child receipt")?,
            receipt_class: required_any_key(&fields, &["class", "receipt-class"], "expected child receipt")?,
            decision: required_key(&fields, "decision", "expected child receipt")?,
            node_id: optional_key(&fields, "node"),
            peer_id: optional_key(&fields, "peer"),
            operation_id: required_any_key_optional(&fields, &["operation", "operation-id"]),
        });
    }
    Ok(receipts)
}

fn parse_required_artifacts(items: &[String]) -> Outcome<Vec<molten::nixos_vm::VmEvidenceManifestRequiredArtifact>> {
    let mut artifacts = Vec::with_capacity(items.len());
    for item in items {
        let Some((kind, content_ref)) = item.split_once('=') else {
            return Err(molten::error::MoltenError::invalid_harness("required artifact must use kind=ref syntax"));
        };
        if kind.trim().is_empty() || content_ref.trim().is_empty() {
            return Err(molten::error::MoltenError::invalid_harness(
                "required artifact kind and ref must not be empty",
            ));
        }
        artifacts.push(molten::nixos_vm::VmEvidenceManifestRequiredArtifact {
            kind: kind.to_string(),
            content_ref: content_ref.to_string(),
        });
    }
    Ok(artifacts)
}

fn parse_key_value_fields(item: &str, label: &str) -> Outcome<std::collections::BTreeMap<String, String>> {
    let mut fields = std::collections::BTreeMap::new();
    for pair in item.split(',') {
        let Some((key, value)) = pair.split_once('=') else {
            return Err(molten::error::MoltenError::invalid_harness(format!(
                "{label} must use comma-separated key=value fields"
            )));
        };
        if key.trim().is_empty() || value.trim().is_empty() {
            return Err(molten::error::MoltenError::invalid_harness(format!(
                "{label} key and value must not be empty"
            )));
        }
        if fields.insert(key.to_string(), value.to_string()).is_some() {
            return Err(molten::error::MoltenError::invalid_harness(format!("{label} duplicate key {key}")));
        }
    }
    Ok(fields)
}

fn required_key(fields: &std::collections::BTreeMap<String, String>, key: &str, label: &str) -> Outcome<String> {
    fields
        .get(key)
        .cloned()
        .ok_or_else(|| molten::error::MoltenError::invalid_harness(format!("{label} missing required key {key}")))
}

fn required_any_key(
    fields: &std::collections::BTreeMap<String, String>,
    keys: &[&str],
    label: &str,
) -> Outcome<String> {
    required_any_key_optional(fields, keys).ok_or_else(|| {
        molten::error::MoltenError::invalid_harness(format!("{label} missing required key {}", keys.join(" or ")))
    })
}

fn required_any_key_optional(fields: &std::collections::BTreeMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| fields.get(*key).cloned())
}

fn optional_key(fields: &std::collections::BTreeMap<String, String>, key: &str) -> Option<String> {
    fields.get(key).cloned()
}

fn manifest_path(root: Option<&FilePath>, path: &std::path::Path) -> String {
    if let Some(root) = root
        && let Ok(relative) = path.strip_prefix(root)
    {
        return relative.display().to_string();
    }
    path.display().to_string()
}

fn run_show(artifact: FilePath) -> Outcome<()> {
    let value = super::io::read_preserves_file(&artifact)?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    let rendered = molten::preserves_rail::to_text(&value)?;
    let kind = super::io::kind(&rendered);
    println!("nixos-vm {kind} ref={reference} path={}", artifact.display());
    Ok(())
}
