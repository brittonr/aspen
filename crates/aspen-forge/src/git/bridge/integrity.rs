//! DAG integrity verification for federation mirrors.
//!
//! Walks the object graph from ref heads and verifies all referenced objects
//! are reachable in the KV store. Used as a post-import diagnostic to catch
//! gaps before clients attempt to clone.

use std::collections::HashSet;
use std::collections::VecDeque;

use aspen_core::KeyValueStore;

use super::constants;
use crate::git::object::GitObject;
use crate::identity::RepoId;
use crate::types::SignedObject;

/// Result of a DAG integrity verification.
#[derive(Debug)]
pub struct DagIntegrityResult {
    /// Total objects stored under `forge:obj:{repo}:*`.
    pub total_stored: u32,
    /// Objects reachable from ref heads.
    pub reachable: u32,
    /// BLAKE3 hashes referenced but not found in KV.
    pub missing: Vec<String>,
    /// Ref heads found: (ref_name, blake3_hash).
    pub ref_heads: Vec<(String, [u8; 32])>,
}

impl DagIntegrityResult {
    /// Whether the DAG is complete (no missing objects).
    pub fn is_complete(&self) -> bool {
        self.missing.is_empty()
    }
}

/// Verify DAG integrity for a repository.
///
/// Walks from each ref head, reading objects from KV and following
/// child references. Returns counts of stored, reachable, and missing
/// objects.
pub async fn verify_dag_integrity(kv: &dyn KeyValueStore, repo_id: &RepoId) -> DagIntegrityResult {
    let repo_hex = repo_id.to_hex();
    let obj_prefix = format!("{}{}:", constants::KV_PREFIX_OBJ, repo_hex);
    let refs_prefix = format!("forge:refs:{}:", repo_hex);

    // Count stored objects
    let stored_count = count_kv_keys(kv, &obj_prefix).await;

    // Scan ref heads
    let ref_heads = scan_ref_heads(kv, &refs_prefix).await;

    // BFS from each ref head
    let mut reachable: HashSet<[u8; 32]> = HashSet::new();
    let mut queue: VecDeque<[u8; 32]> = VecDeque::new();
    let mut missing: Vec<String> = Vec::new();

    for (_ref_name, head_hash) in &ref_heads {
        if !reachable.contains(head_hash) {
            queue.push_back(*head_hash);
        }
    }

    while let Some(hash) = queue.pop_front() {
        if reachable.contains(&hash) {
            continue;
        }
        reachable.insert(hash);

        // Read the object from KV
        let obj_key = format!("{}{}", obj_prefix, hex::encode(hash));
        let obj_data = match kv.read(aspen_core::ReadRequest::new(obj_key)).await {
            Ok(r) => r.kv.map(|kv| kv.value),
            Err(_) => None,
        };

        let Some(obj_value) = obj_data else {
            missing.push(hex::encode(hash));
            continue;
        };

        // Decode object bytes (handle chunked storage)
        let decoded = decode_kv_object(kv, &format!("{}{}", obj_prefix, hex::encode(hash)), &obj_value).await;
        let Some(obj_bytes) = decoded else {
            missing.push(hex::encode(hash));
            continue;
        };

        // Parse and extract child references
        let Ok(signed) = SignedObject::<GitObject>::from_bytes(&obj_bytes) else {
            continue;
        };

        match &signed.payload {
            GitObject::Commit(c) => {
                queue.push_back(c.tree);
                for p in &c.parents {
                    queue.push_back(*p);
                }
            }
            GitObject::Tree(t) => {
                for entry in &t.entries {
                    if !entry.is_gitlink() {
                        queue.push_back(entry.hash);
                    }
                }
            }
            GitObject::Tag(tag) => {
                queue.push_back(tag.target);
            }
            GitObject::Blob(_) => {}
        }
    }

    DagIntegrityResult {
        total_stored: stored_count,
        reachable: reachable.len() as u32,
        missing,
        ref_heads,
    }
}

/// Decode a KV object value, handling chunked storage.
async fn decode_kv_object(kv: &dyn KeyValueStore, obj_key: &str, value: &str) -> Option<Vec<u8>> {
    use base64::Engine as _;

    if let Some(rest) = value.strip_prefix("chunks:") {
        let num_chunks: usize = rest.parse().ok()?;
        let mut combined = String::new();
        for i in 0..num_chunks {
            let chunk_key = format!("{}:c:{}", obj_key, i);
            let chunk_data = match kv.read(aspen_core::ReadRequest::new(chunk_key)).await {
                Ok(r) => r.kv.map(|kv| kv.value),
                Err(_) => None,
            };
            combined.push_str(chunk_data.as_deref().unwrap_or(""));
        }
        base64::engine::general_purpose::STANDARD.decode(&combined).ok()
    } else {
        base64::engine::general_purpose::STANDARD.decode(value).ok()
    }
}

/// Count KV keys with a given prefix (up to 50K).
async fn count_kv_keys(kv: &dyn KeyValueStore, prefix: &str) -> u32 {
    match kv
        .scan(aspen_core::ScanRequest {
            prefix: prefix.to_string(),
            limit_results: Some(50_000),
            continuation_token: None,
        })
        .await
    {
        Ok(r) => r.entries.len() as u32,
        Err(_) => 0,
    }
}

/// Scan ref heads from KV.
async fn scan_ref_heads(kv: &dyn KeyValueStore, refs_prefix: &str) -> Vec<(String, [u8; 32])> {
    match kv
        .scan(aspen_core::ScanRequest {
            prefix: refs_prefix.to_string(),
            limit_results: Some(100),
            continuation_token: None,
        })
        .await
    {
        Ok(r) => r
            .entries
            .into_iter()
            .filter_map(|e| {
                let ref_name = e.key.strip_prefix(&format!("{}:", refs_prefix.trim_end_matches(':')))?;
                let hash_bytes = hex::decode(e.value.trim()).ok()?;
                if hash_bytes.len() == 32 {
                    let mut h = [0u8; 32];
                    h.copy_from_slice(&hash_bytes);
                    Some((ref_name.to_string(), h))
                } else {
                    None
                }
            })
            .collect(),
        Err(_) => vec![],
    }
}
