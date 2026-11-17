// SPDX-License-Identifier: AGPL-3.0-or-later
use alloc::collections::BTreeSet;
use alloc::string::String;

use serde::Deserialize;
use serde::Serialize;

use super::*;
/// Explicit operator grant for this standalone deployment profile. This is not
/// a Basalt token, provenance attestation, or a Molten system-tier activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub profile: String,
    pub cluster: Id,
    pub generation: u64,
    pub allow_any_pure_component: bool,
    pub allowed_components: BTreeSet<Id>,
    pub allowed_pure_net_manifests: BTreeSet<Id>,
    pub allowed_pure_net_programs: BTreeSet<Id>,
    pub max_memory_bytes: u64,
    pub max_fuel: u64,
    pub max_timeout_ms: u64,
    pub max_output_bytes: u64,
}
pub fn validate_policy(p: &Policy) -> Result<(), Violation> {
    if !matches!(p.profile.as_str(), PROFILE | PURE_NET_PROFILE)
        || (p.profile == PROFILE
            && (!p.allowed_pure_net_manifests.is_empty() || !p.allowed_pure_net_programs.is_empty()))
        || (p.profile == PURE_NET_PROFILE && (p.allow_any_pure_component || !p.allowed_components.is_empty()))
        || p.generation == 0
        || p.allowed_components.len() > 4096
        || p.allowed_pure_net_manifests.len() > 4096
        || p.allowed_pure_net_programs.len() > 4096
        || p.max_memory_bytes < 65_536
        || p.max_memory_bytes > 1024 * 1024 * 1024
        || p.max_fuel == 0
        || p.max_fuel > 100_000_000_000
        || p.max_timeout_ms == 0
        || p.max_timeout_ms > 3_600_000
        || p.max_output_bytes > MAX_CONTENT_BYTES as u64
    {
        return Err(Violation::Malformed("policy"));
    }
    Ok(())
}
pub fn policy_allows_task(p: &Policy, revoked: &BTreeSet<Id>, t: &TaskSpec) -> Result<bool, Violation> {
    validate_policy(p)?;
    Ok(policy_allows_workload(p, revoked, t.workload)
        && t.resources.memory_bytes <= p.max_memory_bytes
        && t.fuel <= p.max_fuel
        && t.timeout_ms <= p.max_timeout_ms
        && t.max_output_bytes <= p.max_output_bytes)
}
pub fn policy_allows_request(
    p: &Policy,
    revoked: &BTreeSet<Id>,
    r: &ExecutionRequest,
    execution: bool,
) -> Result<bool, Violation> {
    validate_policy(p)?;
    validate_request(r)?;
    if r.fence.cluster != p.cluster || r.fence.generation != p.generation {
        return Ok(false);
    }
    // Revocation cannot prevent requesting a stop or recording a stopped result.
    if !execution {
        return Ok(true);
    }
    Ok(policy_allows_workload(p, revoked, r.workload)
        && r.resources.memory_bytes <= p.max_memory_bytes
        && r.fuel <= p.max_fuel
        && r.timeout_ms <= p.max_timeout_ms
        && r.max_output_bytes <= p.max_output_bytes)
}

fn policy_allows_workload(p: &Policy, revoked: &BTreeSet<Id>, workload: Workload) -> bool {
    match workload {
        Workload::Component { artifact } => {
            p.profile == PROFILE
                && !revoked.contains(&artifact)
                && (p.allow_any_pure_component || p.allowed_components.contains(&artifact))
        }
        Workload::PureNet { manifest, artifact } => {
            p.profile == PURE_NET_PROFILE
                && !revoked.contains(&manifest)
                && !revoked.contains(&artifact)
                && p.allowed_pure_net_manifests.contains(&manifest)
                && p.allowed_pure_net_programs.contains(&artifact)
        }
    }
}
