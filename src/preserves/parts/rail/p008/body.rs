
// The structural scan is the kernel walker (`molten_preserves_core::inspect`) driven over `IOValue`
// nodes by `scan_bridge`: an explicit stack of open containers in preorder, the first match, the
// first bound failure, and `Embedded` payloads scanned like any other child.

// r[impl molten.preserves_value_inspection.structural_scan]
pub fn find_structural_match<F>(
    value: &IoValue,
    scope: StructuralInspectionScope,
    predicate: F,
) -> Result<Option<StructuralMatch>>
where
    F: FnMut(StructuralTokenKind, &str) -> bool,
{
    find_structural_match_with_limits(value, scope, StructuralInspectionLimits::default(), predicate)
}

pub fn find_structural_match_with_limits<F>(
    value: &IoValue,
    scope: StructuralInspectionScope,
    limits: StructuralInspectionLimits,
    predicate: F,
) -> Result<Option<StructuralMatch>>
where
    F: FnMut(StructuralTokenKind, &str) -> bool,
{
    scan_bridge::find_match(value, scope, limits, predicate)
}

pub fn find_named_structural_marker(value: &IoValue, markers: &[&str]) -> Result<Option<StructuralMatch>> {
    scan_bridge::find_named(value, markers)
}

// r[impl molten.preserves_value_inspection.marker_detection]
pub fn find_sensitive_structural_marker(value: &IoValue) -> Result<Option<StructuralMatch>> {
    find_named_structural_marker(value, SENSITIVE_STRUCTURAL_MARKERS)
}

// r[impl molten.preserves_value_inspection.ambient_token_denial]
pub fn find_ambient_job_token(value: &IoValue) -> Result<Option<StructuralMatch>> {
    find_named_structural_marker(value, AMBIENT_JOB_TOKENS)
}

// r[impl molten.preserves_value_inspection.ref_retention]
pub fn find_structural_content_ref(value: &IoValue, target_ref: &str) -> Result<Option<StructuralMatch>> {
    let target = ContentRef::parse(target_ref)?;
    scan_bridge::find_content_ref(value, &target)
}

pub fn contains_structural_content_ref(value: &IoValue, target_ref: &str) -> Result<bool> {
    Ok(find_structural_content_ref(value, target_ref)?.is_some())
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/preserves/parts/rail/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/preserves/parts/rail/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/preserves/parts/rail/tests/m000/p002/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/preserves/parts/rail/tests/m000/p003/body.rs"));
}
