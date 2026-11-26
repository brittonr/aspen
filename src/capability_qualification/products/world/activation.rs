//! Branch activation through the production branch-authority shell. The
//! runtime observes the current Basalt policy document, capability admission
//! of an Ed25519-signed branch token against the current revocation ledger,
//! durable activation records (replay) and scope at every call, so activation
//! rechecks the facts that hold when it runs, not those the plan saw.

use std::collections::BTreeMap;

use basalt::world_branch_authority::DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON;
use basalt::world_branch_authority::parse_branch_authority_policy;
use ed25519_dalek::Signer;
use ed25519_dalek::SigningKey;
use ed25519_dalek::Verifier;
use molten_core::world_branch_authority::CapabilityKind;
use molten_core::world_branch_authority::CurrentAuthorityFacts;
use molten_core::world_branch_authority::NormalizedCapabilityScope;
use molten_core::world_branch_authority::WorldBranchAction;
use molten_core::world_branch_authority::WorldBranchActivationDecision;
use molten_core::world_branch_authority::WorldBranchAuthorityDiagnostic;
use molten_core::world_branch_authority::WorldBranchAuthorityFacts;
use molten_core::world_branch_authority::WorldBranchAuthorityPlan;
use molten_core::world_branch_authority::WorldBranchObligation;
use molten_core::world_branch_authority::WorldBranchPromotionReservationAdmission;
use molten_core::world_branch_authority::WorldBranchRealizationObservation;
use preserves::IOValue;

use super::runtime::NodeRoot;
use super::runtime::fixture_key;
use super::runtime::key_ref;
use super::runtime::reference;
use crate::capability_tokens::CapabilityProofset;
use crate::capability_tokens::CapabilityRequest;
use crate::capability_tokens::CapabilityToken;
use crate::capability_tokens::admit_capability;
use crate::capability_tokens::capability_token_value;
use crate::error::Result;
use crate::node_state::DirectoryRoot;
use crate::node_state::RelativePath;
use crate::world_branch_authority::ActivationOutcome;
use crate::world_branch_authority::BranchActivationPort;
use crate::world_branch_authority::BranchAuthorityReceiptPort;
use crate::world_branch_authority::CurrentBranchAuthorityPort;
use crate::world_branch_authority::CurrentBranchPolicyPort;
use crate::world_branch_authority::CurrentPolicyObservation;
use crate::world_branch_authority::DestinationGrantPort;
use crate::world_branch_authority::LinearAuthorityTransferPort;
use crate::world_branch_authority::LinearOwnershipObservation;
use crate::world_branch_authority::LinearTransferOutcome;
use crate::world_branch_authority::PromotionReservationPort;
use crate::world_branch_authority::SimulationAuthorityPort;
use crate::world_branch_authority::execute_world_branch_authority;

const POLICY_PATH: &str = "storage/world-branch/policy.json";
const REVOCATION_PATH: &str = "storage/world-branch/revocations.preserves";
const GRANT_DIRECTORY: &str = "storage/world-branch/grants";
const ACTIVATION_DIRECTORY: &str = "storage/world-branch/activations";
const RECEIPT_DIRECTORY: &str = "storage/world-branch/receipts";
const REVOCATION_RECORD: &str = "aspen-qualification-branch-revocations-v1";
const TOKEN_KIND: &str = "world-branch-authority";
const BRANCH_ABILITY: &str = "world/branch/create";
const SCOPE_RESOURCE: &str = "world/branch/qualification";
const SCOPE_LIMIT: u64 = 64;
const TOKEN_EXPIRY_TICK: u64 = 64;
/// The logical tick at which the revocation lands: after planning (tick 1)
/// and before the activation recheck (tick 2).
const REVOCATION_TICK: u64 = 2;
const MAX_STATE_BYTES: u64 = 65_536;

