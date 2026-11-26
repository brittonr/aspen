//! Signed local authority and intent-closure facts for one promotion transaction.
//!
//! The files are explicit operator inputs. They are verified before use and do
//! not turn a caller-provided boolean into authority. Signatures cover canonical
//! packed Preserves records, never delimiter-joined text.

mod grant;

use std::collections::BTreeSet;
use std::path::Path;

use ed25519_dalek::Signature;
use ed25519_dalek::Verifier;
use ed25519_dalek::VerifyingKey;
use molten_core::world_promotion::WorldPromotionPlan;
use molten_core::world_promotion::WorldPromotionTransactionFacts;
use molten_core::world_promotion::*;
use molten_node_host::node_state::DirectoryRoot;
use molten_node_host::node_state::NamespaceKind;
use serde::Deserialize;
use serde::Serialize;

use super::LocalIntentMaterial;
use super::WORLD_PROMOTION_GRANT_SCHEMA;
use super::WORLD_PROMOTION_INTENT_CLOSURE_SCHEMA;
use super::WorldPromotionCurrentPort;
use super::canonical_intent_closure;
use super::canonical_promotion_grant;
use crate::error::Failure;
use crate::error::Result;

/// The JSON carrier of one signed grant. The signature covers the canonical record of the typed
/// grant it parses to (`canonical_promotion_grant`); exactly the target fields of `action` must
/// be present.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignedPromotionAuthority {
    pub schema: String,
    pub action: String,
    pub branch_id: String,
    pub authority_ref: String,
    pub policy_ref: String,
    pub generation: u64,
    pub admitted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_head: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reservation_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_ref: Option<String>,
    pub nonce: String,
    pub expires_at: u64,
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
    pub grant: WorldPromotionGrant,
    pub observed_unix_seconds: u64,
    pub closure: SignedIntentClosure,
    material: LocalIntentMaterial,
}

impl ObservedPromotionCurrent {
    /// Reads the content-addressed material for every plan intent now, at
    /// application time, and requires it to match the plan exactly.
    fn is_intent_material_available(&self, plan: &WorldPromotionPlan) -> Result<bool> {
        let mut material = Vec::with_capacity(plan.intents.len());
        for intent in &plan.intents {
            match self.material.read(&intent.intent_ref)? {
                Some(stored) => material.push(stored),
                None => return Ok(false),
            }
        }
        Ok(intent_material_matches(plan, &material))
    }
}

impl WorldPromotionCurrentPort for ObservedPromotionCurrent {
    // r[impl molten.world_promotion.transaction]
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
            grant: self.grant.clone(),
            observed_unix_seconds: self.observed_unix_seconds,
            intent_closure_complete: is_closure_complete,
            intent_material_available: self.is_intent_material_available(plan)?,
            reservation_refs: plan.reservations.iter().map(|reservation| reservation.reservation_ref.clone()).collect(),
        })
    }
}

/// The signed inputs of one promotion and the wall-clock time they are decided at.
pub struct ObservedCurrentInput<'a> {
    pub state_root: &'a Path,
    pub authority_path: &'a Path,
    pub closure_path: &'a Path,
    pub trusted_signer: &'a str,
    pub branch_id: &'a str,
    pub observed_unix_seconds: u64,
}

/// Verifies the signed promotion grant and intent closure for `branch_id`. The live branch head
/// and grant consumption are read later, inside the store's write transaction.
pub fn read_observed_current(input: ObservedCurrentInput<'_>) -> Result<ObservedPromotionCurrent> {
    let grant = read_promotion_grant(input.authority_path, input.trusted_signer)?;
    let closure = read_closure(input.closure_path, input.trusted_signer)?;
    if grant.branch_id.as_str() != input.branch_id || closure.branch_id != input.branch_id {
        return Err(Failure::invalid_harness("signed promotion input names the wrong branch"));
    }
    let root = DirectoryRoot::open(input.state_root)?;
    let storage = root.namespace(NamespaceKind::Storage)?;
    Ok(ObservedPromotionCurrent {
        grant,
        observed_unix_seconds: input.observed_unix_seconds,
        closure,
        material: LocalIntentMaterial::from_storage(storage),
    })
}

/// Reads and verifies one signed promotion or operator grant. Verification establishes who signed
/// which exact action, target, nonce and expiry; it does not establish that the grant is current:
/// the store decides that against the live head and consumed nonces inside its write transaction.
// r[impl molten.world_commit.promotion_authority_action_and_canonical_binding]
pub fn read_promotion_grant(path: &Path, trusted_signer: &str) -> Result<WorldPromotionGrant> {
    let authority: SignedPromotionAuthority = read_json(path)?;
    verify_promotion_grant(&authority, trusted_signer)
}

/// Verifies one parsed grant document against the trusted operator key.
pub fn verify_promotion_grant(
    authority: &SignedPromotionAuthority,
    trusted_signer: &str,
) -> Result<WorldPromotionGrant> {
    if authority.schema != WORLD_PROMOTION_GRANT_SCHEMA {
        return Err(Failure::invalid_harness("promotion authority schema is unsupported"));
    }
    let grant = authority.grant()?;
    let canonical = canonical_promotion_grant(&grant)?;
    verify(trusted_signer, &authority.signer, &authority.signature, &canonical.bytes)?;
    Ok(grant)
}

pub(super) fn read_closure(path: &Path, trusted_signer: &str) -> Result<SignedIntentClosure> {
    let closure: SignedIntentClosure = read_json(path)?;
    verify_closure(&closure, trusted_signer)?;
    Ok(closure)
}

pub(super) fn verify_closure(closure: &SignedIntentClosure, trusted_signer: &str) -> Result<()> {
    if closure.schema != WORLD_PROMOTION_INTENT_CLOSURE_SCHEMA {
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
    let canonical = canonical_intent_closure(closure)?;
    verify(trusted_signer, &closure.signer, &closure.signature, &canonical.bytes)
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
