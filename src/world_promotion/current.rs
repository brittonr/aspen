//! Signed local authority and intent-closure facts for one promotion transaction.
//!
//! The files are explicit operator inputs. They are verified before use and do
//! not turn a caller-provided boolean into authority.

use std::collections::BTreeSet;
use std::path::Path;

use ed25519_dalek::Signature;
use ed25519_dalek::Verifier;
use ed25519_dalek::VerifyingKey;
use molten_core::world_head::WorldHeadPolicyRef;
use molten_core::world_head::WorldHeadState;
use molten_core::world_promotion::WorldPromotionPlan;
use molten_core::world_promotion::WorldPromotionTransactionFacts;
use molten_core::world_promotion::*;
use molten_node_host::node_state::DirectoryRoot;
use molten_node_host::node_state::NamespaceKind;
use serde::Deserialize;
use serde::Serialize;

use super::WorldPromotionCurrentPort;
use crate::error::Failure;
use crate::error::Result;

const AUTHORITY_SCHEMA: &str = "molten.world-promotion.authority-grant.v1";
const CLOSURE_SCHEMA: &str = "molten.world-promotion.intent-closure.v1";
const AUTHORITY_DOMAIN: &[u8] = b"molten.world-promotion.authority-grant.v1";
const CLOSURE_DOMAIN: &[u8] = b"molten.world-promotion.intent-closure.v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignedPromotionAuthority {
    pub schema: String,
    pub branch_id: String,
    pub authority_ref: String,
    pub policy_ref: String,
    pub generation: u64,
    pub admitted: bool,
    pub signer: String,
    pub signature: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignedIntentClosure {
    pub schema: String,
    pub operation_ref: String,
    pub branch_id: String,
    pub candidate_head: String,
    pub expected_generation: u64,
    pub intent_refs: Vec<String>,
    pub complete: bool,
    pub signer: String,
    pub signature: String,
}

pub struct ObservedPromotionCurrent {
    pub observed_head: Option<WorldHeadState>,
    pub authority_ref: WorldPromotionAuthorityRef,
    pub authority_admitted: bool,
    pub authority_generation: u64,
    pub policy_ref: WorldHeadPolicyRef,
    pub closure: SignedIntentClosure,
}

impl WorldPromotionCurrentPort for ObservedPromotionCurrent {
    fn observe_transaction(&mut self, plan: &WorldPromotionPlan) -> Result<WorldPromotionTransactionFacts> {
        let expected_intents = plan.intents.iter().map(|intent| intent.intent_ref.as_str()).collect::<BTreeSet<_>>();
        let closure_intents = self.closure.intent_refs.iter().map(String::as_str).collect::<BTreeSet<_>>();
        let is_closure_intents_unique = closure_intents.len() == self.closure.intent_refs.len();
        let is_closure_complete = self.closure.complete
            && is_closure_intents_unique
            && self.closure.operation_ref == plan.operation_ref.as_str()
            && self.closure.branch_id == plan.before.branch_id.as_str()
            && self.closure.candidate_head == plan.after.head.as_str()
            && self.closure.expected_generation == plan.before.generation
            && expected_intents == closure_intents;
        Ok(WorldPromotionTransactionFacts {
            observed_head: self.observed_head.clone(),
            authority_ref: self.authority_ref.clone(),
            authority_admitted: self.authority_admitted,
            authority_generation: self.authority_generation,
            policy_ref: self.policy_ref.clone(),
            intent_closure_complete: is_closure_complete,
            reservation_refs: plan.reservations.iter().map(|reservation| reservation.reservation_ref.clone()).collect(),
        })
    }
}

pub fn read_observed_current(
    state_root: &Path,
    authority_path: &Path,
    closure_path: &Path,
    trusted_signer: &str,
    branch_id: &str,
) -> Result<ObservedPromotionCurrent> {
    let authority = read_authority(authority_path, trusted_signer)?;
    let closure = read_closure(closure_path, trusted_signer)?;
    if authority.branch_id != branch_id || closure.branch_id != branch_id {
        return Err(Failure::invalid_harness("signed promotion input names the wrong branch"));
    }
    let root = DirectoryRoot::open(state_root)?;
    let storage = root.namespace(NamespaceKind::Storage)?;
    let store = crate::world_promotion::LocalWorldPromotionStore::open(&storage)?;
    let branch = molten_core::world_head::WorldBranchId::new(branch_id.to_string())
        .map_err(|error| Failure::invalid_harness(format!("invalid branch id: {error}")))?;
    let observed_head = store
        .read_head(&branch)
        .map_err(|error| Failure::invalid_harness(format!("promotion head read failed: {error}")))?;
    Ok(ObservedPromotionCurrent {
        observed_head,
        authority_ref: authority.authority_ref()?,
        authority_admitted: authority.admitted,
        authority_generation: authority.generation,
        policy_ref: authority.policy_ref()?,
        closure,
    })
}

