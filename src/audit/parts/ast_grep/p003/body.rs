// Content-bound rule-bundle and scan-scope identities over supplied file bytes.

/// One supplied file observation: a repository-relative path and the exact bytes the audit shell read from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentSource<'a> {
    pub path: &'a str,
    pub bytes: &'a [u8],
}

/// Canonical identity of the rule bundle: the profile metadata plus the BLAKE3 digest of every declared YAML
/// rule file's bytes. Each declared rule must be supplied exactly once, must declare its own `id` and posture,
/// and no undeclared rule file may be supplied, so a YAML edit always changes the identity.
pub fn rule_bundle_hash(profile: &AuditProfile, rule_sources: &[ContentSource<'_>]) -> crate::error::Result<String> {
    // r[impl aspen.ast_grep_runtime_authority_audits.identity]
    // r[impl molten.evidence_gates.static_audit_content_bound_hashes]
    let sources = content_sources("rule", rule_sources)?;
    let mut diagnostics = Vec::with_capacity(profile.rules.len().saturating_mul(2).saturating_add(sources.len()));
    for rule in &profile.rules {
        let Some(source) = sources.get(rule.rule_path.as_str()) else {
            diagnostics.push(format!("rule {} YAML source {} was not supplied", rule.id, rule.rule_path));
            continue;
        };
        let Ok(text) = std::str::from_utf8(source.bytes) else {
            diagnostics.push(format!("rule source {} is not UTF-8", rule.rule_path));
            continue;
        };
        if !text.lines().any(|line| line.trim_end() == format!("id: {}", rule.id)) {
            diagnostics.push(format!("rule source {} does not declare id {}", rule.rule_path, rule.id));
        }
        if !text.lines().any(|line| line.trim() == format!("posture: {}", rule.posture.as_str())) {
            diagnostics.push(format!(
                "rule source {} does not declare posture {}",
                rule.rule_path,
                rule.posture.as_str()
            ));
        }
    }
    let declared = profile.rules.iter().map(|rule| rule.rule_path.as_str()).collect::<std::collections::BTreeSet<_>>();
    diagnostics.extend(
        sources
            .keys()
            .filter(|path| !declared.contains(*path))
            .map(|path| format!("rule source {path} is not declared by profile {}", profile.id)),
    );
    if !diagnostics.is_empty() {
        return Err(malformed_observation(diagnostics.join("; ")));
    }
    let value = crate::preserves_rail::record("ast-grep-rule-bundle-v1", vec![
        crate::preserves_rail::string(&profile.id),
        crate::preserves_rail::sequence(
            profile_metadata_lines(profile).iter().map(crate::preserves_rail::string).collect(),
        ),
        crate::preserves_rail::sequence(content_digest_values(&sources)),
    ]);
    crate::preserves_rail::canonical_hash(&value)
}

/// Canonical identity of the exact scanned source inventory: every repository-relative path with the BLAKE3
/// digest of the bytes scanned there. Adding, removing, renaming, or editing any scanned file changes it.
pub fn scan_scope_hash(scanned_sources: &[ContentSource<'_>]) -> crate::error::Result<String> {
    // r[impl aspen.ast_grep_runtime_authority_audits.identity]
    // r[impl molten.evidence_gates.static_audit_content_bound_hashes]
    let sources = content_sources("scanned", scanned_sources)?;
    let value = crate::preserves_rail::record("ast-grep-scan-scope-v1", vec![crate::preserves_rail::sequence(
        content_digest_values(&sources),
    )]);
    crate::preserves_rail::canonical_hash(&value)
}

fn profile_metadata_lines(profile: &AuditProfile) -> Vec<String> {
    let mut lines = vec![format!("profile:{}", profile.id)];
    for surface in &profile.surfaces {
        lines.push(format!("surface:{}", surface.id));
        for scope in &surface.scan_scopes {
            lines.push(format!("scope:{}:{scope}", surface.id));
        }
    }
    for rule in &profile.rules {
        lines.push(format!(
            "rule:{}:{}:{}:{}:{}:{}:{}",
            rule.id,
            rule.category,
            rule.pattern_summary,
            rule.posture.as_str(),
            rule.rule_path,
            rule.positive_fixture.as_deref().unwrap_or("none"),
            rule.negative_fixture.as_deref().unwrap_or("none")
        ));
    }
    for non_claim in &profile.non_claims {
        lines.push(format!("non-claim:{non_claim}"));
    }
    lines.sort();
    lines
}

/// Indexes supplied file observations by path, rejecting an empty or oversized inventory, duplicate paths,
/// and paths that are not normalized repository-relative paths.
fn content_sources<'a>(
    label: &str,
    sources: &[ContentSource<'a>],
) -> crate::error::Result<std::collections::BTreeMap<&'a str, ContentSource<'a>>> {
    if sources.is_empty() {
        return Err(malformed_observation(format!("no {label} sources were supplied")));
    }
    crate::bounded::ensure_count_at_most(sources.len(), MAX_CONTENT_SOURCES, "ast-grep content sources")?;
    let mut indexed = std::collections::BTreeMap::new();
    for source in sources {
        if !is_repository_relative_path(source.path) {
            return Err(malformed_observation(format!(
                "{label} source path {:?} is not a normalized repository-relative path",
                source.path
            )));
        }
        if indexed.insert(source.path, *source).is_some() {
            return Err(malformed_observation(format!("{label} source {} was supplied twice", source.path)));
        }
    }
    Ok(indexed)
}

fn content_digest_values(sources: &std::collections::BTreeMap<&str, ContentSource<'_>>) -> Vec<preserves::IOValue> {
    sources
        .iter()
        .map(|(path, source)| {
            crate::preserves_rail::record("file", vec![
                crate::preserves_rail::string(path),
                crate::preserves_rail::string(crate::preserves_rail::content_ref_from_bytes(source.bytes)),
            ])
        })
        .collect()
}

fn is_repository_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && path.split('/').all(|component| !component.is_empty() && component != "." && component != "..")
}

fn malformed_observation(message: String) -> crate::error::Failure {
    crate::error::Failure::classified(crate::error::FailureClass::MalformedObservation, message)
}
