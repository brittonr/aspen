
fn materialization_boundary_count(dag: &JobDag) -> Result<u64> {
    usize_to_u64(
        dag.edges.iter().filter(|edge| edge.materialization != "stream").count()
            + dag.nodes.iter().filter(|node| node.kind == "materialize").count(),
        "job profile materialization boundary count",
    )
}

fn profile_value(
    dag: &JobDag,
    request: &JobOutputRequest,
    profile_stages: StageProfiles,
    counts: &ProfileCounts,
) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("job-profile-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_PROFILE_SCHEMA),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(&dag.job_ref)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&request.request_ref)]),
        crate::preserves_rail::record("stage-count", vec![crate::preserves_rail::u64_value(counts.stage_count)]),
        crate::preserves_rail::record("edge-count", vec![crate::preserves_rail::u64_value(counts.edge_count)]),
        crate::preserves_rail::record("materialization-boundaries", vec![crate::preserves_rail::u64_value(
            counts.materialization_boundaries,
        )]),
        estimated_bytes_value(profile_stages.config_bytes, counts.cache_entries)?,
        crate::preserves_rail::record("stages", vec![crate::preserves_rail::sequence(profile_stages.values)]),
        checks_value(&[
            "deterministic-profile",
            "no-wall-clock-time",
            "cache-projection-only",
            "trellis-order-bound",
        ]),
    ]))
}

fn estimated_bytes_value(config_bytes: u64, cache_entries: usize) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("estimated-bytes", vec![
        crate::preserves_rail::record("config", vec![crate::preserves_rail::u64_value(config_bytes)]),
        crate::preserves_rail::record("known-cache-entries", vec![crate::preserves_rail::u64_value(usize_to_u64(
            cache_entries,
            "job cache entry count",
        )?)]),
    ]))
}

fn profile_receipt_value(dag: &JobDag, request: &JobOutputRequest, profile_ref: &str) -> Result<IoValue> {
    analysis_receipt_value(AnalysisReceiptValueInput {
        label: "job-profile-receipt-v1",
        schema: crate::preserves_rail::JOB_PROFILE_RECEIPT_SCHEMA,
        operation: "profile",
        job_ref: &dag.job_ref,
        request_ref: &request.request_ref,
        artifact_ref: profile_ref,
        diagnostics: &[],
        checks: &[
            ("deterministic-profile", "pass"),
            ("no-wall-clock-time", "pass"),
            ("cache-projection-only", "pass"),
        ],
    })
}

pub fn sync_plan_value(
    source_registry: &FilePath,
    target_registry: &FilePath,
    request_value: &IoValue,
) -> Result<JobSyncPlan> {
    let request = parse_job_sync_request_value(request_value)?;
    let dag = read_job_dag(source_registry, &request.job_ref)?;
    let roots = sync_roots(source_registry, &dag, &request)?;
    let closure = crate::artifacts::dependency_closure(source_registry, &roots)?;
    if !closure.missing_refs.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "job sync source dependency closure missing refs: {}",
            closure.missing_refs.join(",")
        )));
    }
    let mut missing_refs = Vec::new();
    for artifact_ref in &closure.closure_refs {
        match crate::artifacts::read_artifact(target_registry, artifact_ref) {
            Ok(_) => {}
            Err(_) => push_bounded(&mut missing_refs, artifact_ref.clone(), MAX_JOB_REFS, "job sync missing refs")?,
        }
    }
    let value = crate::preserves_rail::record("job-sync-plan-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_SYNC_PLAN_SCHEMA),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&request.request_ref)]),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(&request.job_ref)]),
        crate::preserves_rail::record("target-peer", vec![crate::preserves_rail::string(&request.target_peer)]),
        crate::preserves_rail::record("roots", vec![refs_sequence(&roots)]),
        crate::preserves_rail::record("closure", vec![refs_sequence(&closure.closure_refs)]),
        crate::preserves_rail::record("missing", vec![refs_sequence(&missing_refs)]),
        crate::preserves_rail::record("stages", vec![crate::preserves_rail::sequence(
            request.stage_ids.iter().map(crate::preserves_rail::string).collect(),
        )]),
        checks_value(&[
            "dependency-closure",
            "hash-verify-before-install",
            "transport-neutral",
            "no-execution",
            "no-mobile-closures",
        ]),
    ]);
    let plan_ref = crate::preserves_rail::canonical_hash(&value)?;
    let receipt_value = analysis_receipt_value(AnalysisReceiptValueInput {
        label: "job-sync-receipt-v1",
        schema: crate::preserves_rail::JOB_SYNC_RECEIPT_SCHEMA,
        operation: "sync-plan",
        job_ref: &request.job_ref,
        request_ref: &request.request_ref,
        artifact_ref: &plan_ref,
        diagnostics: &[],
        checks: &[
            ("dependency-closure", "pass"),
            ("missing-set-computed", "pass"),
            ("no-execution", "pass"),
        ],
    })?;
    Ok(JobSyncPlan {
        plan_ref,
        request,
        root_refs: roots,
        closure_refs: closure.closure_refs,
        missing_refs,
        value,
        receipt_value,
    })
}

struct SyncInstallCandidate {
    artifact_ref: String,
    source: crate::artifacts::ArtifactRecord,
    payload: IoValue,
}

struct CandidateSelection {
    install_candidates: Vec<SyncInstallCandidate>,
    already_present_refs: Vec<String>,
    provenance_receipt_refs: Vec<String>,
    diagnostics: Vec<String>,
}

struct ReceiptInput<'a> {
    plan: &'a JobSyncPlan,
    decision: &'a str,
    installed_refs: &'a [String],
    already_present_refs: &'a [String],
    provenance_receipt_refs: &'a [String],
    diagnostics: &'a [String],
}