/// The branch-authority runtime of one qualification node.
struct BranchRuntime {
    root: DirectoryRoot,
    issuer: SigningKey,
    token: CapabilityToken,
    signature: ed25519_dalek::Signature,
    tick: u64,
    observations: Vec<CurrentAuthorityFacts>,
    admissions: Vec<IOValue>,
    realized: BTreeMap<String, String>,
    activation_calls: usize,
    receipts: Vec<IOValue>,
}

impl BranchRuntime {
    fn open(path: &std::path::Path, revocation_tick: Option<u64>) -> Result<Self> {
        let node = NodeRoot::open(path, "branch-authority")?;
        let root = node.root;
        for directory in [GRANT_DIRECTORY, ACTIVATION_DIRECTORY, RECEIPT_DIRECTORY] {
            root.create_dir_all(&RelativePath::parse(directory)?)?;
        }
        root.write(&RelativePath::parse(POLICY_PATH)?, DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON.as_bytes())?;
        let issuer = fixture_key("world branch capability authority");
        let token = CapabilityToken {
            token_kind: TOKEN_KIND.to_string(),
            issuer_ref: key_ref(&issuer.verifying_key()),
            holder_ref: reference("aspen qualification branch operator"),
            session_ref: reference("aspen qualification branch session"),
            context_ref: source_branch_ref(),
            resource_ref: capability_ref(),
            ability: BRANCH_ABILITY.to_string(),
            scope: SCOPE_RESOURCE.to_string(),
            attenuation: "exact".to_string(),
            caveats: Vec::new(),
            expires_at_tick: TOKEN_EXPIRY_TICK,
            revoked_refs: Vec::new(),
            policy_refs: Vec::new(),
            resource_refs: Vec::new(),
            delegation_refs: Vec::new(),
            evidence_refs: Vec::new(),
        };
        let signature = issuer.sign(&crate::preserves_rail::canonical_bytes(&capability_token_value(&token)?)?);
        let revocations = revocation_tick
            .map(|tick| {
                crate::preserves_rail::record("revocation", vec![
                    crate::preserves_rail::string(&token.issuer_ref),
                    crate::preserves_rail::u64_value(tick),
                ])
            })
            .into_iter()
            .collect();
        let ledger = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(REVOCATION_RECORD, vec![
            crate::preserves_rail::sequence(revocations),
        ]))?;
        root.write(&RelativePath::parse(REVOCATION_PATH)?, &ledger)?;
        Ok(Self {
            root,
            issuer,
            token,
            signature,
            tick: 0,
            observations: Vec::new(),
            admissions: Vec::new(),
            realized: BTreeMap::new(),
            activation_calls: 0,
            receipts: Vec::new(),
        })
    }

    /// Revocations in force at `tick`, read from the current ledger.
    fn revoked_at(&self, tick: u64) -> Result<Vec<String>> {
        use preserves::ValueImpl as _;
        let bytes = self.root.read(&RelativePath::parse(REVOCATION_PATH)?, MAX_STATE_BYTES)?;
        let value = crate::preserves_rail::parse_canonical_bytes(&bytes)?;
        let fields = crate::preserves_rail::simple_record_fields(&value, REVOCATION_RECORD, 1)?;
        let entries =
            fields[0].collect_sequence().ok_or_else(|| super::invalid("revocation ledger must be a sequence"))?;
        let mut revoked = Vec::new();
        for entry in entries.as_slice() {
            let entry = crate::preserves_rail::value_to_iovalue(entry);
            let pair = crate::preserves_rail::simple_record_fields(&entry, "revocation", 2)?;
            let effective = pair[1]
                .as_u64()
                .ok_or_else(|| super::invalid("revocation tick missing"))?
                .map_err(|error| super::invalid(format!("revocation tick: {error}")))?;
            if effective <= tick {
                revoked.push(crate::preserves_rail::required_string_field(&pair[0], "revoked ref")?);
            }
        }
        Ok(revoked)
    }

    fn signature_verified(&self, token: &CapabilityToken, signature: &ed25519_dalek::Signature) -> Result<bool> {
        let bytes = crate::preserves_rail::canonical_bytes(&capability_token_value(token)?)?;
        let issuer = self.issuer.verifying_key();
        Ok(token.issuer_ref == key_ref(&issuer) && issuer.verify(&bytes, signature).is_ok())
    }

    fn activation_path(destination_branch_ref: &str) -> Result<RelativePath> {
        RelativePath::parse(&format!(
            "{ACTIVATION_DIRECTORY}/{}.preserves",
            crate::preserves_rail::content_ref_hex(destination_branch_ref)?
        ))
    }

    /// The durable activation record of a destination branch, if any.
    fn activation_record(&self, destination_branch_ref: &str) -> Result<Option<String>> {
        let path = Self::activation_path(destination_branch_ref)?;
        if !self.root.try_exists(&path)? {
            return Ok(None);
        }
        let value = crate::preserves_rail::parse_canonical_bytes(&self.root.read(&path, MAX_STATE_BYTES)?)?;
        let fields = crate::preserves_rail::simple_record_fields(&value, "aspen-branch-activation-v1", 3)?;
        Ok(Some(crate::preserves_rail::required_string_field(&fields[0], "activation decision")?))
    }

    fn publish_exact(&self, path: &RelativePath, bytes: &[u8]) -> Result<()> {
        if self.root.try_exists(path)? {
            if self.root.read(path, MAX_STATE_BYTES)? != bytes {
                return Err(super::invalid("branch-authority publication conflicts with retained bytes"));
            }
            return Ok(());
        }
        self.root.write_new(path, bytes)
    }
}

