use molten_core::content_store_adapter::*;

use crate::error::MoltenError;
use crate::error::Result;
use crate::node_state::NodeStateFileObservation;
use crate::node_state::NodeStateNamespace;
use crate::node_state::NodeStatePath;

const PARTIAL_STATE_MAGIC: &str = "MCPS001";
const BLAKE3_HEX_LENGTH: usize = 64;
const MAX_PARTIAL_STATE_BYTES: u64 = 262_144;
const ABSENT_VALUE: &str = "-";

// r[impl molten.content_store_adapter.partial_state]
pub fn persist_partial_state(
    namespace: &NodeStateNamespace,
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    state: &ContentPartialState,
) -> Result<()> {
    let issues = validate_partial_state(profile, manifest, state);
    if !issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!("partial state persistence denied: {issues:?}")));
    }
    let encoded = encode_partial_state(state);
    let encoded_length = u64::try_from(encoded.len())
        .map_err(|_| MoltenError::invalid_harness("partial state length does not fit u64"))?;
    if encoded_length > MAX_PARTIAL_STATE_BYTES {
        return Err(MoltenError::invalid_harness("partial state exceeds persistence bound"));
    }
    namespace.write(&partial_state_path(&state.operation_ref)?, encoded.as_bytes())
}

pub fn load_partial_state(
    namespace: &NodeStateNamespace,
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    operation_ref: &str,
) -> Result<Option<ContentPartialState>> {
    let path = partial_state_path(operation_ref)?;
    let bytes = match namespace.observe_file(&path)? {
        NodeStateFileObservation::Missing => return Ok(None),
        NodeStateFileObservation::Regular(file) => file.read_bounded(MAX_PARTIAL_STATE_BYTES)?,
        NodeStateFileObservation::NonRegular(kind) => {
            return Err(MoltenError::invalid_harness(format!("partial state must be a regular file, got {kind:?}")));
        }
    };
    let text = std::str::from_utf8(&bytes).map_err(|_| MoltenError::invalid_harness("partial state is not UTF-8"))?;
    let state = decode_partial_state(text)?;
    if state.operation_ref != operation_ref {
        return Err(MoltenError::invalid_harness("partial state operation binding mismatch"));
    }
    let issues = validate_partial_state(profile, manifest, &state);
    if !issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!("persisted partial state denied: {issues:?}")));
    }
    Ok(Some(state))
}

pub fn remove_partial_state(namespace: &NodeStateNamespace, operation_ref: &str) -> Result<()> {
    let path = partial_state_path(operation_ref)?;
    match namespace.observe_file(&path)? {
        NodeStateFileObservation::Missing => Ok(()),
        NodeStateFileObservation::Regular(_) => namespace.remove_regular_file(&path),
        NodeStateFileObservation::NonRegular(kind) => {
            Err(MoltenError::invalid_harness(format!("partial state removal requires a regular file, got {kind:?}")))
        }
    }
}

fn encode_partial_state(state: &ContentPartialState) -> String {
    let mut lines = vec![
        PARTIAL_STATE_MAGIC.to_string(),
        state.schema.clone(),
        state.operation_ref.clone(),
        state.manifest_ref.clone(),
        state.profile_ref.clone(),
        state.generation.to_string(),
        state.terminal.as_str().to_string(),
        state.verified_chunk_refs.len().to_string(),
    ];
    lines.extend(state.verified_chunk_refs.iter().cloned());
    lines.push(state.missing_chunk_refs.len().to_string());
    lines.extend(state.missing_chunk_refs.iter().cloned());
    lines.extend([
        state.verified_bytes.to_string(),
        state.event_count.to_string(),
        state.last_sequence.map_or_else(|| ABSENT_VALUE.to_string(), |value| value.to_string()),
        state.failure.map_or(ABSENT_VALUE, ContentFailure::as_str).to_string(),
    ]);
    lines.join("\n")
}

