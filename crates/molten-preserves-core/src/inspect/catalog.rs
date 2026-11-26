//! Named structural markers and the standard scans built on them.

pub const SENSITIVE_STRUCTURAL_MARKERS: &[&str] = &["secret", "confidential", "credential", "private", "encrypted-ref"];

pub const AMBIENT_JOB_TOKENS: &[&str] = &[
    "mobile-code",
    "raw-closure",
    "closure",
    "host-path",
    "source-path",
    "source-registry",
    "process-command",
    "command",
    "env",
    "environment",
    "source-text",
];

/// The first record label or symbol in `names`.
pub fn find_named<N: super::TraversalNode>(
    root: N,
    names: &[&str],
) -> Result<Option<super::TokenMatch>, super::TraversalError> {
    super::find_match(root, super::TokenScope::STRUCTURAL_MARKERS, super::TraversalLimits::DEFAULT, |kind, token| {
        matches!(kind, super::TokenKind::RecordLabel | super::TokenKind::Symbol) && names.contains(&token)
    })
}

/// The first sensitive marker (`secret`, `credential`, ...), including inside `Embedded` values.
pub fn find_sensitive<N: super::TraversalNode>(root: N) -> Result<Option<super::TokenMatch>, super::TraversalError> {
    find_named(root, SENSITIVE_STRUCTURAL_MARKERS)
}

/// The first ambient job token (`host-path`, `command`, ...), including inside `Embedded` values.
pub fn find_ambient_job_token<N: super::TraversalNode>(
    root: N,
) -> Result<Option<super::TokenMatch>, super::TraversalError> {
    find_named(root, AMBIENT_JOB_TOKENS)
}

/// The first string equal to the canonical reference `target`.
pub fn find_content_ref<N: super::TraversalNode>(
    root: N,
    target: &crate::identity::ContentRef,
) -> Result<Option<super::TokenMatch>, super::TraversalError> {
    super::find_match(root, super::TokenScope::CONTENT_REFS, super::TraversalLimits::DEFAULT, |kind, token| {
        kind == super::TokenKind::ContentRef && token == target.as_str()
    })
}
