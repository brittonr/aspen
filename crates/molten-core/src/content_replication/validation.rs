#![allow(
    tigerstyle::non_trait_imports,
    reason = "validation uses ordered sets to bound and stabilize diagnostics"
)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;

const BLAKE3_PREFIX: &str = "blake3:";
const BLAKE3_HEX_LENGTH: usize = 64;

pub fn validate_input(input: &ReconcileInput) -> Vec<Issue> {
    let mut issues = BTreeSet::new();
    validate_manifest_fields(&input.manifest, &mut issues);
    validate_peers(input, &mut issues);
    validate_inventory(input, &mut issues);
    validate_history(input, &mut issues);
    issues.into_iter().collect()
}

pub fn validate_manifest(manifest: &Manifest) -> Vec<Issue> {
    let mut issues = BTreeSet::new();
    validate_manifest_fields(manifest, &mut issues);
    issues.into_iter().collect()
}

fn validate_manifest_fields(manifest: &Manifest, issues: &mut BTreeSet<Issue>) {
    if !valid_id(&manifest.service_id)
        || manifest.generation == 0
        || manifest.membership_epoch == 0
        || manifest.placement_epoch == 0
        || manifest.contents.is_empty()
    {
        issues.insert(Issue::InvalidManifest);
    }
    for reference in manifest_refs(manifest) {
        if !valid_ref(reference) {
            issues.insert(Issue::InvalidReference);
        }
    }
    validate_ports(manifest, issues);
    validate_policy(manifest, issues);
    validate_contents(manifest, issues);
    let expected_non_claims = NON_CLAIMS.iter().map(ToString::to_string).collect::<Vec<_>>();
    if manifest.non_claims != expected_non_claims {
        issues.insert(Issue::InvalidManifest);
    }
}

fn manifest_refs(manifest: &Manifest) -> [&str; 6] {
    [
        &manifest.authority_ref,
        &manifest.identity_ref,
        &manifest.content_profile_ref,
        &manifest.transport_profile_ref,
        &manifest.retention_policy_ref,
        &manifest.evidence_profile_ref,
    ]
}

fn validate_ports(manifest: &Manifest, issues: &mut BTreeSet<Issue>) {
    let ports = manifest.ports.iter().map(String::as_str).collect::<BTreeSet<_>>();
    if ports.len() != manifest.ports.len() || REQUIRED_PORTS.iter().any(|port| !ports.contains(port)) {
        issues.insert(Issue::MissingPort);
    }
}

fn validate_policy(manifest: &Manifest, issues: &mut BTreeSet<Issue>) {
    let policy = &manifest.policy;
    if policy.desired_replicas == 0
        || policy.minimum_verified_replicas == 0
        || policy.minimum_verified_replicas > policy.desired_replicas
        || policy.desired_replicas > MAX_PEERS
        || policy.minimum_fault_domains == 0
        || policy.minimum_fault_domains > policy.desired_replicas
        || policy.minimum_fault_domains > MAX_FAULT_DOMAINS
        || manifest.repair.max_attempts == 0
        || manifest.repair.max_attempts > MAX_REPAIR_ATTEMPTS
    {
        issues.insert(Issue::InvalidPolicy);
    }
    let resources = &manifest.resources;
    if resources.max_concurrent_transfers == 0
        || resources.max_transfer_bytes == 0
        || resources.max_transfer_bytes > MAX_REPLICATION_BYTES
        || resources.max_queue_depth == 0
        || resources.max_queue_depth > MAX_QUEUE_DEPTH
        || resources.max_timers == 0
        || resources.max_diagnostics == 0
        || resources.max_diagnostics > MAX_DIAGNOSTICS
    {
        issues.insert(Issue::InvalidResourceLimit);
    }
}

fn validate_contents(manifest: &Manifest, issues: &mut BTreeSet<Issue>) {
    if manifest.contents.len() > MAX_CONTENTS {
        issues.insert(Issue::TooManyContents);
    }
    let mut refs = BTreeSet::new();
    for content in &manifest.contents {
        if !refs.insert(&content.content_ref) {
            issues.insert(Issue::DuplicateContent);
        }
        if content.encoded_bytes == 0
            || content.encoded_bytes > manifest.resources.max_transfer_bytes
            || !valid_ref(&content.content_ref)
            || !valid_ref(&content.manifest_ref)
            || content.transform_ref.as_ref().is_some_and(|reference| !valid_ref(reference))
            || content.cleanup_authority_ref.as_ref().is_some_and(|reference| !valid_ref(reference))
        {
            issues.insert(Issue::InvalidReference);
        }
    }
}

fn validate_peers(input: &ReconcileInput, issues: &mut BTreeSet<Issue>) {
    if input.peers.len() > MAX_PEERS {
        issues.insert(Issue::TooManyPeers);
    }
    let mut peers = BTreeSet::new();
    for peer in &input.peers {
        if !peers.insert(&peer.peer_id) {
            issues.insert(Issue::DuplicatePeer);
        }
        if !valid_id(&peer.peer_id)
            || !valid_id(&peer.fault_domain)
            || peer.membership_epoch == 0
            || peer.placement_epoch == 0
        {
            issues.insert(Issue::InvalidManifest);
        }
    }
}

fn validate_inventory(input: &ReconcileInput, issues: &mut BTreeSet<Issue>) {
    let mut replicas = BTreeSet::new();
    for replica in &input.inventory.replicas {
        if !replicas.insert((&replica.content_ref, &replica.peer_id)) {
            issues.insert(Issue::DuplicateReplica);
        }
        if !valid_ref(&replica.content_ref)
            || !valid_ref(&replica.manifest_ref)
            || !valid_id(&replica.peer_id)
            || !valid_id(&replica.fault_domain)
            || replica.cleanup_clearance_ref.as_ref().is_some_and(|reference| !valid_ref(reference))
        {
            issues.insert(Issue::InvalidReference);
        }
    }
}

fn validate_history(input: &ReconcileInput, issues: &mut BTreeSet<Issue>) {
    let mut operations = BTreeMap::new();
    for operation in &input.history {
        if !valid_ref(&operation.operation_id)
            || !valid_ref(&operation.content_ref)
            || !valid_id(&operation.target_peer)
            || operation.source_peer.as_ref().is_some_and(|peer| !valid_id(peer))
            || operation.result_ref.as_ref().is_some_and(|reference| !valid_ref(reference))
        {
            issues.insert(Issue::InvalidReference);
        }
        if let Some(prior) = operations.insert(&operation.operation_id, operation) {
            if prior == operation {
                issues.insert(Issue::DuplicateOperation);
            } else {
                issues.insert(Issue::ConflictingOperation);
            }
        }
    }
}

pub fn valid_ref(value: &str) -> bool {
    value.strip_prefix(BLAKE3_PREFIX).is_some_and(|hex| {
        hex.len() == BLAKE3_HEX_LENGTH && hex.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

pub fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_ID_BYTES && !value.chars().any(char::is_control)
}