fn collect_candidates(
    input: &SyncLoopbackInput<'_>,
    plan: &JobSyncPlan,
    ordered_refs: Vec<String>,
) -> Result<CandidateSelection> {
    let missing = plan.missing_refs.iter().cloned().collect::<OrderedSet<_>>();
    let mut install_candidates = Vec::new();
    let mut already_present_refs = Vec::new();
    let mut provenance_receipt_refs = Vec::new();
    let mut diagnostics = Vec::new();
    for artifact_ref in ordered_refs {
        if !missing.contains(&artifact_ref) {
            push_bounded(&mut already_present_refs, artifact_ref, MAX_JOB_REFS, "job sync already-present refs")?;
            continue;
        }
        let source = crate::artifacts::read_artifact(input.source_registry, &artifact_ref)?;
        let payload = crate::artifacts::read_payload(input.source_registry, &artifact_ref)?;
        let provenance = crate::provenance::evaluate(&crate::provenance::EvaluationInput {
            operation: "remote-sync-install",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: input.provenance_values,
            build_verification_values: input.build_verification_values,
            prior_diagnostics: &[],
        })?;
        push_bounded(
            &mut provenance_receipt_refs,
            provenance.receipt_ref.clone(),
            MAX_JOB_REFS,
            "job sync provenance receipt refs",
        )?;
        if provenance.decision == "pass" {
            push_bounded(
                &mut install_candidates,
                SyncInstallCandidate {
                    artifact_ref,
                    source,
                    payload,
                },
                MAX_JOB_REFS,
                "job sync install candidates",
            )?;
        } else {
            push_bounded(
                &mut diagnostics,
                format!("job sync provenance denied artifact {} with receipt {}", artifact_ref, provenance.receipt_ref),
                MAX_JOB_REFS,
                "job sync diagnostics",
            )?;
            for diagnostic in provenance.diagnostics {
                push_bounded(&mut diagnostics, diagnostic, MAX_JOB_REFS, "job sync diagnostics")?;
            }
        }
    }
    Ok(CandidateSelection {
        install_candidates,
        already_present_refs,
        provenance_receipt_refs,
        diagnostics,
    })
}

fn apply_candidates(
    target_registry: &FilePath,
    request: &JobSyncRequest,
    candidates: Vec<SyncInstallCandidate>,
) -> Result<Vec<String>> {
    let mut installed_refs = Vec::new();
    for candidate in candidates {
        let installed = crate::artifacts::install_artifact(target_registry, &crate::artifacts::ArtifactInstallInput {
            kind: candidate.source.kind.clone(),
            payload: candidate.payload,
            schema_refs: candidate.source.schema_refs.clone(),
            dependency_refs: candidate.source.dependency_refs.clone(),
            effect_manifest_ref: candidate.source.effect_manifest_ref.clone(),
            policy_refs: candidate.source.policy_refs.clone(),
            evidence_refs: candidate.source.evidence_refs.clone(),
            installer_ref: local_ref("job-sync-installer", &request.request_ref)?,
            capability_refs: if request.capability_refs.is_empty() {
                vec![local_ref("job-sync-capability", &request.request_ref)?]
            } else {
                request.capability_refs.clone()
            },
        })?;
        if installed.decision != "pass" || installed.artifact_ref != candidate.artifact_ref {
            return Err(MoltenError::invalid_harness(format!(
                "job sync install mismatch for {}: decision={} installed={}",
                candidate.artifact_ref, installed.decision, installed.artifact_ref
            )));
        }
        let target = crate::artifacts::read_artifact(target_registry, &candidate.artifact_ref)?;
        if target.value != candidate.source.value {
            return Err(MoltenError::invalid_harness(format!(
                "job sync target artifact {} differs from source",
                candidate.artifact_ref
            )));
        }
        push_bounded(&mut installed_refs, candidate.artifact_ref, MAX_JOB_REFS, "job sync installed refs")?;
    }
    Ok(installed_refs)
}

fn loopback_receipt(input: ReceiptInput<'_>) -> Result<IoValue> {
    let mut refs = input.plan.closure_refs.clone();
    extend_cloned_bounded(&mut refs, input.installed_refs, MAX_JOB_REFS, "job sync refs")?;
    extend_cloned_bounded(&mut refs, input.already_present_refs, MAX_JOB_REFS, "job sync refs")?;
    extend_cloned_bounded(&mut refs, input.provenance_receipt_refs, MAX_JOB_REFS, "job sync refs")?;
    push_bounded(&mut refs, input.plan.plan_ref.clone(), MAX_JOB_REFS, "job sync refs")?;
    let is_clean = input.diagnostics.is_empty();
    Ok(crate::preserves_rail::record("job-sync-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_SYNC_RECEIPT_SCHEMA),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string("sync-loopback")]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(&input.plan.request.job_ref)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&input.plan.request.request_ref)]),
        crate::preserves_rail::record("artifact", vec![crate::preserves_rail::string(&input.plan.plan_ref)]),
        crate::preserves_rail::record("installed", vec![refs_sequence(input.installed_refs)]),
        crate::preserves_rail::record("already-present", vec![refs_sequence(input.already_present_refs)]),
        crate::preserves_rail::record("provenance", vec![refs_sequence(input.provenance_receipt_refs)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&[
            ("hash-verify-before-install", status(is_clean)),
            ("provenance-before-install", status(is_clean)),
            ("dependency-closure", "pass"),
            ("loopback-transfer", status(is_clean)),
            ("no-execution", "pass"),
            ("no-mobile-closures", "pass"),
            ("canonical-receipt", "pass"),
        ]),
    ]))
}
