
fn read_source_pin_records(
    root: &std::path::Path,
) -> Outcome<Vec<molten::project_config_portability::SourcePinRecord>> {
    let cargo_lock = std::fs::read_to_string(root.join("Cargo.lock")).map_err(molten::error::MoltenError::from)?;
    let flake = std::fs::read_to_string(root.join("flake.nix")).map_err(molten::error::MoltenError::from)?;
    let cargo_revisions = cargo_private_revisions(&cargo_lock);
    let nix_revisions = nix_private_revisions(&flake);
    let mut dependencies = std::collections::BTreeSet::new();
    dependencies.extend(cargo_revisions.keys().cloned());
    dependencies.extend(nix_revisions.keys().cloned());
    Ok(dependencies
        .into_iter()
        .map(|dependency| molten::project_config_portability::SourcePinRecord {
            cargo_revision: cargo_revisions.get(&dependency).cloned().unwrap_or_else(|| "missing".to_string()),
            nix_revision: nix_revisions.get(&dependency).cloned().unwrap_or_else(|| "missing".to_string()),
            dependency,
        })
        .collect())
}

fn cargo_private_revisions(lock_text: &str) -> std::collections::BTreeMap<String, String> {
    let mut revisions = std::collections::BTreeMap::new();
    for line in lock_text.lines() {
        let Some(source_start) = line.find(CARGO_SOURCE_PREFIX) else {
            continue;
        };
        let source = &line[source_start + CARGO_SOURCE_PREFIX.len()..];
        if let Some((dependency, revision)) = parse_source_dependency_revision(source) {
            revisions.entry(dependency).or_insert(revision);
        }
    }
    revisions
}

fn nix_private_revisions(flake_text: &str) -> std::collections::BTreeMap<String, String> {
    let mut revisions = std::collections::BTreeMap::new();
    for line in flake_text.lines() {
        let Some(source_start) = line.find(NIX_SOURCE_PREFIX) else {
            continue;
        };
        let source = &line[source_start + NIX_SOURCE_PREFIX.len()..];
        if let Some((dependency, revision)) = parse_source_dependency_revision(source) {
            revisions.entry(dependency).or_insert(revision);
        }
    }
    revisions
}

fn parse_source_dependency_revision(source: &str) -> Option<(String, String)> {
    let (dependency_part, revision_part) = source.split_once(SOURCE_REVISION_SEPARATOR)?;
    let dependency = dependency_part.split_once(GIT_SUFFIX).map(|(name, _suffix)| name).unwrap_or(dependency_part);
    if dependency.is_empty() {
        return None;
    }
    let revision = revision_part.split(TOML_QUOTE).next().unwrap_or_default().trim().to_string();
    if revision.is_empty() {
        return None;
    }
    Some((dependency.to_string(), revision))
}

fn render_config_lint_summary(report: &molten::project_config_portability::ConfigPortabilityReport) -> String {
    let diagnostics = if report.diagnostics.is_empty() {
        "none".to_string()
    } else {
        report.diagnostics.join(DIAGNOSTIC_JOIN_SEPARATOR)
    };
    format!(
        "config-portability report={} decision={} compared={} diagnostics={}\n",
        report.report_ref,
        report.decision,
        report.compared_source_pins.join(","),
        diagnostics
    )
}

fn run_effective_config(input: EffectiveConfigCommandInput) -> Outcome<()> {
    let sources = parse_effective_config_fields(input.fields)?;
    let readback = molten::project_effective_config::build_effective_config_readback(
        &molten::project_effective_config::EffectiveConfigInput {
            profile_refs: input.profile_refs,
            sources,
            release_mode: input.release_mode,
            diagnostics: Vec::new(),
        },
    )?;
    write_optional_preserves(input.out.as_ref(), &readback.value)?;
    write_optional_text(
        input.summary_out.as_ref(),
        &molten::project_effective_config::explain_effective_config(&readback)?,
    )?;
    eprintln!("effective-config ref={} decision={}", readback.fingerprint_ref, readback.decision);
    if readback.decision == "pass" {
        Ok(())
    } else {
        Err(molten::error::MoltenError::invalid_harness(format!(
            "effective config denied: {}",
            readback.diagnostics.join(DIAGNOSTIC_JOIN_SEPARATOR)
        )))
    }
}

fn parse_effective_config_fields(
    fields: Vec<String>,
) -> Outcome<Vec<molten::project_effective_config::ConfigSourceInput>> {
    let mut output = Vec::with_capacity(fields.len());
    for field in fields {
        let parts = field.split(EFFECTIVE_CONFIG_FIELD_SEPARATOR).map(str::to_string).collect::<Vec<_>>();
        if parts.len() != EFFECTIVE_CONFIG_FIELD_PARTS {
            return Err(molten::error::MoltenError::invalid_harness(format!(
                "effective config field must have {EFFECTIVE_CONFIG_FIELD_PARTS} pipe-delimited parts"
            )));
        }
        if parts.iter().take(EFFECTIVE_CONFIG_FIELD_PARTS - 1).any(|part| part.trim().is_empty()) {
            return Err(molten::error::MoltenError::invalid_harness("effective config field parts must not be empty"));
        }
        let caveats = if parts[4].trim().is_empty() {
            Vec::new()
        } else {
            parts[4]
                .split(EFFECTIVE_CONFIG_CAVEAT_SEPARATOR)
                .map(str::trim)
                .filter(|caveat| !caveat.is_empty())
                .map(str::to_string)
                .collect()
        };
        output.push(molten::project_effective_config::ConfigSourceInput {
            field: parts[0].clone(),
            value: parts[1].clone(),
            source_class: parts[2].clone(),
            source_ref: if parts[3] == EFFECTIVE_CONFIG_NONE_REF {
                None
            } else {
                Some(parts[3].clone())
            },
            admitted_override: parts[2] == "cli-override",
            caveats,
        });
    }
    Ok(output)
}

