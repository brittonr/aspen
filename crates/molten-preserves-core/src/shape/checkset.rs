//! `<checks [<check name status> ...]>` sets: shape, admitted statuses and required names.

use alloc::borrow::ToOwned;

use crate::bounded::Count;

type String = alloc::string::String;
type Vec<T> = alloc::vec::Vec<T>;
type ViewError = super::ViewError;
type PackedValue = crate::model::PackedValue;

/// The statuses a parsed check set admits.
pub const ADMITTED_CHECK_STATUSES: [&str; 3] = ["pass", "fail", "deny"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCheck {
    pub name: String,
    pub status: String,
}

/// Parses `<checks [<check name status> ...]>` with at most `maximum` checks, unique names and
/// statuses in [`ADMITTED_CHECK_STATUSES`].
// r[impl molten.preserves_rail_toolkit.check_sets]
pub fn parse_checks_record(value: &PackedValue, maximum: u64, context: &str) -> Result<Vec<ParsedCheck>, ViewError> {
    let maximum = maximum.to_usize("checks")?;
    let record = super::simple_record_fields(value, crate::build::CHECKS_LABEL, 1)?;
    let values = super::required_sequence_field(&record[0], "checks")?;
    crate::bounded::ensure_count_at_most(values.len(), maximum, "checks")?;
    let mut checks = Vec::with_capacity(values.len());
    let mut seen = alloc::collections::BTreeSet::new();
    for item in values {
        let check = parse_check(item, context)?;
        if !seen.insert(check.name.clone()) {
            return Err(ViewError::DuplicateCheck {
                context: context.to_owned(),
                name: check.name,
            });
        }
        checks.push(check);
    }
    Ok(checks)
}

/// Requires check names only: this establishes record shape, not verification. Statuses, including
/// a constant `pass` on a denial label, are producer-described; admission uses the decision and
/// bound refs.
// r[impl molten.preserves_rail_toolkit.check_sets]
pub fn require_checks_present(checks: &[ParsedCheck], expected: &[&str], context: &str) -> Result<(), ViewError> {
    let missing = expected.iter().find(|expected| !checks.iter().any(|check| check.name == **expected));
    match missing {
        Some(name) => Err(ViewError::MissingCheck {
            context: context.to_owned(),
            name: (*name).to_owned(),
        }),
        None => Ok(()),
    }
}

fn parse_check(item: &PackedValue, context: &str) -> Result<ParsedCheck, ViewError> {
    let check = super::simple_record_fields(item, crate::build::CHECK_LABEL, 2)?;
    let name = super::required_string_field(&check[0], "check name")?;
    let status = super::required_string_field(&check[1], "check status")?;
    if !ADMITTED_CHECK_STATUSES.contains(&status.as_str()) {
        return Err(ViewError::UnsupportedContextCheckStatus {
            context: context.to_owned(),
            status,
        });
    }
    Ok(ParsedCheck { name, status })
}
