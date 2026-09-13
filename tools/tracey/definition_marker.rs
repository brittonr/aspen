//! Pure recognition of the two definition forms in the accepted specification tree.
//! This parser does not establish implementation coverage or lifecycle acceptance.

const REQUIREMENT_HEADING: &str = "### Requirement:";
const HEADING_MARKER_PREFIX: &str = " [r[";
const HEADING_MARKER_SUFFIX: &str = "]]";

pub(super) fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
}

pub(super) fn requirement_marker(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let raw = if let Some(marker) = trimmed.strip_prefix("r[") {
        let end = marker.find(']')?;
        &marker[..end]
    } else {
        let heading = trimmed.strip_prefix(REQUIREMENT_HEADING)?;
        let (title, marker) = heading.rsplit_once(HEADING_MARKER_PREFIX)?;
        if title.trim().is_empty() || title.contains("r[") {
            return None;
        }
        let raw = marker.strip_suffix(HEADING_MARKER_SUFFIX)?;
        if raw.contains('[') || raw.contains(']') {
            return None;
        }
        raw
    };
    if raw.split_whitespace().count() != 1 {
        return None;
    }
    let id = raw.split('+').next().unwrap_or(raw);
    valid_id(id).then(|| id.to_string())
}