fn run_context_profile(input: ContextProfileCommandInput) -> Outcome<()> {
    let profile = molten::operator_context_profile::ContextProfileInput {
        profile_id: input.profile_id,
        profile_tier: input.profile_tier,
        refs: molten::operator_context_profile::ContextRefSet {
            policy_refs: input.policy_refs,
            capability_refs: Vec::new(),
            authority_refs: input.authority_refs,
            resource_refs: input.resource_refs,
            evidence_refs: input.evidence_refs,
            redaction_refs: Vec::new(),
            retention_refs: input.retention_refs,
        },
        allowed_operations: input.allowed_operations,
        caveats: vec!["CLI context profile expansion is evidence-only".to_string()],
    };
    let requirements = molten::operator_context_profile::OperationRequirements {
        operation: input.operation,
        require_policy: input.require_policy,
        require_authority: input.require_authority,
        require_resource: input.require_resource,
        require_evidence: input.require_evidence,
        require_retention: input.require_retention,
    };
    let overrides = molten::operator_context_profile::ContextOverrideInput {
        policy_refs: Vec::new(),
        authority_refs: input.override_authority_refs,
        resource_refs: Vec::new(),
        evidence_refs: input.override_evidence_refs,
        retention_refs: Vec::new(),
    };
    let expansion = molten::operator_context_profile::expand_context_profile(&profile, &requirements, &overrides)?;
    write_optional_preserves(input.out.as_ref(), &expansion.value)?;
    write_optional_text(input.summary_out.as_ref(), &render_context_profile_summary(&expansion))?;
    eprintln!("context-profile expansion={} decision={}", expansion.expansion_ref, expansion.decision);
    if expansion.decision == "pass" {
        Ok(())
    } else {
        Err(molten::error::MoltenError::invalid_harness(format!(
            "context profile expansion denied: {}",
            expansion.diagnostics.join(DIAGNOSTIC_JOIN_SEPARATOR)
        )))
    }
}

fn render_context_profile_summary(expansion: &molten::operator_context_profile::ContextExpansion) -> String {
    let diagnostics = if expansion.diagnostics.is_empty() {
        "none".to_string()
    } else {
        expansion.diagnostics.join(DIAGNOSTIC_JOIN_SEPARATOR)
    };
    format!(
        "context-profile profile={} expansion={} decision={} diagnostics={}\n",
        expansion.profile_ref, expansion.expansion_ref, expansion.decision, diagnostics
    )
}

fn collect_spec_sources(root: &std::path::Path) -> Outcome<Vec<molten::requirement_traceability::SpecSource>> {
    let mut sources = Vec::new();
    collect_specs_under(&root.join(".cairn/specs"), false, &mut sources)?;
    collect_specs_under(&root.join(".cairn/changes"), true, &mut sources)?;
    Ok(sources)
}

fn raw_file_ref(path: &std::path::Path) -> Outcome<String> {
    let bytes = std::fs::read(path).map_err(molten::error::MoltenError::from)?;
    Ok(molten::preserves_rail::content_ref_from_bytes(&bytes))
}

fn parse_junit_counts(text: &str) -> Outcome<molten::testing_hardening::CiTestCounts> {
    let total = junit_attribute(text, JUNIT_TESTS_ATTRIBUTE)?;
    let failures = junit_optional_attribute(text, JUNIT_FAILURES_ATTRIBUTE)?;
    let errors = junit_optional_attribute(text, JUNIT_ERRORS_ATTRIBUTE)?;
    let skipped = junit_optional_attribute(text, JUNIT_SKIPPED_ATTRIBUTE)?;
    let failed = failures
        .checked_add(errors)
        .ok_or_else(|| molten::error::MoltenError::invalid_harness("JUnit failure/error count overflow"))?;
    Ok(molten::testing_hardening::CiTestCounts {
        total,
        passed: junit_passed_count(total, failed, skipped)?,
        failed,
        skipped,
    })
}

fn junit_passed_count(total: u64, failed: u64, skipped: u64) -> Outcome<u64> {
    total
        .checked_sub(failed)
        .and_then(|count| count.checked_sub(skipped))
        .ok_or_else(|| molten::error::MoltenError::invalid_harness("JUnit passed count underflow"))
}

fn junit_optional_attribute(text: &str, name: &str) -> Outcome<u64> {
    match junit_attribute_value(text, name)? {
        Some(value) => parse_junit_attribute_value(value, name),
        None => Ok(0),
    }
}

fn junit_attribute(text: &str, name: &str) -> Outcome<u64> {
    let Some(value) = junit_attribute_value(text, name)? else {
        return Err(molten::error::MoltenError::invalid_harness(format!("JUnit missing {name} attribute")));
    };
    parse_junit_attribute_value(value, name)
}

fn junit_attribute_value<'a>(text: &'a str, name: &str) -> Outcome<Option<&'a str>> {
    let prefix = format!("{name}=\" ");
    let compact_prefix = prefix.trim_end();
    let Some(start_index) = text.find(compact_prefix).map(|index| index + compact_prefix.len()) else {
        return Ok(None);
    };
    let rest = &text[start_index..];
    let Some(end_index) = rest.find(JUNIT_QUOTE) else {
        return Err(molten::error::MoltenError::invalid_harness(format!("JUnit unterminated {name} attribute")));
    };
    Ok(Some(&rest[..end_index]))
}

fn parse_junit_attribute_value(value: &str, name: &str) -> Outcome<u64> {
    value
        .parse::<u64>()
        .map_err(|error| molten::error::MoltenError::invalid_harness(format!("JUnit invalid {name} count: {error}")))
}