fn decode_partial_state(text: &str) -> Result<ContentPartialState> {
    let mut lines = text.lines();
    require_line(&mut lines, "partial-state-magic", PARTIAL_STATE_MAGIC)?;
    let schema = next_line(&mut lines, "partial-state-schema")?.to_string();
    let operation_ref = next_line(&mut lines, "partial-state-operation-ref")?.to_string();
    let manifest_ref = next_line(&mut lines, "partial-state-manifest-ref")?.to_string();
    let profile_ref = next_line(&mut lines, "partial-state-profile-ref")?.to_string();
    let generation = parse_u64(next_line(&mut lines, "partial-state-generation")?, "partial-state-generation")?;
    let terminal = parse_terminal(next_line(&mut lines, "partial-state-terminal")?)?;
    let verified_count =
        parse_usize(next_line(&mut lines, "partial-state-verified-count")?, "partial-state-verified-count")?;
    let verified_chunk_refs = take_lines(&mut lines, verified_count, "partial-state-verified-ref")?;
    let missing_count =
        parse_usize(next_line(&mut lines, "partial-state-missing-count")?, "partial-state-missing-count")?;
    let missing_chunk_refs = take_lines(&mut lines, missing_count, "partial-state-missing-ref")?;
    let verified_bytes =
        parse_u64(next_line(&mut lines, "partial-state-verified-bytes")?, "partial-state-verified-bytes")?;
    let event_count = parse_usize(next_line(&mut lines, "partial-state-event-count")?, "partial-state-event-count")?;
    let last_sequence = parse_optional_u64(next_line(&mut lines, "partial-state-last-sequence")?)?;
    let failure = parse_optional_failure(next_line(&mut lines, "partial-state-failure")?)?;
    if lines.next().is_some() {
        return Err(MoltenError::invalid_harness("partial state has trailing fields"));
    }
    Ok(ContentPartialState {
        schema,
        operation_ref,
        manifest_ref,
        profile_ref,
        generation,
        terminal,
        verified_chunk_refs,
        missing_chunk_refs,
        verified_bytes,
        event_count,
        last_sequence,
        failure,
    })
}

fn partial_state_path(operation_ref: &str) -> Result<NodeStatePath> {
    let hex = operation_ref
        .strip_prefix("blake3:")
        .filter(|value| value.len() == BLAKE3_HEX_LENGTH)
        .ok_or_else(|| MoltenError::invalid_harness("partial state operation ref is malformed"))?;
    NodeStatePath::parse(&format!("content-partial-{hex}.state"))
}

fn require_line<'a>(lines: &mut impl Iterator<Item = &'a str>, field: &str, expected: &str) -> Result<()> {
    let actual = next_line(lines, field)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{field} mismatch")))
    }
}

fn next_line<'a>(lines: &mut impl Iterator<Item = &'a str>, field: &str) -> Result<&'a str> {
    lines.next().ok_or_else(|| MoltenError::invalid_harness(format!("partial state lacks {field}")))
}

fn take_lines<'a>(lines: &mut impl Iterator<Item = &'a str>, count: usize, field: &str) -> Result<Vec<String>> {
    if count > MAX_PARTIAL_STATE_BYTES as usize {
        return Err(MoltenError::invalid_harness(format!("{field} count exceeds bound")));
    }
    (0..count).map(|_| next_line(lines, field).map(str::to_string)).collect()
}

fn parse_u64(value: &str, field: &str) -> Result<u64> {
    value.parse().map_err(|_| MoltenError::invalid_harness(format!("{field} is not u64")))
}

fn parse_usize(value: &str, field: &str) -> Result<usize> {
    value.parse().map_err(|_| MoltenError::invalid_harness(format!("{field} is not usize")))
}

fn parse_optional_u64(value: &str) -> Result<Option<u64>> {
    if value == ABSENT_VALUE {
        Ok(None)
    } else {
        parse_u64(value, "partial-state-last-sequence").map(Some)
    }
}

fn parse_terminal(value: &str) -> Result<ContentTerminal> {
    match value {
        "accepted" => Ok(ContentTerminal::Accepted),
        "streaming" => Ok(ContentTerminal::Streaming),
        "verified" => Ok(ContentTerminal::Verified),
        "durable" => Ok(ContentTerminal::Durable),
        "cancelled" => Ok(ContentTerminal::Cancelled),
        "retryable" => Ok(ContentTerminal::Retryable),
        "failed" => Ok(ContentTerminal::Failed),
        "uncertain" => Ok(ContentTerminal::Uncertain),
        "denied" => Ok(ContentTerminal::Denied),
        _ => Err(MoltenError::invalid_harness("partial state terminal is unsupported")),
    }
}

fn parse_optional_failure(value: &str) -> Result<Option<ContentFailure>> {
    if value == ABSENT_VALUE {
        return Ok(None);
    }
    let failure = match value {
        "corrupt-chunk" => ContentFailure::CorruptChunk,
        "truncated-chunk" => ContentFailure::TruncatedChunk,
        "reordered-chunk" => ContentFailure::ReorderedChunk,
        "unexpected-chunk" => ContentFailure::UnexpectedChunk,
        "stale-ticket" => ContentFailure::StaleTicket,
        "unsupported-transform" => ContentFailure::UnsupportedTransform,
        "root-escape" => ContentFailure::RootEscape,
        "overload" => ContentFailure::Overload,
        "permission-denied" => ContentFailure::PermissionDenied,
        "timeout" => ContentFailure::Timeout,
        "transport-disconnected" => ContentFailure::TransportDisconnected,
        "adapter-failure" => ContentFailure::AdapterFailure,
        _ => return Err(MoltenError::invalid_harness("partial state failure is unsupported")),
    };
    Ok(Some(failure))
}
