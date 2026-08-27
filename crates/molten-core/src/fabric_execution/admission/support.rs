use std::collections::BTreeSet;

use super::ExecutionAdmissionIssue;
use crate::fabric::valid_blake3_ref;
use crate::fabric::valid_fabric_token;

pub(super) fn validate_bound(
    field: &'static str,
    actual: u64,
    maximum: u64,
    issues: &mut Vec<ExecutionAdmissionIssue>,
) {
    if actual == 0 {
        issues.push(ExecutionAdmissionIssue::ZeroBound(field));
    } else if actual > maximum {
        issues.push(ExecutionAdmissionIssue::BoundExceeded { field, actual, maximum });
    }
}

pub(super) fn validate_token(field: &'static str, value: &str, issues: &mut Vec<ExecutionAdmissionIssue>) {
    if value.is_empty() {
        issues.push(ExecutionAdmissionIssue::EmptyField(field));
    } else if !valid_fabric_token(value) {
        issues.push(ExecutionAdmissionIssue::MalformedToken {
            field,
            value: value.to_string(),
        });
    }
}

pub(super) fn validate_ref(field: &'static str, value: &str, issues: &mut Vec<ExecutionAdmissionIssue>) {
    if value.is_empty() {
        issues.push(ExecutionAdmissionIssue::EmptyField(field));
    } else if !valid_blake3_ref(value) {
        issues.push(ExecutionAdmissionIssue::MalformedRef {
            field,
            value: value.to_string(),
        });
    }
}

pub(super) fn validate_refs(field: &'static str, values: &[String], issues: &mut Vec<ExecutionAdmissionIssue>) {
    for value in values {
        validate_ref(field, value, issues);
    }
}

pub(super) fn has_duplicates<T: Ord + Clone>(values: &[T]) -> bool {
    let mut seen = BTreeSet::new();
    values.iter().any(|value| !seen.insert(value.clone()))
}
