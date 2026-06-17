use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::record;
use molten::preserves_rail::sequence;
use molten::preserves_rail::to_text;
use molten::rewrites;

#[path = "rewrite/input.rs"]
mod input;

#[derive(Debug, Subcommand)]
pub(crate) enum RewriteCommand {
    Find {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long, default_value = "any")]
        pattern_kind: String,
        #[arg(long, default_value = "")]
        pattern: String,
        #[arg(long = "kind")]
        artifact_kinds: Vec<String>,
        #[arg(long = "root")]
        root_refs: Vec<String>,
        #[arg(long = "include-dependencies", default_value = "true")]
        dependency_inclusion_enabled: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        matches_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Preview {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long = "kind")]
        artifact_kinds: Vec<String>,
        #[arg(long = "root")]
        root_refs: Vec<String>,
        #[arg(long = "include-dependencies", default_value = "true")]
        dependency_inclusion_enabled: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        plan_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Apply {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long = "kind")]
        artifact_kinds: Vec<String>,
        #[arg(long = "root")]
        root_refs: Vec<String>,
        #[arg(long = "include-dependencies", default_value = "true")]
        dependency_inclusion_enabled: bool,
        #[arg(long = "hide-ref")]
        hidden_refs: Vec<String>,
        #[arg(long)]
        plan_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
        #[arg(long)]
        upgrade_plan_out: Option<PathBuf>,
        #[arg(long, default_value = "rewrite-session")]
        session_id: String,
    },
    Show {
        artifact: PathBuf,
    },
}

pub(crate) fn run_rewrite_command(command: RewriteCommand) -> Result<()> {
    match command {
        RewriteCommand::Find {
            registry,
            pattern_kind,
            pattern,
            artifact_kinds,
            root_refs,
            dependency_inclusion_enabled,
            hidden_refs,
            matches_out,
            receipt_out,
        } => {
            let query = rewrite_query(input::QueryCliInput {
                pattern_kind,
                pattern,
                artifact_kinds,
                root_refs,
                dependency_inclusion_enabled,
                hidden_refs,
            })?;
            let found = rewrites::find(&registry, &query)?;
            if let Some(path) = matches_out.as_ref() {
                let value = record("rewrite-matches", vec![sequence(
                    found.matches.iter().map(|rewrite_match| rewrite_match.value.clone()).collect(),
                )]);
                write_file(path, &to_text(&value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "rewrite receipt", &found.receipt_value)?;
            for rewrite_match in &found.matches {
                println!("{} {} {}", rewrite_match.artifact_ref, rewrite_match.kind, rewrite_match.bindings.len());
            }
            eprintln!("rewrite find matches={} query={}", found.matches.len(), found.query_ref);
            Ok(())
        }
        RewriteCommand::Preview {
            registry,
            from,
            to,
            artifact_kinds,
            root_refs,
            dependency_inclusion_enabled,
            hidden_refs,
            plan_out,
            receipt_out,
        } => {
            let input = rewrite_plan_input(input::PlanCliInput {
                from,
                to,
                artifact_kinds,
                root_refs,
                dependency_inclusion_enabled,
                hidden_refs,
            })?;
            let preview = rewrites::preview(&registry, &input)?;
            if let Some(path) = plan_out.as_ref() {
                write_file(path, &to_text(&preview.plan_value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "rewrite receipt", &preview.receipt_value)?;
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
        RewriteCommand::Apply {
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
        } => {
            let input = rewrite_plan_input(input::PlanCliInput {
                from,
                to,
                artifact_kinds,
                root_refs,
                dependency_inclusion_enabled,
                hidden_refs,
            })?;
            let applied = rewrites::apply(&registry, &input)?;
            if let Some(path) = plan_out.as_ref() {
                write_file(path, &to_text(&applied.preview.plan_value)?)?;
            }
            if let Some(path) = upgrade_plan_out.as_ref() {
                let upgrade_plan = rewrites::upgrade_plan_from_apply(
                    &applied,
                    &session_id,
                    &cli_rewrite_ref("initiator", &session_id)?,
                    &[cli_rewrite_ref("capability", &session_id)?],
                    &[cli_rewrite_ref("policy", &session_id)?],
                )?;
                write_file(path, &to_text(&upgrade_plan)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "rewrite receipt", &applied.receipt_value)?;
            for installed in &applied.installed {
                println!(
                    "{} {} {}",
                    installed.old_artifact_ref, installed.new_artifact_ref, installed.install_receipt_ref
                );
            }
            eprintln!("rewrite apply installed={} plan={}", applied.installed.len(), applied.preview.plan_ref);
            Ok(())
        }
        RewriteCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", rewrites::rewrite_summary(&value)?);
            Ok(())
        }
    }
}

fn rewrite_query(input: input::QueryCliInput) -> Result<rewrites::RewriteQueryInput> {
    Ok(rewrites::RewriteQueryInput {
        artifact_kinds: input.artifact_kinds,
        root_refs: input.root_refs,
        include_dependencies: input.dependency_inclusion_enabled,
        pattern: cli_rewrite_pattern(&input.pattern_kind, &input.pattern)?,
        policy_refs: vec![cli_rewrite_ref("query-policy", &input.pattern_kind)?],
        capability_refs: vec![cli_rewrite_ref("query-capability", &input.pattern_kind)?],
        hidden_refs: input.hidden_refs,
    })
}

fn rewrite_plan_input(input: input::PlanCliInput) -> Result<rewrites::RewritePlanInput> {
    Ok(rewrites::RewritePlanInput {
        query: rewrite_query(input::QueryCliInput {
            pattern_kind: "string-equals".to_string(),
            pattern: input.from.clone(),
            artifact_kinds: input.artifact_kinds,
            root_refs: input.root_refs,
            dependency_inclusion_enabled: input.dependency_inclusion_enabled,
            hidden_refs: input.hidden_refs,
        })?,
        replacement: rewrites::RewriteReplacement::StringValue {
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

fn cli_rewrite_pattern(kind: &str, pattern: &str) -> Result<rewrites::RewritePattern> {
    match kind {
        "any" => Ok(rewrites::RewritePattern::Any),
        "artifact-kind" => Ok(rewrites::RewritePattern::ArtifactKind(pattern.to_string())),
        "record-label" => Ok(rewrites::RewritePattern::RecordLabel(pattern.to_string())),
        "string-equals" => Ok(rewrites::RewritePattern::StringEquals(pattern.to_string())),
        "string-contains" => Ok(rewrites::RewritePattern::StringContains(pattern.to_string())),
        "schema-shape-kind" => Ok(rewrites::RewritePattern::SchemaShapeKind(pattern.to_string())),
        "ref-contains" => Ok(rewrites::RewritePattern::RefContains(pattern.to_string())),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported rewrite pattern kind {other}; expected any, artifact-kind, record-label, string-equals, \
             string-contains, schema-shape-kind, or ref-contains"
        ))),
    }
}

fn cli_rewrite_ref(kind: &str, label: &str) -> Result<String> {
    rewrites::default_local_ref(kind, label)
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn emit_named_receipt(path: Option<&PathBuf>, label: &str, receipt: &preserves::IOValue) -> Result<()> {
    let receipt_text = to_text(receipt)?;
    let receipt_ref = canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("{label} {receipt_ref}");
    }
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
