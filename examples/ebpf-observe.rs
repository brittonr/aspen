//! Actual unprivileged endpoint smoke consumer. No BPF loading or host discovery.

#[derive(clap::Parser)]
#[command(about = "Unprivileged Linux observation endpoint smoke consumer; no BPF loading or host discovery")]
struct Arguments {
    #[arg(long, value_parser = bounded)]
    endpoint_root: String,
    #[arg(long, value_parser = bounded)]
    endpoint_leaf: String,
    #[arg(long, value_parser = bounded)]
    expected_binding: String,
    #[arg(long, value_parser = bounded)]
    expected_profile: String,
    #[arg(long, value_parser = bounded)]
    epoch: String,
    #[arg(long, value_parser = bounded)]
    now_tick: String,
}

fn main() -> std::process::ExitCode {
    match <Arguments as clap::Parser>::try_parse().map_err(|error| error.to_string()).and_then(run) {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::FAILURE,
        Err(issue) => {
            eprintln!("Linux observation denied: {issue}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(args: Arguments) -> Result<bool, String> {
    let privileges = consumer_privileges()?;
    let export = molten::fabric_observability::decode_ebpf_profile(&read_input(&args.expected_profile, 65536)?)
        .map_err(|issue| format!("profile {issue:?}"))?;
    let expected: molten::fabric_observability::EbpfBinding =
        serde_json::from_slice(&read_input(&args.expected_binding, 16384)?).map_err(|_| "binding syntax")?;
    if export.binding != expected {
        return Err("independent binding/profile mismatch".into());
    }
    let profile = molten::fabric_observability::ebpf_observation_profile(&export)
        .map_err(|issue| format!("profile {issue:?}"))?;
    let descriptors =
        molten::fabric_observability::ebpf_metric_descriptors(&export).map_err(|issue| format!("profile {issue:?}"))?;
    let adapter = molten::fabric_observability::checked_ebpf_profile(export, &profile, &descriptors)
        .map_err(|issue| format!("profile {issue:?}"))?;
    let epoch = args.epoch.parse::<u64>().map_err(|_| "epoch syntax")?;
    let now = args.now_tick.parse::<u64>().map_err(|_| "freshness syntax")?;
    let artifacts = observe(&args, &profile, &adapter, epoch, now)?;
    let is_complete = artifacts.status.artifact.kind == molten::fabric_observability::AdapterOutcomeKind::Exported;
    let sample_refs: Vec<_> = artifacts.samples.iter().map(|sample| sample.artifact_ref.clone()).collect();
    let samples: Vec<_> = artifacts.samples.into_iter().map(|sample| sample.artifact).collect();
    let series = molten::fabric_observability::aggregate_metric_samples(&profile, &descriptors, &samples, now)
        .map_err(|_| "canonical aggregation denied")?;
    let snapshot = molten::fabric_observability::bounded_operator_snapshot(
        &profile,
        molten::fabric_observability::SnapshotBuildInput {
            snapshot_id: "linux-ebpf-window",
            profile_ref: &profile.profile_ref,
            scope: molten::fabric_observability::ClaimScope::LocalComponent,
            generation: epoch,
            as_of_tick: now,
            valid_until_tick: artifacts.health.artifact.context.valid_until_tick,
            series,
            event_refs: vec![artifacts.event.artifact_ref.clone()],
            health_refs: vec![artifacts.health.artifact_ref.clone()],
            integrity_result_refs: Vec::new(),
            adapter_outcome_refs: vec![artifacts.outcome.artifact_ref.clone()],
            evidence_refs: artifacts.health.artifact.context.evidence_refs.clone(),
        },
    )
    .map_err(|_| "canonical snapshot denied")?;
    println!(
        "{}",
        serde_json::json!({"complete": is_complete,
        "health": artifacts.health.artifact.state.as_str(), "health_ref": artifacts.health.artifact_ref,
        "sample_refs": sample_refs, "snapshot_ref": snapshot.artifact_ref,
        "status": artifacts.status.artifact.kind.as_str(), "status_ref": artifacts.status.artifact_ref,
        "outcome_ref": artifacts.outcome.artifact_ref,
        "event_ref": artifacts.event.artifact_ref, "deny_required_readiness": artifacts.deny_required_readiness,
        "consumer_privileges": privileges,
        "accounting": artifacts.accounting, "scope":"local-component", "epoch":epoch})
    );
    Ok(is_complete)
}

fn observe(
    args: &Arguments,
    profile: &molten::fabric_observability::ObservationProfile,
    adapter: &molten::fabric_observability::EbpfProfile,
    epoch: u64,
    now: u64,
) -> Result<molten::fabric_observability::EbpfServiceObservation, String> {
    let root = molten::node_state::Scope::open_existing(
        molten::node_state::NamespaceKind::Ledger,
        std::path::Path::new(&args.endpoint_root),
    )
    .map_err(|_| "endpoint capability unavailable")?;
    let leaf = molten::node_state::RelativePath::parse(&args.endpoint_leaf).map_err(|_| "endpoint locator")?;
    molten::fabric_observability::observe_linux_ebpf(molten::fabric_observability::EbpfServiceInput {
        profile,
        adapter,
        capability: Some(molten::fabric_observability::EbpfReadCapability {
            root: &root,
            leaf,
            content_ref: None,
        }),
        control: molten::fabric_observability::EbpfServiceControl {
            epoch,
            now,
            deadline: now,
            cancelled: false,
        },
        context: context(adapter, profile, epoch, now)?,
    })
    .map_err(|_| "canonical service denied".to_string())
}

fn bounded(value: &str) -> Result<String, String> {
    if value.len() > 4096 {
        return Err("argument bound".into());
    }
    Ok(value.to_owned())
}

fn read_input(path: &str, bound: u64) -> Result<Vec<u8>, String> {
    let path = std::path::Path::new(path);
    let parent = path.parent().ok_or("input root missing")?;
    let name = path.file_name().and_then(|name| name.to_str()).ok_or("input leaf")?;
    let root = molten::node_state::Scope::open_existing(molten::node_state::NamespaceKind::Ledger, parent)
        .map_err(|_| "input capability unavailable")?;
    let leaf = molten::node_state::RelativePath::parse(name).map_err(|_| "input leaf")?;
    root.read(&leaf, bound).map_err(|_| "bounded input read failed".into())
}

fn context(
    adapter: &molten::fabric_observability::EbpfProfile,
    profile: &molten::fabric_observability::ObservationProfile,
    epoch: u64,
    now: u64,
) -> Result<molten::fabric_observability::ObservationContext, String> {
    let binding = &adapter.binding;
    let mut evidence_refs = vec![
        molten::fabric_observability::canonical_ebpf_binding_ref(binding)
            .map_err(|_| "upstream identity roles denied")?,
        binding.program.clone(),
        binding.object.clone(),
        binding.btf.clone(),
        binding.scope.clone(),
        binding.accounting.clone(),
        binding.adapter.clone(),
    ];
    evidence_refs.sort();
    evidence_refs.dedup();
    Ok(molten::fabric_observability::ObservationContext {
        source_id: "linux-ebpf".into(),
        source_ref: binding.adapter.clone(),
        profile_ref: profile.profile_ref.clone(),
        scope: molten::fabric_observability::ClaimScope::LocalComponent,
        generation: epoch,
        observed_tick: now,
        valid_until_tick: now.checked_add(adapter.freshness_ticks).ok_or("freshness overflow")?,
        resource_ref: binding.scope.clone(),
        evidence_refs,
        non_claims: molten::fabric_observability::REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    })
}

fn consumer_privileges() -> Result<serde_json::Value, String> {
    // Verification-only own-process capability acquisition, not host/BPF discovery.
    let own_process = std::path::PathBuf::from(format!("/proc/{}", std::process::id()));
    let root = molten::node_state::Scope::open_existing(molten::node_state::NamespaceKind::Ledger, &own_process)
        .map_err(|_| "own process capability unavailable")?;
    let leaf = molten::node_state::RelativePath::parse("status").map_err(|_| "own process locator")?;
    let bytes = root.read(&leaf, 65536).map_err(|_| "own process status unavailable")?;
    let text = std::str::from_utf8(&bytes).map_err(|_| "own process status malformed")?;
    let selected = ["Uid", "CapEff", "CapPrm", "CapInh", "CapAmb", "CapBnd", "NoNewPrivs"];
    let mut fields = std::collections::BTreeMap::new();
    for line in text.lines() {
        if let Some((key, value)) = line.split_once(':')
            && selected.contains(&key)
        {
            let value = value.trim();
            if value.len() > 64 || fields.insert(key, value).is_some() {
                return Err("own process status ambiguous".into());
            }
        }
    }
    if fields.len() != selected.len() || fields.get("NoNewPrivs") != Some(&"1") {
        return Err("consumer privilege posture unavailable".into());
    }
    let mut uids = [0_u32; 4];
    let mut values = fields["Uid"].split_whitespace();
    for uid in &mut uids {
        *uid = values.next().ok_or("consumer uid missing")?.parse().map_err(|_| "consumer uid malformed")?;
    }
    if values.next().is_some() || uids.contains(&0) {
        return Err("consumer must not retain a root identity".into());
    }
    for key in ["CapEff", "CapPrm", "CapInh", "CapAmb", "CapBnd"] {
        if u64::from_str_radix(fields[key], 16).map_err(|_| "consumer capabilities malformed")? != 0 {
            return Err("consumer must not retain mutation capabilities".into());
        }
    }
    Ok(serde_json::json!({"uids": uids, "effective": fields["CapEff"], "permitted": fields["CapPrm"],
        "inheritable": fields["CapInh"], "ambient": fields["CapAmb"], "bounding": fields["CapBnd"], "no_new_privileges": true}))
}
