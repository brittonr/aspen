#![allow(
    tigerstyle::borrowed_argument_types,
    reason = "the canonical header writer appends to one profile-bounded byte vector"
)]

use super::super::*;
use super::codec::*;

const EMPTY_BYTES: usize = 0;

// r[impl molten.prolly_map.canonical_nodes]
pub fn encode_leaf(profile: &ProllyProfile, entries: &[SemanticEntry]) -> Result<EncodedBlock, Vec<ProllyIssue>> {
    let mut issues = validate_entries(profile, entries);
    if length_exceeds(entries.len(), profile.limits.max_entries) {
        issues.push(ProllyIssue::EntryLimitExceeded);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let bytes = leaf_bytes(profile, entries).map_err(|issue| vec![issue])?;
    enforce_node_size(profile, bytes.len()).map_err(|issue| vec![issue])?;
    let node_ref = node_identity(&profile.leaf_domain, &profile.profile_ref, &bytes).map_err(|issue| vec![issue])?;
    Ok(EncodedBlock { node_ref, bytes })
}

pub fn encode_internal(profile: &ProllyProfile, children: &[ChildRange]) -> Result<EncodedBlock, Vec<ProllyIssue>> {
    let issues = validate_children(profile, children);
    if !issues.is_empty() {
        return Err(issues);
    }
    let bytes = internal_bytes(profile, children).map_err(|issue| vec![issue])?;
    enforce_node_size(profile, bytes.len()).map_err(|issue| vec![issue])?;
    let node_ref =
        node_identity(&profile.internal_domain, &profile.profile_ref, &bytes).map_err(|issue| vec![issue])?;
    Ok(EncodedBlock { node_ref, bytes })
}

pub fn decode_block(profile: &ProllyProfile, block: &EncodedBlock) -> Result<ProllyNode, Vec<ProllyIssue>> {
    let mut cursor = Cursor::new(&block.bytes);
    let magic = cursor.take(NODE_MAGIC.len()).map_err(|issue| vec![issue])?;
    if magic != NODE_MAGIC {
        return Err(vec![ProllyIssue::NodeEncodingMalformed("magic")]);
    }
    let version = cursor.u32().map_err(|issue| vec![issue])?;
    if version != profile.format_version {
        return Err(vec![ProllyIssue::NodeEncodingMalformed("version")]);
    }
    let kind = cursor.u8().map_err(|issue| vec![issue])?;
    let profile_ref = cursor.string_u16().map_err(|issue| vec![issue])?;
    if profile_ref != profile.profile_ref.as_str() {
        return Err(vec![ProllyIssue::NodeProfileMismatch]);
    }
    let count = cursor.u32().map_err(|issue| vec![issue])?;
    let node = match kind {
        LEAF_KIND => decode_leaf(profile, block, &mut cursor, count)?,
        INTERNAL_KIND => decode_internal(profile, block, &mut cursor, count)?,
        _ => return Err(vec![ProllyIssue::NodeEncodingMalformed("kind")]),
    };
    if cursor.remaining() != EMPTY_BYTES {
        return Err(vec![ProllyIssue::NodeEncodingTrailingBytes]);
    }
    Ok(node)
}

pub fn validate_entries(profile: &ProllyProfile, entries: &[SemanticEntry]) -> Vec<ProllyIssue> {
    let mut issues = Vec::new();
    let mut prior: Option<&[u8]> = None;
    for entry in entries {
        if entry.key.is_empty() {
            issues.push(ProllyIssue::EmptyKey);
        }
        if length_exceeds(entry.key.len(), profile.limits.max_key_bytes) {
            issues.push(ProllyIssue::KeyLimitExceeded);
        }
        if length_exceeds(entry.value.len(), profile.limits.max_value_bytes) {
            issues.push(ProllyIssue::ValueLimitExceeded);
        }
        if prior.is_some_and(|key| key >= entry.key.as_slice()) {
            issues.push(if prior == Some(entry.key.as_slice()) {
                ProllyIssue::DuplicateKey
            } else {
                ProllyIssue::EntriesNotStrictlySorted
            });
        }
        prior = Some(&entry.key);
    }
    issues.sort();
    issues.dedup();
    issues
}

pub fn leaf_encoded_len(profile: &ProllyProfile, entries: &[SemanticEntry]) -> Result<u32, ProllyIssue> {
    let bytes = leaf_bytes(profile, entries)?;
    u32::try_from(bytes.len()).map_err(|_| ProllyIssue::NodeSizeLimitExceeded)
}

pub fn internal_encoded_len(profile: &ProllyProfile, children: &[ChildRange]) -> Result<u32, ProllyIssue> {
    let bytes = internal_bytes(profile, children)?;
    u32::try_from(bytes.len()).map_err(|_| ProllyIssue::NodeSizeLimitExceeded)
}

fn leaf_bytes(profile: &ProllyProfile, entries: &[SemanticEntry]) -> Result<Vec<u8>, ProllyIssue> {
    let capacity = usize::try_from(profile.max_node_bytes).map_err(|_| ProllyIssue::NodeSizeLimitExceeded)?;
    let mut bytes = Vec::with_capacity(capacity);
    header(&mut bytes, profile, LEAF_KIND, entries.len())?;
    for entry in entries {
        put_bytes_u16(&mut bytes, &entry.key)?;
        put_bytes_u32(&mut bytes, &entry.value)?;
    }
    Ok(bytes)
}

fn internal_bytes(profile: &ProllyProfile, children: &[ChildRange]) -> Result<Vec<u8>, ProllyIssue> {
    let capacity = usize::try_from(profile.max_node_bytes).map_err(|_| ProllyIssue::NodeSizeLimitExceeded)?;
    let mut bytes = Vec::with_capacity(capacity);
    header(&mut bytes, profile, INTERNAL_KIND, children.len())?;
    for child in children {
        put_bytes_u16(&mut bytes, &child.min_key)?;
        put_bytes_u16(&mut bytes, &child.max_key)?;
        put_bytes_u16(&mut bytes, child.node_ref.as_str().as_bytes())?;
        bytes.extend_from_slice(&child.encoded_len.to_le_bytes());
    }
    Ok(bytes)
}

fn header(bytes: &mut Vec<u8>, profile: &ProllyProfile, kind: u8, count: usize) -> Result<(), ProllyIssue> {
    bytes.extend_from_slice(&NODE_MAGIC);
    bytes.extend_from_slice(&profile.format_version.to_le_bytes());
    bytes.push(kind);
    put_bytes_u16(bytes, profile.profile_ref.as_str().as_bytes())?;
    let count = u32::try_from(count).map_err(|_| ProllyIssue::EntryLimitExceeded)?;
    bytes.extend_from_slice(&count.to_le_bytes());
    Ok(())
}

fn decode_leaf(
    profile: &ProllyProfile,
    block: &EncodedBlock,
    cursor: &mut Cursor<'_>,
    count: u32,
) -> Result<ProllyNode, Vec<ProllyIssue>> {
    if count > profile.limits.max_entries {
        return Err(vec![ProllyIssue::EntryLimitExceeded]);
    }
    let count = usize::try_from(count).map_err(|_| vec![ProllyIssue::EntryLimitExceeded])?;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        entries.push(SemanticEntry {
            key: cursor.bytes_u16().map_err(|issue| vec![issue])?,
            value: cursor.bytes_u32().map_err(|issue| vec![issue])?,
        });
    }
    let issues = validate_entries(profile, &entries);
    if !issues.is_empty() {
        return Err(issues);
    }
    validate_decoded_identity(profile, block, &profile.leaf_domain)?;
    let encoded_len = u32::try_from(block.bytes.len()).map_err(|_| vec![ProllyIssue::NodeSizeLimitExceeded])?;
    Ok(ProllyNode::Leaf(LeafNode {
        schema: PROLLY_LEAF_SCHEMA.to_string(),
        profile_ref: profile.profile_ref.clone(),
        entries,
        encoded_len,
        node_ref: block.node_ref.clone(),
    }))
}