impl CurrentBranchPolicyPort for BranchRuntime {
    fn observe_policy(&mut self) -> Result<CurrentPolicyObservation> {
        let policy_json = self.root.read_to_string(&RelativePath::parse(POLICY_PATH)?, MAX_STATE_BYTES)?;
        let policy = parse_branch_authority_policy(&policy_json)
            .map_err(|_| super::invalid("current branch-authority policy document is malformed"))?;
        Ok(CurrentPolicyObservation {
            policy_ref: policy.identity().to_string(),
            generation: policy.generation(),
            current: true,
            policy_json,
        })
    }
}

impl CurrentBranchAuthorityPort for BranchRuntime {
    fn observe_authority(&mut self, facts: &WorldBranchAuthorityFacts) -> Result<CurrentAuthorityFacts> {
        self.tick = self.tick.checked_add(1).ok_or_else(|| super::invalid("authority tick overflowed"))?;
        let at_tick = self.tick;
        let revoked = self.revoked_at(at_tick)?;
        let policy = self.observe_policy()?;
        let token = &self.token;
        let proofset = CapabilityProofset {
            holder_ref: token.holder_ref.clone(),
            session_ref: token.session_ref.clone(),
            context_ref: token.context_ref.clone(),
            tokens: vec![token.clone()],
            policy_refs: Vec::new(),
            resource_refs: Vec::new(),
            revocation_refs: revoked.clone(),
            evidence_refs: Vec::new(),
        };
        let request = CapabilityRequest {
            holder_ref: token.holder_ref.clone(),
            session_ref: token.session_ref.clone(),
            context_ref: facts.source_branch_ref.clone(),
            resource_ref: facts.capability_ref.clone(),
            ability: BRANCH_ABILITY.to_string(),
            scope: facts.destination_scope.resource().to_string(),
            at_tick,
            required_policy_refs: Vec::new(),
            required_resource_refs: Vec::new(),
            required_token_kind: Some(TOKEN_KIND.to_string()),
            caveat_context: Vec::new(),
        };
        let admission = admit_capability(&proofset, &request)?;
        let current = CurrentAuthorityFacts {
            observation_ref: crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
                "aspen-qualification-branch-current-authority-v1",
                vec![
                    crate::preserves_rail::string(&admission.receipt_ref),
                    crate::preserves_rail::string(&policy.policy_ref),
                    crate::preserves_rail::u64_value(at_tick),
                ],
            ))?,
            policy_current: policy.current && policy.generation == facts.policy_generation,
            capability_current: admission.decision == "pass",
            revocation_current: !revoked
                .iter()
                .any(|reference| *reference == token.issuer_ref || token.delegation_refs.contains(reference)),
            replay_current: self.activation_record(&facts.destination_branch_ref)?.is_none(),
            scope_current: facts.destination_scope.is_within(&facts.source_scope)
                && token.scope == facts.destination_scope.resource(),
            ucan_verified: self.signature_verified(&self.token, &self.signature)?,
        };
        self.admissions.push(admission.value);
        self.observations.push(current.clone());
        Ok(current)
    }
}

