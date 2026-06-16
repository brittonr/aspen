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
use molten::typed_storage;

#[derive(Debug, Subcommand)]
pub(crate) enum StorageCommand {
    Put {
        value: PathBuf,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        schema_ref: Option<String>,
        #[arg(long)]
        producer_ref: Option<String>,
        #[arg(long)]
        ref_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Get {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        schema_ref: Option<String>,
        #[arg(long)]
        migration_recipe: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Recipe {
        #[arg(long)]
        source_schema_ref: String,
        #[arg(long)]
        target_schema_ref: String,
        #[arg(long)]
        transformer_ref: String,
        #[arg(long, default_value = "schema-rename")]
        transformer_kind: String,
        #[arg(long, default_value = "explicit")]
        mode: String,
        #[arg(long)]
        out: PathBuf,
    },
    Migrate {
        recipe: PathBuf,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        namespace: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        ref_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Verify {
        storage_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        schema_ref: Option<String>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

pub(crate) fn run_storage_command(command: StorageCommand) -> Result<()> {
    match command {
        StorageCommand::Put {
            value,
            store,
            namespace,
            key,
            schema_ref,
            producer_ref,
            ref_out,
            receipt_out,
        } => {
            let value = read_preserves_file(&value)?;
            let producer_ref = match producer_ref {
                Some(producer_ref) => producer_ref,
                None => cli_storage_ref("producer", &namespace, &key)?,
            };
            let admission = typed_storage::TypedStorageAdmission::local_fixture(&format!("cli:{namespace}:{key}"));
            let put = typed_storage::put_value(&store, &typed_storage::TypedStoragePutInput {
                namespace: namespace.clone(),
                key: key.clone(),
                schema_ref,
                value,
                producer_ref,
                policy_refs: vec![admission.policy_ref.clone()],
                evidence_refs: admission.evidence_refs.clone(),
                admission,
            })?;
            if let Some(path) = ref_out.as_ref() {
                write_file(path, &to_text(&put.typed_ref_value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "typed storage receipt", &put.receipt_value)?;
            println!(
                "storage put ok namespace={} key={} storage_ref={} schema_ref={} value_ref={} store={}",
                namespace,
                key,
                put.storage_ref,
                put.schema_ref,
                put.value_ref,
                store.display()
            );
            Ok(())
        }
        StorageCommand::Get {
            store,
            namespace,
            key,
            schema_ref,
            migration_recipe,
            out,
            receipt_out,
        } => {
            let admission = typed_storage::TypedStorageAdmission::local_fixture(&format!("cli:{namespace}:{key}"));
            let get = if let Some(migration_recipe) = migration_recipe.as_ref() {
                let expected_schema_ref = schema_ref.as_deref().ok_or_else(|| {
                    MoltenError::invalid_harness("storage get --migration-recipe requires --schema-ref target")
                })?;
                let recipe_value = read_preserves_file(migration_recipe)?;
                typed_storage::get_value_with_migration(typed_storage::MigrationGetInput {
                    root: &store,
                    namespace: &namespace,
                    key: &key,
                    expected_schema_ref,
                    migration_recipe_value: &recipe_value,
                    admission: &admission,
                })?
            } else {
                typed_storage::get_value(&store, &namespace, &key, schema_ref.as_deref(), &admission)?
            };
            let text = to_text(&get.value)?;
            if let Some(path) = out.as_ref() {
                write_file(path, &text)?;
            } else {
                println!("{text}");
            }
            emit_named_receipt(receipt_out.as_ref(), "typed storage receipt", &get.receipt_value)?;
            eprintln!(
                "storage get ok namespace={} key={} storage_ref={} schema_ref={} store={}",
                namespace,
                key,
                get.storage_ref,
                get.typed_ref.schema_ref,
                store.display()
            );
            Ok(())
        }
        StorageCommand::Recipe {
            source_schema_ref,
            target_schema_ref,
            transformer_ref,
            transformer_kind,
            mode,
            out,
        } => {
            let recipe = typed_storage::migration_recipe_value(&typed_storage::StorageMigrationRecipeInput {
                source_schema_ref,
                target_schema_ref,
                transformer_ref,
                transformer_kind,
                mode,
                policy_refs: vec![cli_storage_ref("migration-policy", "recipe", "policy")?],
                evidence_refs: vec![cli_storage_ref("migration-evidence", "recipe", "evidence")?],
            })?;
            write_file(&out, &to_text(&recipe)?)?;
            println!("storage recipe ok recipe_ref={} out={}", canonical_hash(&recipe)?, out.display());
            Ok(())
        }
        StorageCommand::Migrate {
            recipe,
            store,
            namespace,
            key,
            ref_out,
            receipt_out,
        } => {
            let recipe = read_preserves_file(&recipe)?;
            let admission = typed_storage::TypedStorageAdmission::local_fixture(&format!("cli:{namespace}:{key}"));
            let migrated = typed_storage::migrate_value(&store, &namespace, &key, &recipe, &admission)?;
            if let Some(path) = ref_out.as_ref() {
                write_file(path, &to_text(&migrated.typed_ref_value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "typed storage receipt", &migrated.receipt_value)?;
            println!(
                "storage migrate ok namespace={} key={} old_ref={} new_ref={} recipe={} store={}",
                namespace,
                key,
                migrated.old_storage_ref,
                migrated.new_storage_ref,
                migrated.recipe_ref,
                store.display()
            );
            Ok(())
        }
        StorageCommand::Verify {
            storage_ref,
            store,
            schema_ref,
            receipt_out,
        } => {
            let verified = typed_storage::verify_ref(&store, &storage_ref, schema_ref.as_deref())?;
            emit_named_receipt(receipt_out.as_ref(), "typed storage receipt", &verified.receipt_value)?;
            println!(
                "storage verify ok storage_ref={} namespace={} key={} schema_ref={} store={}",
                verified.storage_ref,
                verified.typed_ref.namespace,
                verified.typed_ref.key,
                verified.typed_ref.schema_ref,
                store.display()
            );
            Ok(())
        }
    }
}

fn cli_storage_ref(kind: &str, namespace: &str, key: &str) -> Result<String> {
    canonical_hash(&record("typed-storage-cli-ref", vec![string(kind), string(namespace), string(key)]))
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