fn decode_internal(
    profile: &ProllyProfile,
    block: &EncodedBlock,
    cursor: &mut Cursor<'_>,
    count: u32,
) -> Result<ProllyNode, Vec<ProllyIssue>> {
    if count == 0 || count > u32::from(profile.max_fanout) {
        return Err(vec![ProllyIssue::InternalFanoutExceeded]);
    }
    let count = usize::try_from(count).map_err(|_| vec![ProllyIssue::InternalFanoutExceeded])?;
    let mut children = Vec::with_capacity(count);
    for _ in 0..count {
        let min_key = cursor.bytes_u16().map_err(|issue| vec![issue])?;
        let max_key = cursor.bytes_u16().map_err(|issue| vec![issue])?;
        let node_ref = cursor.string_u16().map_err(|issue| vec![issue])?;
        let encoded_len = cursor.u32().map_err(|issue| vec![issue])?;
        children.push(ChildRange {
            min_key,
            max_key,
            node_ref: NodeRef::new(node_ref),
            encoded_len,
        });
    }
    let issues = validate_children(profile, &children);
    if !issues.is_empty() {
        return Err(issues);
    }
    validate_decoded_identity(profile, block, &profile.internal_domain)?;
    let encoded_len = u32::try_from(block.bytes.len()).map_err(|_| vec![ProllyIssue::NodeSizeLimitExceeded])?;
    Ok(ProllyNode::Internal(InternalNode {
        schema: PROLLY_INTERNAL_SCHEMA.to_string(),
        profile_ref: profile.profile_ref.clone(),
        children,
        encoded_len,
        node_ref: block.node_ref.clone(),
    }))
}

