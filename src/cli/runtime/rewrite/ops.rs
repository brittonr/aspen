type Command = super::RewriteCommand;
type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        command @ Command::Find { .. } => find(command),
        command @ Command::Preview { .. } => preview(command),
        command @ Command::Apply { .. } => apply(command),
        Command::Show { artifact } => show(artifact),
    }
}

fn find(command: Command) -> Outcome<()> {
    let Command::Find {
        registry,
        pattern_kind,
        pattern,
        artifact_kinds,
        root_refs,
        dependency_inclusion_enabled,
        hidden_refs,
        matches_out,
        receipt_out,
    } = command
    else {
        return Err(wrong_handler("find"));
    };
    let query = rewrite_query(super::input::QueryCliInput {
        pattern_kind,
        pattern,
        artifact_kinds,
        root_refs,
        dependency_inclusion_enabled,
        hidden_refs,
    })?;
    let found = molten::rewrites::find(&registry, &query)?;
    if let Some(path) = matches_out.as_ref() {
        let value = molten::preserves_rail::record("rewrite-matches", vec![molten::preserves_rail::sequence(
            found.matches.iter().map(|rewrite_match| rewrite_match.value.clone()).collect(),
        )]);
        super::io::write_file(path, &molten::preserves_rail::to_text(&value)?)?;
    }
    super::io::emit_named_receipt(receipt_out.as_ref(), "rewrite receipt", &found.receipt_value)?;
    for rewrite_match in &found.matches {
        println!("{} {} {}", rewrite_match.artifact_ref, rewrite_match.kind, rewrite_match.bindings.len());
    }
    eprintln!("rewrite find matches={} query={}", found.matches.len(), found.query_ref);
    Ok(())
}

fn preview(command: Command) -> Outcome<()> {
    let Command::Preview {
        registry,
        from,
        to,
        artifact_kinds,
        root_refs,
        dependency_inclusion_enabled,
        hidden_refs,
        plan_out,
        receipt_out,
    } = command
    else {
        return Err(wrong_handler("preview"));
    };
    let input = rewrite_plan_input(super::input::PlanCliInput {
        from,
        to,
        artifact_kinds,
        root_refs,
        dependency_inclusion_enabled,
        hidden_refs,
    })?;
    let preview = molten::rewrites::preview(&registry, &input)?;
    if let Some(path) = plan_out.as_ref() {
        super::io::write_file(path, &molten::preserves_rail::to_text(&preview.plan_value)?)?;
    }
    super::io::emit_named_receipt(receipt_out.as_ref(), "rewrite receipt", &preview.receipt_value)?;
    for diff in &preview.diffs {
        println!("{} {} {}", diff.artifact_ref, diff.old_payload_ref, diff.new_payload_ref);
    }
    eprintln!(
        "rewrite preview decision={} diffs={} plan={}",
        if preview.diffs.is_empty() { "deny" } else { "pass" },
        preview.diffs.len(),
        preview.plan_ref
    );
    Ok(())
}

fn apply(command: Command) -> Outcome<()> {
    let Command::Apply {
        registry,
        from,
        to,
        artifact_kinds,
        root_refs,
        dependency_inclusion_enabled,
        hidden_refs,
        plan_out,
        receipt_out,
        upgrade_plan_out,
        session_id,
        source_state_root,
        initiator_ref,
        capability_refs,
        policy_refs,
    } = command
    else {
        return Err(wrong_handler("apply"));
    };
    let input = rewrite_plan_input(super::input::PlanCliInput {
        from,
        to,
        artifact_kinds,
        root_refs,
        dependency_inclusion_enabled,
        hidden_refs,
    })?;
    // Admit the upgrade-plan authority before any registry mutation, so a refusal leaves no plan and no
    // rewrite.
    let upgrade = upgrade_plan_out
        .map(|path| {
            UpgradePlanRequest::admit(UpgradePlanArgs {
                path,
                session_id,
                source_state_root,
                initiator_ref,
                capability_refs,
                policy_refs,
            })
        })
        .transpose()?;
    let (applied, upgrade_plan) = if let Some(upgrade) = upgrade.as_ref() {
        let (applied, plan) = upgrade.apply(&registry, &input)?;
        (applied, Some((&upgrade.path, plan)))
    } else {
        (molten::rewrites::apply(&registry, &input)?, None)
    };
    if let Some(path) = plan_out.as_ref() {
        super::io::write_file(path, &molten::preserves_rail::to_text(&applied.preview.plan_value)?)?;
    }
    if let Some((path, plan)) = upgrade_plan.as_ref() {
        super::io::write_file(path, &molten::preserves_rail::to_text(plan)?)?;
    }
    super::io::emit_named_receipt(receipt_out.as_ref(), "rewrite receipt", &applied.receipt_value)?;
    for installed in &applied.installed {
        println!("{} {} {}", installed.old_artifact_ref, installed.new_artifact_ref, installed.install_receipt_ref);
    }
    eprintln!("rewrite apply installed={} plan={}", applied.installed.len(), applied.preview.plan_ref);
    Ok(())
}

struct UpgradePlanArgs {
    path: FilePath,
    session_id: String,
    source_state_root: Option<FilePath>,
    initiator_ref: Option<String>,
    capability_refs: Vec<String>,
    policy_refs: Vec<String>,
}

/// Upgrade-plan authority admitted before apply: explicit operator refs plus authenticated source
/// evidence.
struct UpgradePlanRequest {
    path: FilePath,
    session_id: String,
    initiator_ref: String,
    capability_refs: Vec<String>,
    policy_refs: Vec<String>,
    source_gate: molten::node_daemon::AuthenticatedGate,
}

