use std::fs;
use std::path::Path;
use std::path::PathBuf;

use molten::error::MoltenError;
use molten::error::Result;
use molten::eval_cache;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::record;
use molten::preserves_rail::sequence;
use molten::preserves_rail::string;
use molten::preserves_rail::to_text;

#[path = "cache/command.rs"]
mod command;

pub(crate) type CacheCommand = command::Command;

pub(crate) fn run_cache_command(command: CacheCommand) -> Result<()> {
    match command {
        CacheCommand::Put {
            input,
            cache,
            output,
            operation,
            version,
            dependencies,
            dependency_closure_hash,
            handler_profile_ref,
            policy_refs,
            capability_refs,
            revocation_refs,
            tool_ref,
            tool_version,
            mut assumption_refs,
            tier,
            status,
            evidence_refs,
            diagnostics,
            key_out,
            value_out,
            receipt_out,
        } => {
            let input_value = read_preserves_file(&input)?;
            let output_value = output.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let tool_ref = match tool_ref {
                Some(tool_ref) => tool_ref,
                None => cli_cache_ref("tool", &operation)?,
            };
            if matches!(status.as_str(), eval_cache::STATUS_DENY | eval_cache::STATUS_ERROR) {
                for evidence_ref in &evidence_refs {
                    if !assumption_refs.contains(evidence_ref)
                        && !policy_refs.contains(evidence_ref)
                        && !capability_refs.contains(evidence_ref)
                        && !revocation_refs.contains(evidence_ref)
                    {
                        assumption_refs.push(evidence_ref.clone());
                    }
                }
            }
            let closure_hash = match dependency_closure_hash {
                Some(hash) => hash,
                None => canonical_hash(&record("eval-cache-cli-closure", vec![
                    string(&operation),
                    preserves_sequence_strings(&dependencies),
                ]))?,
            };
            let key_input = eval_cache::EvalCacheKeyInput {
                operation: operation.clone(),
                version,
                input_ref: canonical_hash(&input_value)?,
                dependency_closure_hash: closure_hash,
                dependency_refs: dependencies,
                handler_profile_ref,
                policy_refs: policy_refs.clone(),
                capability_refs,
                revocation_refs,
                tool_ref,
                tool_version,
                assumption_refs,
            };
            let value_input = eval_cache::EvalCacheValueInput {
                tier,
                status,
                output: output_value,
                dependency_refs: key_input.dependency_refs.clone(),
                policy_refs,
                evidence_refs,
                diagnostics,
            };
            let put = eval_cache::put(&cache, &key_input, &value_input)?;
            if let Some(path) = key_out.as_ref() {
                write_file(path, &to_text(&put.key.value)?)?;
            }
            if let Some(path) = value_out.as_ref() {
                write_file(path, &to_text(&put.value.value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "eval cache receipt", &put.receipt_value)?;
            println!(
                "cache put ok key={} value={} operation={} tier={} status={} cache={}",
                put.key.key_ref,
                put.value.value_ref,
                put.key.operation,
                put.value.tier,
                put.value.status,
                cache.display()
            );
            Ok(())
        }
        CacheCommand::Get {
            key_ref,
            cache,
            current_policy_refs,
            current_capability_refs,
            current_revocation_refs,
            semantic_enabled,
            out,
            receipt_out,
        } => {
            let get = eval_cache::get(&cache, &key_ref, &eval_cache::EvalCacheGetInput {
                current_policy_refs,
                current_capability_refs,
                current_revocation_refs,
                semantic: semantic_enabled,
            })?;
            if let Some(output) = get.output.as_ref() {
                let text = to_text(output)?;
                if let Some(path) = out.as_ref() {
                    write_file(path, &text)?;
                } else {
                    println!("{text}");
                }
            } else if out.is_none() {
                println!("<none>");
            }
            emit_named_receipt(receipt_out.as_ref(), "eval cache receipt", &get.receipt_value)?;
            eprintln!(
                "cache get ok key={} value={} status={} tier={} cache={}",
                get.key.key_ref,
                get.value.value_ref,
                get.value.status,
                get.value.tier,
                cache.display()
            );
            Ok(())
        }
        CacheCommand::Status { cache } => {
            let status = eval_cache::status(&cache)?;
            println!(
                "keys={} values={} tombstones={} receipts={} tiers[pure={},simulated={},policy-current={},trace-only={}] statuses[pass={},deny={},error={},trace-only={}]",
                status.keys,
                status.values,
                status.tombstones,
                status.receipts,
                status.pure,
                status.simulated,
                status.policy_current,
                status.trace_only_tier,
                status.pass,
                status.deny,
                status.error,
                status.trace_only_status
            );
            Ok(())
        }
        CacheCommand::List {
            cache,
            operation,
            tier,
            status,
            dependency_ref,
            policy_ref,
            capability_ref,
            revocation_ref,
            evidence_ref,
        } => {
            for entry in eval_cache::list(&cache, &eval_cache::EvalCacheListFilter {
                operation,
                tier,
                status,
                dependency_ref,
                policy_ref,
                capability_ref,
                revocation_ref,
                evidence_ref,
            })? {
                println!(
                    "{} {} {} {} tombstoned={}",
                    entry.key_ref, entry.value_ref, entry.operation, entry.status, entry.tombstoned
                );
            }
            Ok(())
        }
        CacheCommand::Show { reference, cache } => {
            if let Ok(key) = eval_cache::read_key(&cache, &reference) {
                println!("{}", to_text(&key.value)?);
                return Ok(());
            }
            for entry in eval_cache::list(&cache, &eval_cache::EvalCacheListFilter {
                operation: None,
                tier: None,
                status: None,
                dependency_ref: None,
                policy_ref: None,
                capability_ref: None,
                revocation_ref: None,
                evidence_ref: None,
            })? {
                if entry.value_ref == reference {
                    let value = eval_cache::read_value(&cache, &entry.key_ref)?;
                    println!("{}", to_text(&value.value)?);
                    return Ok(());
                }
            }
            let receipt = eval_cache::read_receipt(&cache, &reference)?;
            println!("{}", to_text(&receipt.value)?);
            Ok(())
        }
        CacheCommand::Invalidate {
            cache,
            key_ref,
            dependency_ref,
            policy_ref,
            capability_ref,
            revocation_ref,
            operation,
            reason,
            apply_refs,
            retention,
            receipt_out,
        } => {
            let invalidated = eval_cache::invalidate(&cache, &eval_cache::EvalCacheInvalidateInput {
                key_ref,
                dependency_ref,
                policy_ref,
                capability_ref,
                revocation_ref,
                operation,
                reason,
                retention_evidence: retention.into_retention_evidence(),
                apply_refs,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "eval cache receipt", &invalidated.receipt_value)?;
            for key_ref in &invalidated.invalidated_key_refs {
                println!("{key_ref}");
            }
            eprintln!(
                "cache invalidate ok decision={} keys={} retention_receipts={} cache={}",
                invalidated.decision,
                invalidated.invalidated_key_refs.len(),
                invalidated.retention_receipt_refs.len(),
                cache.display()
            );
            Ok(())
        }
        CacheCommand::IndexRebuild { cache, receipt_out } => {
            let receipt = eval_cache::rebuild_index(&cache)?;
            emit_named_receipt(receipt_out.as_ref(), "eval cache receipt", &receipt)?;
            println!("cache index-rebuild ok cache={}", cache.display());
            Ok(())
        }
    }
}

fn cli_cache_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("eval-cache-cli-ref", vec![string(kind), string(label)]))
}

fn preserves_sequence_strings(values: &[String]) -> preserves::IOValue {
    sequence(values.iter().map(string).collect())
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