fn validate_children(profile: &ProllyProfile, children: &[ChildRange]) -> Vec<ProllyIssue> {
    let mut issues = Vec::new();
    if children.is_empty() {
        issues.push(ProllyIssue::EmptyInternalNode);
    }
    if children.len() > usize::from(profile.max_fanout) {
        issues.push(ProllyIssue::InternalFanoutExceeded);
    }
    let mut prior_max: Option<&[u8]> = None;
    for child in children {
        if child.min_key.is_empty() || child.min_key > child.max_key || !is_content_ref(child.node_ref.as_str()) {
            issues.push(ProllyIssue::ChildRangeInvalid);
        }
        if prior_max.is_some_and(|prior| prior >= child.min_key.as_slice()) {
            issues.push(ProllyIssue::ChildRangeOverlap);
        }
        prior_max = Some(&child.max_key);
    }
    issues.sort();
    issues.dedup();
    issues
}

fn validate_decoded_identity(
    profile: &ProllyProfile,
    block: &EncodedBlock,
    domain: &str,
) -> Result<(), Vec<ProllyIssue>> {
    enforce_node_size(profile, block.bytes.len()).map_err(|issue| vec![issue])?;
    let expected = node_identity(domain, &profile.profile_ref, &block.bytes).map_err(|issue| vec![issue])?;
    if expected != block.node_ref {
        return Err(vec![ProllyIssue::NodeIdentityMismatch]);
    }
    Ok(())
}

fn enforce_node_size(profile: &ProllyProfile, size: usize) -> Result<(), ProllyIssue> {
    let maximum = usize::try_from(profile.max_node_bytes).map_err(|_| ProllyIssue::NodeSizeLimitExceeded)?;
    if size > maximum {
        return Err(ProllyIssue::NodeSizeLimitExceeded);
    }
    Ok(())
}

fn length_exceeds(length: usize, maximum: u32) -> bool {
    match u32::try_from(length) {
        Ok(length) => length > maximum,
        Err(_) => true,
    }
}

fn node_identity(domain: &str, profile_ref: &ProfileRef, bytes: &[u8]) -> Result<NodeRef, ProllyIssue> {
    let mut hasher = hasher(domain)?;
    hasher.update_tagged_str(b"profile-ref", profile_ref.as_str()).map_err(identity_issue)?;
    hasher.update_tagged_bytes(b"node-bytes", bytes).map_err(identity_issue)?;
    Ok(NodeRef::new(format!("blake3:{}", hasher.finish().to_hex())))
}