impl DestinationGrantPort for BranchRuntime {
    /// Mint and durably record an independent signed destination grant.
    fn realize_destination_grant(
        &mut self,
        plan: &WorldBranchAuthorityPlan,
    ) -> Result<WorldBranchRealizationObservation> {
        let grant = CapabilityToken {
            token_kind: "world-branch-destination-grant".to_string(),
            context_ref: plan.destination_branch_ref.clone(),
            attenuation: "independent".to_string(),
            scope: plan.destination_scope.resource().to_string(),
            ..self.token.clone()
        };
        let grant_value = capability_token_value(&grant)?;
        let grant_ref = crate::preserves_rail::canonical_hash(&grant_value)?;
        let signature = self.issuer.sign(&crate::preserves_rail::canonical_bytes(&grant_value)?);
        let record = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(
            "aspen-branch-destination-grant-v1",
            vec![
                grant_value,
                crate::preserves_rail::string(hex::encode(signature.to_bytes())),
            ],
        ))?;
        let path = RelativePath::parse(&format!(
            "{GRANT_DIRECTORY}/{}.preserves",
            crate::preserves_rail::content_ref_hex(&grant_ref)?
        ))?;
        self.publish_exact(&path, &record)?;
        let is_durable = self.root.read(&path, MAX_STATE_BYTES)? == record;
        let destination_grant_current = is_durable && self.signature_verified(&grant, &signature)?;
        let evidence_refs = plan
            .obligations
            .iter()
            .filter(|obligation| **obligation != WorldBranchObligation::DenyActivation)
            .map(|obligation| {
                crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
                    "aspen-branch-obligation-evidence-v1",
                    vec![
                        crate::preserves_rail::string(obligation.as_str()),
                        crate::preserves_rail::string(&grant_ref),
                    ],
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        self.realized.insert(plan.plan_ref.clone(), plan.destination_branch_ref.clone());
        Ok(WorldBranchRealizationObservation {
            plan_ref: plan.plan_ref.clone(),
            policy_ref: plan.policy_ref.clone(),
            capability_ref: plan.capability_ref.clone(),
            operation_ref: grant_ref,
            evidence_refs,
            destination_scope: plan.destination_scope.clone(),
            destination_grant_current,
            source_active: true,
            destination_active: false,
            transfer_generation: None,
            simulation_adapter_ref: None,
            simulation_adapter_deterministic: false,
            release_reservation_ref: None,
            promotion_admission: None,
            bearer_material_present: false,
            receipt_claims_authority: false,
        })
    }
}

/// The qualification node holds no exclusive lease, simulation adapter or
/// release reservation; those realizations fail closed.
impl LinearAuthorityTransferPort for BranchRuntime {
    fn observe_ownership(&mut self, _plan: &WorldBranchAuthorityPlan) -> Result<LinearOwnershipObservation> {
        Err(super::invalid("qualification branch node holds no linear ownership"))
    }

    fn transfer(
        &mut self,
        _plan: &WorldBranchAuthorityPlan,
        _expected_generation: u64,
        _operation_ref: &str,
    ) -> Result<LinearTransferOutcome> {
        Ok(LinearTransferOutcome::Denied)
    }