pub fn read_operator_authority(path: &Path, trusted_signer: &str) -> Result<SignedPromotionAuthority> {
    read_authority(path, trusted_signer)
}

pub(super) fn read_authority(path: &Path, trusted_signer: &str) -> Result<SignedPromotionAuthority> {
    let authority: SignedPromotionAuthority = read_json(path)?;
    if authority.schema != AUTHORITY_SCHEMA {
        return Err(Failure::invalid_harness("promotion authority schema is unsupported"));
    }
    if authority.authority_ref.trim().is_empty()
        || authority.policy_ref.trim().is_empty()
        || authority.generation == 0
        || authority.branch_id.trim().is_empty()
    {
        return Err(Failure::invalid_harness("promotion authority is incomplete"));
    }
    verify(trusted_signer, &authority.signer, &authority.signature, &authority_message(&authority))?;
    Ok(authority)
}

pub(super) fn read_closure(path: &Path, trusted_signer: &str) -> Result<SignedIntentClosure> {
    let closure: SignedIntentClosure = read_json(path)?;
    if closure.schema != CLOSURE_SCHEMA {
        return Err(Failure::invalid_harness("promotion intent-closure schema is unsupported"));
    }
    if closure.operation_ref.trim().is_empty()
        || closure.branch_id.trim().is_empty()
        || closure.candidate_head.trim().is_empty()
        || closure.expected_generation == 0
    {
        return Err(Failure::invalid_harness("promotion intent closure is incomplete"));
    }
    if closure.intent_refs.len() > molten_core::world_promotion::MAX_WORLD_PROMOTION_INTENTS {
        return Err(Failure::invalid_harness("promotion intent closure exceeds its bound"));
    }
    verify(trusted_signer, &closure.signer, &closure.signature, &closure_message(&closure))?;
    Ok(closure)
}

impl SignedPromotionAuthority {
    fn authority_ref(&self) -> Result<WorldPromotionAuthorityRef> {
        WorldPromotionAuthorityRef::new(self.authority_ref.clone())
            .map_err(|error| Failure::invalid_harness(format!("invalid authority ref: {error:?}")))
    }

    fn policy_ref(&self) -> Result<WorldHeadPolicyRef> {
        WorldHeadPolicyRef::new(self.policy_ref.clone())
            .map_err(|error| Failure::invalid_harness(format!("invalid policy ref: {error:?}")))
    }
}

pub(super) fn authority_message(authority: &SignedPromotionAuthority) -> Vec<u8> {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        String::from_utf8_lossy(AUTHORITY_DOMAIN),
        authority.branch_id,
        authority.authority_ref,
        authority.policy_ref,
        authority.generation,
        authority.admitted
    )
    .into_bytes()
}

pub(super) fn closure_message(closure: &SignedIntentClosure) -> Vec<u8> {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        String::from_utf8_lossy(CLOSURE_DOMAIN),
        closure.operation_ref,
        closure.branch_id,
        closure.candidate_head,
        closure.expected_generation,
        closure.complete,
        closure.intent_refs.join(",")
    )
    .into_bytes()
}

fn verify(trusted_signer: &str, signer: &str, signature: &str, message: &[u8]) -> Result<()> {
    if signer != trusted_signer {
        return Err(Failure::invalid_harness("promotion signer does not match the trusted operator key"));
    }
    let key_bytes = decode_hex(signer, 32)?;
    let mut key_array = [0_u8; 32];
    key_array.copy_from_slice(&key_bytes);
    let key = VerifyingKey::from_bytes(&key_array)
        .map_err(|error| Failure::invalid_harness(format!("promotion signer key is invalid: {error}")))?;
    let signature_bytes = decode_hex(signature, 64)?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|error| Failure::invalid_harness(format!("promotion signature is invalid: {error}")))?;
    key.verify(message, &signature)
        .map_err(|error| Failure::invalid_harness(format!("promotion signature does not verify: {error}")))
}

fn decode_hex(value: &str, expected_bytes: usize) -> Result<Vec<u8>> {
    if value.len() != expected_bytes * 2 {
        return Err(Failure::invalid_harness("promotion hex input has the wrong length"));
    }
    let mut bytes = Vec::with_capacity(expected_bytes);
    let mut chars = value.bytes();
    while let (Some(high), Some(low)) = (chars.next(), chars.next()) {
        bytes.push((hex_nibble(high)? << 4) | hex_nibble(low)?);
    }
    Ok(bytes)
}

fn hex_nibble(value: u8) -> Result<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(Failure::invalid_harness("promotion hex input contains a non-hex digit")),
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let bytes = std::fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(|error| Failure::invalid_harness(format!("{}: {error}", path.display())))
}
