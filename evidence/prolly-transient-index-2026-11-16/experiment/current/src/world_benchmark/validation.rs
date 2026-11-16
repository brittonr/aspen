mod profile;
mod receipt;
mod result;

pub use profile::*;
pub use receipt::*;
pub use result::*;

use super::WorldBenchmarkIssue;
use super::WorldBenchmarkMetricKind;

pub(super) fn valid_reference(value: &str) -> bool {
    const PREFIX: &str = "blake3:";
    const HEX_LENGTH: usize = 64;
    value.strip_prefix(PREFIX).is_some_and(|hex| {
        hex.len() == HEX_LENGTH && hex.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

pub(super) fn valid_revision(value: &str) -> bool {
    const REVISION_LENGTH: usize = 40;
    value.len() == REVISION_LENGTH && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(super) fn metric_name(kind: WorldBenchmarkMetricKind) -> &'static str {
    kind.as_str()
}

pub(super) fn sorted_issues(mut issues: Vec<WorldBenchmarkIssue>) -> Vec<WorldBenchmarkIssue> {
    issues.sort();
    issues.dedup();
    issues
}