    fn reconcile_transfer(
        &mut self,
        _plan: &WorldBranchAuthorityPlan,
        _operation_ref: &str,
    ) -> Result<Option<WorldBranchRealizationObservation>> {
        Ok(None)
    }
}

impl SimulationAuthorityPort for BranchRuntime {
    fn bind_simulation(&mut self, _plan: &WorldBranchAuthorityPlan) -> Result<WorldBranchRealizationObservation> {
        Err(super::invalid("qualification branch node has no deterministic simulation adapter"))
    }
}

impl PromotionReservationPort for BranchRuntime {
    fn admit_promotion_reservation(
        &mut self,
        _plan: &WorldBranchAuthorityPlan,
    ) -> Result<WorldBranchPromotionReservationAdmission> {
        Err(super::invalid("qualification branch node holds no committed release reservation"))
    }
}

impl BranchActivationPort for BranchRuntime {
    fn activate(&mut self, decision: &WorldBranchActivationDecision) -> Result<ActivationOutcome> {
        self.activation_calls =
            self.activation_calls.checked_add(1).ok_or_else(|| super::invalid("activation count overflowed"))?;
        let destination = self
            .realized
            .get(&decision.plan_ref)
            .cloned()
            .ok_or_else(|| super::invalid("activation decision names an unrealized plan"))?;
        let path = Self::activation_path(&destination)?;
        if self.root.try_exists(&path)? {
            return Ok(ActivationOutcome::Denied);
        }
        let record = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(
            "aspen-branch-activation-v1",
            vec![
                crate::preserves_rail::string(&decision.decision_ref),
                crate::preserves_rail::string(&decision.plan_ref),
                crate::preserves_rail::string(&destination),
            ],
        ))?;
        self.root.write_new(&path, &record)?;
        Ok(ActivationOutcome::Activated)
    }

    fn reconcile_activation(&mut self, decision: &WorldBranchActivationDecision) -> Result<ActivationOutcome> {
        let destination = self
            .realized
            .get(&decision.plan_ref)
            .cloned()
            .ok_or_else(|| super::invalid("activation decision names an unrealized plan"))?;
        Ok(if self.activation_record(&destination)?.as_deref() == Some(decision.decision_ref.as_str()) {
            ActivationOutcome::Activated
        } else {
            ActivationOutcome::Denied
        })
    }
}

impl BranchAuthorityReceiptPort for BranchRuntime {
    fn publish_receipt(&mut self, receipt_ref: &str, canonical_bytes: &[u8]) -> Result<()> {
        let value = crate::preserves_rail::parse_canonical_bytes(canonical_bytes)?;
        let path = RelativePath::parse(&format!(
            "{RECEIPT_DIRECTORY}/{}.preserves",
            crate::preserves_rail::content_ref_hex(receipt_ref)?
        ))?;
        self.publish_exact(&path, canonical_bytes)?;
        self.receipts.push(value);
        Ok(())
    }
}

fn source_branch_ref() -> String {
    reference("aspen qualification source world branch")
}

fn capability_ref() -> String {
    reference("aspen qualification world branch capability")
}

fn facts(runtime: &mut BranchRuntime) -> Result<WorldBranchAuthorityFacts> {
    let scope =
        NormalizedCapabilityScope::new(SCOPE_RESOURCE, ["read".to_string(), "write".to_string()], Some(SCOPE_LIMIT))
            .map_err(|diagnostic| super::invalid(format!("branch scope: {diagnostic:?}")))?;
    Ok(WorldBranchAuthorityFacts {
        capability_kind: CapabilityKind::PublicArtifact,
        action: WorldBranchAction::Create,
        source_branch_ref: source_branch_ref(),
        destination_branch_ref: reference("aspen qualification destination world branch"),
        capability_ref: capability_ref(),
        source_scope: scope.clone(),
        destination_scope: scope,
        policy_generation: runtime.observe_policy()?.generation,
        mapping_lossless: true,
    })
}

