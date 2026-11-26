use std::collections::BTreeMap;
use std::path::Path;

use super::model::{Evidence, Input, Observation, Subject};

pub fn read_bundle(input: &Input, root: &Path) -> BTreeMap<String, Vec<Observation>> {
    if input.capabilities.iter().all(|capability| capability.evidence.is_empty()) {
        return input.capabilities.iter().map(|capability| (capability.id.clone(), Vec::new())).collect();
    }
    // One independently provisioned source selection is revalidated once per
    // report, not once per family. Paths never enter canonical report identity.
    let source_path = root.join(&input.source_root_path);
    let source = molten::node_state::DirectoryRoot::open_existing(&source_path)
        .and_then(|root| molten::capability_qualification::readback(&root, &source_path));
    input.capabilities.iter().map(|capability| {
        let observations = capability.evidence.iter().map(|evidence| {
            inspect(evidence, root, &input.subject, source.as_ref().ok())
        }).collect();
        (capability.id.clone(), observations)
    }).collect()
}

pub fn inspect(evidence: &Evidence, root: &Path, subject: &Subject, source: Option<&molten::capability_qualification::ReadbackSource>) -> Observation {
    let mut result = Observation {
        requirement_id: evidence.requirement_id.clone(), coverage_kind: evidence.coverage_kind.clone(),
        family: evidence.family.clone(), scenario: evidence.scenario.clone(), artifact_ref: evidence.artifact_ref.clone(),
        native_decision: "inconclusive".into(), canonical_decision: "inconclusive".into(),
        validated: false, exact_subject_bound: false, diagnostics: Vec::new(),
    };
    let declarations_match = evidence.subject == *subject && !evidence.scenario.trim().is_empty();
    if !declarations_match {
        result.diagnostics.push("declared exact subject mismatch or missing finite scenario".into());
    }
    if !reference(&evidence.artifact_ref) {
        result.diagnostics.push("placeholder or malformed artifact reference".into()); return result;
    }
    let text = match std::fs::read_to_string(root.join(&evidence.artifact_path)) {
        Ok(text) => text,
        Err(_) => { result.diagnostics.push("missing or unreadable artifact".into()); return result; }
    };
    let value = match molten::preserves_rail::parse_text(&text) {
        Ok(value) => value,
        Err(_) => {
            result.diagnostics.push("not native Preserves; caller-authored JSON success, logs and JUnit are not runtime proof".into()); return result;
        }
    };
    if molten::preserves_rail::canonical_hash(&value).ok().as_ref() != Some(&evidence.artifact_ref) {
        result.diagnostics.push("canonical artifact reference mismatch".into()); return result;
    }
    if let Some(source) = source {
        match molten::capability_qualification::validate(source, &value, subject) {
            Ok(native) => {
                let scope_matches = native.requirement_id == evidence.requirement_id && native.family == evidence.family
                    && native.scenario.id() == evidence.scenario && native.scenario.coverage_kind() == evidence.coverage_kind;
                result.native_decision = native.decision.into(); result.canonical_decision = native.canonical_decision.into();
                result.validated = true;
                result.exact_subject_bound = declarations_match && scope_matches;
                if !scope_matches { result.diagnostics.push("native requirement/family/scenario/coverage binding mismatch".into()); }
                result.diagnostics.push(format!("native {:?} execution of family {} on one host only; not nixos-vm, cross-host, production or release qualification",
                    native.subject.environment, native.family));
                return result;
            }
            Err(error) => result.diagnostics.push(format!("native exact-subject validation denied: {error}")),
        }
    } else {
        result.diagnostics.push("independently selected signed source output unavailable or validation denied".into());
    }
    if let Ok(failure) = molten::harness::parse_failure(&value)
        .or_else(|_| molten::capability_qualification::failure(&value)) {
        result.validated = true; result.native_decision = "fail".into(); result.canonical_decision = "fail".into();
        result.diagnostics.push(format!("canonical failure retained: phase={} kind={} message={}", failure.phase, failure.kind, failure.message));
        result.diagnostics.push("raw failure attribution is caller-declared, not exact-build attested".into());
    } else if molten::harness::validate_report_value(&value).is_ok() {
        result.validated = true; result.native_decision = "pass".into(); result.canonical_decision = "pass".into();
        result.diagnostics.push("native report valid but lacks producer-owned exact source/build/closure/features/profile/adapters/platform/environment/family binding".into());
    } else {
        result.diagnostics.push("unsupported or invalid native artifact; generic manifest/gate success is not runtime execution".into());
    }
    result
}

pub fn reference(value: &str) -> bool {
    value.strip_prefix("blake3:").is_some_and(|hash| hash.len() == 64
        && hash.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && !hash.as_bytes()[..32].iter().all(|byte| *byte == b'0'))
}
