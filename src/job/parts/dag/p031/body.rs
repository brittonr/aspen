
fn selected_stage_set(
    dag: &JobDag,
    requested: &[String],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    stage_verdicts: &mut impl crate::bounded::VecSink<JobAdmissionStageVerdict>,
) -> Result<OrderedSet<String>> {
    let known = dag.nodes.iter().map(|node| node.id.clone()).collect::<OrderedSet<_>>();
    if requested.is_empty() {
        return Ok(known);
    }
    let mut selected = OrderedSet::new();
    for stage_id in requested {
        if known.contains(stage_id) {
            selected.insert(stage_id.clone());
        } else {
            let diagnostic = format!("unknown selected stage {stage_id}");
            push_bounded(diagnostics, diagnostic.clone(), MAX_JOB_REFS, "job admission diagnostics")?;
            push_bounded(
                stage_verdicts,
                JobAdmissionStageVerdict {
                    stage_id: stage_id.clone(),
                    decision: "deny".to_string(),
                    diagnostics: vec![diagnostic],
                },
                MAX_JOB_NODES,
                "job admission stage verdicts",
            )?;
        }
    }
    Ok(selected)
}

fn admission_roots(target_registry: &FilePath, dag: &JobDag, selected: &OrderedSet<String>) -> Result<Vec<String>> {
    let mut roots = vec![job_artifact_ref(target_registry, &dag.job_ref)?];
    for node in &dag.nodes {
        if selected.contains(&node.id)
            && let Some(stage_artifact_ref) = node.stage_artifact_ref.as_ref()
        {
            push_bounded(&mut roots, stage_artifact_ref.clone(), MAX_JOB_REFS, "job admission roots")?;
        }
    }
    roots.sort();
    roots.dedup();
    Ok(roots)
}

fn admission_roots_with_root(
    target_registry: &crate::artifacts::CapabilityArtifactRoot,
    dag: &JobDag,
    selected: &OrderedSet<String>,
) -> Result<Vec<String>> {
    let mut roots = vec![job_artifact_ref_with_root(target_registry, &dag.job_ref)?];
    for node in &dag.nodes {
        if selected.contains(&node.id)
            && let Some(stage_artifact_ref) = node.stage_artifact_ref.as_ref()
        {
            push_bounded(&mut roots, stage_artifact_ref.clone(), MAX_JOB_REFS, "job admission roots")?;
        }
    }
    roots.sort();
    roots.dedup();
    Ok(roots)
}

fn target_closure_state(
    target_registry: &FilePath,
    dag: &JobDag,
    selected: &OrderedSet<String>,
) -> Result<(bool, Vec<String>, Vec<String>)> {
    let roots = match admission_roots(target_registry, dag, selected) {
        Ok(roots) => roots,
        Err(error) => return Ok((false, Vec::new(), vec![format!("target closure roots denied: {error}")])),
    };
    let closure = match crate::artifacts::dependency_closure(target_registry, &roots) {
        Ok(closure) => closure,
        Err(error) => return Ok((false, Vec::new(), vec![format!("target closure computation failed: {error}")])),
    };

    let mut has_target_closure = true;
    let closure_refs = closure.closure_refs;
    let diagnostic_capacity = closure.missing_refs.len().saturating_add(closure_refs.len());
    let mut diagnostics = Vec::with_capacity(diagnostic_capacity);
    if !closure.missing_refs.is_empty() {
        has_target_closure = false;
        diagnostics.extend(closure.missing_refs.iter().map(|missing| format!("target closure missing {missing}")));
    }
    for artifact_ref in &closure_refs {
        if let Some(diagnostic) = target_closure_artifact_diagnostic(target_registry, artifact_ref) {
            has_target_closure = false;
            diagnostics.push(diagnostic);
        }
    }
    Ok((has_target_closure, closure_refs, diagnostics))
}

fn target_closure_artifact_diagnostic(target_registry: &FilePath, artifact_ref: &str) -> Option<String> {
    match crate::artifacts::read_artifact(target_registry, artifact_ref) {
        Ok(artifact) if artifact.artifact_ref == artifact_ref => None,
        Ok(artifact) => Some(format!("target artifact key {artifact_ref} contains envelope {}", artifact.artifact_ref)),
        Err(error) => Some(format!("target artifact {artifact_ref} unreadable: {error}")),
    }
}
