use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::record;
use molten::preserves_rail::string;
use molten::preserves_rail::to_text;
use molten::schema_identity;

#[derive(Debug, Subcommand)]
pub(crate) enum SchemaCommand {
    Identity {
        shape: PathBuf,
        #[arg(long)]
        schema_ref: String,
        #[arg(long, default_value = "structural")]
        mode: String,
        #[arg(long)]
        brand_ref: Option<String>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Alias {
        #[arg(long)]
        from_ref: String,
        #[arg(long)]
        to_ref: String,
        #[arg(long, default_value = "storage")]
        scope: String,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Compat {
        #[arg(long)]
        expected_identity: PathBuf,
        #[arg(long)]
        actual_identity: PathBuf,
        #[arg(long)]
        alias: Option<PathBuf>,
        #[arg(long)]
        migration_ref: Option<String>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    SearchFingerprint {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        fingerprint: String,
    },
}

pub(crate) fn run_schema_command(command: SchemaCommand) -> Result<()> {
    match command {
        SchemaCommand::Identity {
            shape,
            schema_ref,
            mode,
            brand_ref,
            out,
            receipt_out,
        } => {
            let shape = read_preserves_file(&shape)?;
            let value = schema_identity::schema_identity_value(&schema_identity::SchemaIdentityInput {
                mode,
                schema_ref,
                shape,
                brand_ref,
                metadata_refs: vec![cli_schema_ref("metadata", "identity")?],
                policy_refs: vec![cli_schema_ref("policy", "identity")?],
                evidence_refs: vec![cli_schema_ref("evidence", "identity")?],
            })?;
            let identity = schema_identity::parse_schema_identity(&value)?;
            let receipt = schema_identity::compatibility_receipt_value(
                "fingerprint",
                &schema_identity::compatibility_decision_value(&schema_identity::SchemaCompatibilityInput {
                    expected: identity.clone(),
                    actual: identity.clone(),
                    alias: None,
                    migration_ref: None,
                    policy_refs: identity.policy_refs.clone(),
                    evidence_refs: identity.evidence_refs.clone(),
                    deny_by_policy: false,
                })?,
            )?;
            write_file(&out, &to_text(&value)?)?;
            emit_named_receipt(receipt_out.as_ref(), "schema compatibility receipt", &receipt)?;
            println!(
                "schema identity ok identity={} schema={} fingerprint={} out={}",
                identity.identity_ref,
                identity.schema_ref,
                identity.structural_fingerprint,
                out.display()
            );
            Ok(())
        }
        SchemaCommand::Alias {
            from_ref,
            to_ref,
            scope,
            out,
            receipt_out,
        } => {
            let value = schema_identity::schema_alias_value(&schema_identity::SchemaAliasInput {
                from_schema_ref: from_ref,
                to_schema_ref: to_ref,
                scope,
                policy_refs: vec![cli_schema_ref("policy", "alias")?],
                evidence_refs: vec![cli_schema_ref("evidence", "alias")?],
            })?;
            let alias = schema_identity::parse_schema_alias(&value)?;
            let expected = local_unique_schema_identity(&alias.to_schema_ref)?;
            let actual = local_unique_schema_identity(&alias.from_schema_ref)?;
            let compatibility =
                schema_identity::compatibility_decision_value(&schema_identity::SchemaCompatibilityInput {
                    expected,
                    actual,
                    alias: Some(alias.clone()),
                    migration_ref: None,
                    policy_refs: alias.policy_refs.clone(),
                    evidence_refs: alias.evidence_refs.clone(),
                    deny_by_policy: false,
                })?;
            let receipt = schema_identity::compatibility_receipt_value("alias-admit", &compatibility)?;
            write_file(&out, &to_text(&value)?)?;
            emit_named_receipt(receipt_out.as_ref(), "schema compatibility receipt", &receipt)?;
            println!(
                "schema alias ok alias={} from={} to={} out={}",
                alias.alias_ref,
                alias.from_schema_ref,
                alias.to_schema_ref,
                out.display()
            );
            Ok(())
        }
        SchemaCommand::Compat {
            expected_identity,
            actual_identity,
            alias,
            migration_ref,
            out,
            receipt_out,
        } => {
            let expected = schema_identity::parse_schema_identity(&read_preserves_file(&expected_identity)?)?;
            let actual = schema_identity::parse_schema_identity(&read_preserves_file(&actual_identity)?)?;
            let alias = alias
                .as_ref()
                .map(|path| read_preserves_file(path).and_then(|value| schema_identity::parse_schema_alias(&value)))
                .transpose()?;
            let compatibility =
                schema_identity::compatibility_decision_value(&schema_identity::SchemaCompatibilityInput {
                    expected,
                    actual,
                    alias,
                    migration_ref,
                    policy_refs: vec![cli_schema_ref("policy", "compat")?],
                    evidence_refs: vec![cli_schema_ref("evidence", "compat")?],
                    deny_by_policy: false,
                })?;
            let parsed = schema_identity::parse_schema_compatibility(&compatibility)?;
            let receipt = schema_identity::compatibility_receipt_value("compatibility", &compatibility)?;
            if let Some(path) = out.as_ref() {
                write_file(path, &to_text(&compatibility)?)?;
            } else {
                println!("{}", to_text(&compatibility)?);
            }
            emit_named_receipt(receipt_out.as_ref(), "schema compatibility receipt", &receipt)?;
            eprintln!("schema compat ok decision={} compatibility={}", parsed.decision, parsed.compatibility_ref);
            Ok(())
        }
        SchemaCommand::SearchFingerprint { registry, fingerprint } => {
            for identity in schema_identity::search_registry_by_fingerprint(&registry, &fingerprint)? {
                println!("{} {} {}", identity.identity_ref, identity.schema_ref, identity.mode);
            }
            Ok(())
        }
    }
}

fn local_unique_schema_identity(schema_ref: &str) -> Result<schema_identity::SchemaIdentity> {
    let shape = record("shape", vec![string("any-preserves")]);
    let value = schema_identity::schema_identity_value(&schema_identity::SchemaIdentityInput {
        mode: schema_identity::MODE_UNIQUE.to_string(),
        schema_ref: schema_ref.to_string(),
        shape,
        brand_ref: None,
        metadata_refs: vec![cli_schema_ref("metadata", schema_ref)?],
        policy_refs: vec![cli_schema_ref("policy", schema_ref)?],
        evidence_refs: vec![cli_schema_ref("evidence", schema_ref)?],
    })?;
    schema_identity::parse_schema_identity(&value)
}

fn cli_schema_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("schema-cli-ref", vec![string(kind), string(label)]))
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
