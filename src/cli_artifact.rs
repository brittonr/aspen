use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::artifacts;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::record;
use molten::preserves_rail::string;
use molten::preserves_rail::to_text;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub(crate) enum ArtifactCommand {
    Install {
        payload: PathBuf,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long, default_value = "artifact")]
        kind: String,
        #[arg(long = "dependency")]
        dependencies: Vec<String>,
        #[arg(long = "schema-ref")]
        schema_refs: Vec<String>,
        #[arg(long)]
        effect_manifest_ref: Option<String>,
        #[arg(long)]
        artifact_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        kind: Option<String>,
    },
    View {
        artifact_ref: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        payload: bool,
    },
    NameSet {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long, default_value = "name")]
        kind: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        artifact_ref: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    NameShow {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long, default_value = "name")]
        kind: String,
        #[arg(long)]
        name: String,
    },
    Deps {
        artifact_ref: String,
        #[arg(long)]
        registry: PathBuf,
    },
    Closure {
        artifact_ref: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Impact {
        artifact_ref: String,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    IndexRebuild {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

pub(crate) fn run_artifact_command(command: ArtifactCommand) -> Result<()> {
    match command {
        ArtifactCommand::Install {
            payload,
            registry,
            kind,
            dependencies,
            schema_refs,
            effect_manifest_ref,
            artifact_out,
            receipt_out,
        } => {
            let payload = read_preserves_file(&payload)?;
            let schemas = if schema_refs.is_empty() {
                vec![cli_artifact_ref("schema", &kind)?]
            } else {
                schema_refs
            };
            let install = artifacts::install_artifact(&registry, &artifacts::ArtifactInstallInput {
                kind: kind.clone(),
                payload,
                schema_refs: schemas,
                dependency_refs: dependencies,
                effect_manifest_ref,
                policy_refs: vec![cli_artifact_ref("policy", &kind)?],
                evidence_refs: vec![cli_artifact_ref("evidence", &kind)?],
                installer_ref: cli_artifact_ref("installer", &kind)?,
                capability_refs: vec![cli_artifact_ref("capability", &kind)?],
            })?;
            if let Some(path) = artifact_out.as_ref() {
                write_file(path, &to_text(&install.artifact.value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "artifact receipt", &install.receipt_value)?;
            println!(
                "artifact install {} artifact={} kind={} registry={}",
                install.decision,
                install.artifact_ref,
                install.artifact.kind,
                registry.display()
            );
            Ok(())
        }
        ArtifactCommand::List { registry, kind } => {
            for artifact in artifacts::list_artifacts(&registry, kind.as_deref())? {
                println!("{} {}", artifact.artifact_ref, artifact.kind);
            }
            Ok(())
        }
        ArtifactCommand::View {
            artifact_ref,
            registry,
            payload,
        } => {
            if payload {
                println!("{}", to_text(&artifacts::read_payload(&registry, &artifact_ref)?)?);
            } else {
                let artifact = artifacts::read_artifact(&registry, &artifact_ref)?;
                println!("{}", to_text(&artifact.value)?);
            }
            Ok(())
        }
        ArtifactCommand::NameSet {
            registry,
            kind,
            name,
            artifact_ref,
            receipt_out,
        } => {
            let policy_refs = [cli_artifact_ref("policy", &name)?];
            let evidence_refs = [cli_artifact_ref("evidence", &name)?];
            let pointer = artifacts::set_name_pointer(&registry, &artifacts::SetNamePointerInput {
                pointer_kind: &kind,
                name: &name,
                artifact_ref: &artifact_ref,
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
            })?;
            let receipt = artifacts::read_receipt(&registry, &pointer.receipt_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "artifact receipt", &receipt.value)?;
            println!(
                "artifact name-set ok kind={} name={} artifact={} pointer={}",
                pointer.pointer_kind, pointer.name, pointer.artifact_ref, pointer.pointer_ref
            );
            Ok(())
        }
        ArtifactCommand::NameShow { registry, kind, name } => {
            let pointer = artifacts::read_name_pointer(&registry, &kind, &name)?
                .ok_or_else(|| MoltenError::invalid_harness(format!("artifact pointer {kind}:{name} not found")))?;
            println!("{} {} {}", pointer.pointer_kind, pointer.name, pointer.artifact_ref);
            Ok(())
        }
        ArtifactCommand::Deps { artifact_ref, registry } => {
            for dependency in artifacts::direct_dependencies(&registry, &artifact_ref)? {
                println!("{dependency}");
            }
            Ok(())
        }
        ArtifactCommand::Closure {
            artifact_ref,
            registry,
            receipt_out,
        } => {
            let closure = artifacts::dependency_closure(&registry, &[artifact_ref])?;
            emit_named_receipt(receipt_out.as_ref(), "artifact receipt", &closure.receipt_value)?;
            for reference in &closure.closure_refs {
                println!("{reference}");
            }
            if !closure.missing_refs.is_empty() {
                eprintln!("missing dependencies: {}", closure.missing_refs.join(","));
            }
            eprintln!("artifact closure {} refs={}", closure.closure_hash, closure.closure_refs.len());
            Ok(())
        }
        ArtifactCommand::Impact {
            artifact_ref,
            registry,
            receipt_out,
        } => {
            let impact = artifacts::impact(&registry, &[artifact_ref])?;
            emit_named_receipt(receipt_out.as_ref(), "artifact receipt", &impact.receipt_value)?;
            for reference in &impact.impacted_refs {
                println!("{reference}");
            }
            eprintln!("artifact impact {} refs={}", impact.impact_hash, impact.impacted_refs.len());
            Ok(())
        }
        ArtifactCommand::IndexRebuild { registry, receipt_out } => {
            let rebuild = artifacts::rebuild_index(&registry)?;
            emit_named_receipt(receipt_out.as_ref(), "artifact receipt", &rebuild.receipt_value)?;
            println!(
                "artifact index-rebuild ok artifacts={} names={} registry={}",
                rebuild.artifacts,
                rebuild.names,
                registry.display()
            );
            Ok(())
        }
    }
}

fn cli_artifact_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("artifact-cli-ref", vec![string(kind), string(label)]))
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