// r[impl molten.world_branch_authority.activation]
pub(super) fn current_facts() -> Result<super::super::Evidence> {
    let scratch = super::super::scratch("world-activation")?;
    let mut evidence = super::super::Evidence::default();
    let mut runtime = BranchRuntime::open(scratch.path(), None)?;
    let facts = facts(&mut runtime)?;
    let execution = execute_world_branch_authority(&facts, &mut runtime)?;
    let activation = execution.activation.as_ref();
    evidence.check("plan-admitted-by-current-policy", execution.plan.allowed);
    evidence.check(
        "authority-rechecked-at-activation",
        runtime.observations.len() == 2
            && runtime.observations.iter().all(CurrentAuthorityFacts::all_current)
            && runtime.observations[0].observation_ref != runtime.observations[1].observation_ref,
    );
    evidence.check(
        "activation-admitted-on-current-facts",
        activation.is_some_and(|decision| {
            decision.allowed && decision.diagnostic == WorldBranchAuthorityDiagnostic::Admitted
        }),
    );
    evidence.check(
        "activation-performed-once",
        execution.activation_outcome == Some(ActivationOutcome::Activated) && runtime.activation_calls == 1,
    );
    evidence.check(
        "activation-durably-recorded",
        activation.is_some_and(|decision| {
            runtime
                .activation_record(&facts.destination_branch_ref)
                .is_ok_and(|record| record.as_deref() == Some(decision.decision_ref.as_str()))
        }),
    );
    evidence.check(
        "plan-activation-outcome-receipts-published",
        execution.receipt_refs.len() == 3 && runtime.receipts.len() == 3,
    );
    for receipt in runtime.receipts.clone() {
        evidence.runtime(receipt);
    }
    for admission in runtime.admissions.clone() {
        evidence.runtime(admission);
    }

    let replayed = execute_world_branch_authority(&facts, &mut runtime)?;
    evidence.check(
        "replayed-activation-not-admitted",
        !replayed.activation.as_ref().is_some_and(|decision| decision.allowed)
            && replayed.activation_outcome.is_none()
            && runtime.activation_calls == 1
            && runtime.observations.iter().skip(2).any(|current| !current.replay_current),
    );
    Ok(evidence)
}

/// A revocation that lands after planning denies activation on the recheck;
/// `revoke = false` is the control that must activate instead.
// r[impl molten.world_branch_authority.activation]
pub(super) fn revoked_denied(revoke: bool) -> Result<super::super::Evidence> {
    let scratch = super::super::scratch("world-activation-revoked")?;
    let mut evidence = super::super::Evidence::default();
    let mut runtime = BranchRuntime::open(scratch.path(), revoke.then_some(REVOCATION_TICK))?;
    let facts = facts(&mut runtime)?;
    let execution = execute_world_branch_authority(&facts, &mut runtime)?;
    evidence.check("plan-admitted-on-earlier-facts", execution.plan.allowed);
    evidence.check(
        "revocation-observed-at-activation",
        runtime.observations.len() == 2
            && runtime.observations[0].all_current()
            && !runtime.observations[1].revocation_current
            && !runtime.observations[1].capability_current,
    );
    evidence.check(
        "activation-denied-on-current-facts",
        execution.activation.as_ref().is_some_and(|decision| {
            !decision.allowed && decision.diagnostic == WorldBranchAuthorityDiagnostic::CurrentnessDenied
        }),
    );
    evidence.check("no-activation-attempted", runtime.activation_calls == 0 && execution.activation_outcome.is_none());
    evidence.check("no-activation-recorded", runtime.activation_record(&facts.destination_branch_ref)?.is_none());
    evidence.check("denial-receipt-published", execution.receipt_refs.len() == 2 && runtime.receipts.len() == 2);
    for receipt in runtime.receipts {
        evidence.runtime(receipt);
    }
    for admission in runtime.admissions {
        evidence.runtime(admission);
    }
    Ok(evidence)
}