impl UpgradePlanRequest {
    // r[impl molten.local_rewrite.upgrade_hook_authority]
    fn admit(args: UpgradePlanArgs) -> Outcome<Self> {
        let initiator_ref = args.initiator_ref.ok_or_else(|| {
            molten::error::Failure::invalid_harness("rewrite apply --upgrade-plan-out requires --initiator-ref")
        })?;
        require_cli_refs(std::slice::from_ref(&initiator_ref), "--initiator-ref")?;
        require_cli_refs(&args.capability_refs, "--capability-ref")?;
        require_cli_refs(&args.policy_refs, "--policy-ref")?;
        let source_state_root = args.source_state_root.ok_or_else(|| {
            molten::error::Failure::invalid_harness(
                "rewrite apply --upgrade-plan-out requires --source-state-root with an operator-selected \
                 source-gate-trust.json",
            )
        })?;
        let root = molten::node_state::DirectoryRoot::open_existing(&source_state_root)?;
        let source_gate = molten::node_daemon::authenticated_gate_with_root(&root)?;
        Ok(Self {
            path: args.path,
            session_id: args.session_id,
            initiator_ref,
            capability_refs: args.capability_refs,
            policy_refs: args.policy_refs,
            source_gate,
        })
    }

    fn apply(
        &self,
        registry: &std::path::Path,
        input: &molten::rewrites::RewritePlanInput,
    ) -> Outcome<(molten::rewrites::RewriteApply, preserves::IOValue)> {
        molten::rewrites::apply_with_upgrade_plan(registry, input, &molten::rewrites::RewriteUpgradePlanInput {
            session_id: &self.session_id,
            initiator_ref: &self.initiator_ref,
            capability_refs: &self.capability_refs,
            policy_refs: &self.policy_refs,
            source_gate: &self.source_gate,
        })
    }
}

fn require_cli_refs(refs: &[String], flag: &str) -> Outcome<()> {
    if refs.is_empty() {
        return Err(molten::error::Failure::invalid_harness(format!(
            "rewrite apply --upgrade-plan-out requires at least one explicit {flag}"
        )));
    }
    for reference in refs {
        molten::preserves_rail::validate_content_ref(reference).map_err(|error| {
            molten::error::Failure::invalid_harness(format!("{flag} must be a canonical content ref: {error}"))
        })?;
    }
    Ok(())
}

fn show(artifact: FilePath) -> Outcome<()> {
    let value = super::io::read_preserves_file(&artifact)?;
    println!("{}", molten::rewrites::rewrite_summary(&value)?);
    Ok(())
}

fn rewrite_query(input: super::input::QueryCliInput) -> Outcome<molten::rewrites::RewriteQueryInput> {
    Ok(molten::rewrites::RewriteQueryInput {
        artifact_kinds: input.artifact_kinds,
        root_refs: input.root_refs,
        include_dependencies: input.dependency_inclusion_enabled,
        pattern: cli_rewrite_pattern(&input.pattern_kind, &input.pattern)?,
        policy_refs: vec![cli_rewrite_ref("query-policy", &input.pattern_kind)?],
        capability_refs: vec![cli_rewrite_ref("query-capability", &input.pattern_kind)?],
        hidden_refs: input.hidden_refs,
    })
}

fn rewrite_plan_input(input: super::input::PlanCliInput) -> Outcome<molten::rewrites::RewritePlanInput> {
    Ok(molten::rewrites::RewritePlanInput {
        query: rewrite_query(super::input::QueryCliInput {
            pattern_kind: "string-equals".to_string(),
            pattern: input.from.clone(),
            artifact_kinds: input.artifact_kinds,
            root_refs: input.root_refs,
            dependency_inclusion_enabled: input.dependency_inclusion_enabled,
            hidden_refs: input.hidden_refs,
        })?,
        replacement: molten::rewrites::RewriteReplacement::StringValue {
            from: input.from.clone(),
            to: input.to,
        },
        planner_ref: cli_rewrite_ref("planner", &input.from)?,
        policy_refs: vec![cli_rewrite_ref("plan-policy", &input.from)?],
        capability_refs: vec![cli_rewrite_ref("plan-capability", &input.from)?],
        transcript_refs: vec![cli_rewrite_ref("transcript", &input.from)?],
        schema_migration_recipe_refs: vec![cli_rewrite_ref("schema-migration", &input.from)?],
    })
}

fn cli_rewrite_pattern(kind: &str, pattern: &str) -> Outcome<molten::rewrites::RewritePattern> {
    match kind {
        "any" => Ok(molten::rewrites::RewritePattern::Any),
        "artifact-kind" => Ok(molten::rewrites::RewritePattern::ArtifactKind(pattern.to_string())),
        "record-label" => Ok(molten::rewrites::RewritePattern::RecordLabel(pattern.to_string())),
        "string-equals" => Ok(molten::rewrites::RewritePattern::StringEquals(pattern.to_string())),
        "string-contains" => Ok(molten::rewrites::RewritePattern::StringContains(pattern.to_string())),
        "schema-shape-kind" => Ok(molten::rewrites::RewritePattern::SchemaShapeKind(pattern.to_string())),
        "ref-contains" => Ok(molten::rewrites::RewritePattern::RefContains(pattern.to_string())),
        other => Err(molten::error::Failure::invalid_harness(format!(
            "unsupported rewrite pattern kind {other}; expected any, artifact-kind, record-label, string-equals, \
             string-contains, schema-shape-kind, or ref-contains"
        ))),
    }
}

fn cli_rewrite_ref(kind: &str, label: &str) -> Outcome<String> {
    molten::rewrites::default_local_ref(kind, label)
}

fn wrong_handler(name: &str) -> molten::error::Failure {
    molten::error::Failure::invalid_harness(format!("rewrite {name} handler called with another command"))
}
