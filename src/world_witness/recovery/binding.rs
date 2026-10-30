pub struct Transaction {
    pub operation: transactional_reconciliation_core::Operation,
    pub reservation: transactional_reconciliation_core::AttemptReservation,
    pub publication: transactional_reconciliation_core::PublicationIntent,
    pub stage_ref: String,
}

struct Deriver;

impl transactional_reconciliation_core::Blake3IdentityDeriver for Deriver {
    fn derive_identity(
        &self,
        bytes: &[u8],
    ) -> Result<transactional_reconciliation_core::Identity, transactional_reconciliation_core::CoreError> {
        transactional_reconciliation_core::Identity::new(*blake3::hash(bytes).as_bytes())
    }
}

/// The immutable plan binds the actual canonical stage, never a replacement operation label.
pub fn prepare(stage: &crate::world_witness::lifecycle::durable::Stage) -> crate::error::Result<Transaction> {
    stage.validate()?;
    if crate::preserves_rail::content_ref_from_bytes(&stage.bytes) != stage.operation_ref
        || stage.staged.operation_id != stage.operation_ref
    {
        return Err(crate::world_witness::store::failure("stage content identity mismatch"));
    }
    let transition = &stage.staged.transition;
    let revision = transactional_reconciliation_core::Revision::observed(
        transition.before.as_ref().map_or(0, |head| head.generation),
    );
    let desired = identity(transition.after.head.as_str())?;
    let policy = identity(transition.after.policy_ref.as_str())?;
    let scope = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(
        "molten-world-witness-effect-scope-v1",
        vec![
            crate::preserves_rail::string(transition.after.branch_id.as_str()),
            crate::preserves_rail::string(transition.after.policy_ref.as_str()),
        ],
    ))?;
    let prerequisites = vec![transactional_reconciliation_core::Prerequisite::new(
        identity(transition.claim_ref.as_str())?,
        identity(&stage.operation_ref)?,
    )];
    let plan = transactional_reconciliation_core::build_plan(
        &Deriver,
        transactional_reconciliation_core::Limits::new(
            transactional_reconciliation_core::Bound::new(1).map_err(core_error)?,
            transactional_reconciliation_core::Bound::new(1).map_err(core_error)?,
        ),
        transactional_reconciliation_core::PlanningInput::new(revision, desired, policy, prerequisites.clone(), vec![
            transactional_reconciliation_core::OperationDraft::new(
                identity(transition.claim_ref.as_str())?,
                identity(&stage.operation_ref)?,
                transactional_reconciliation_core::Generation::observed(transition.after.generation),
                identity(&crate::preserves_rail::content_ref_from_bytes(&scope))?,
            ),
        ]),
    )
    .map_err(core_error)?;
    let operation = plan.operations()[0];
    let attempt_bytes = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(
        "molten-world-witness-first-attempt-v1",
        vec![
            crate::preserves_rail::string(&stage.operation_ref),
            crate::preserves_rail::string(reference(plan.identity())),
            crate::preserves_rail::string(reference(operation.idempotency_identity())),
        ],
    ))?;
    let reservation = transactional_reconciliation_core::reserve_attempt(
        &plan,
        operation.idempotency_identity(),
        identity(&crate::preserves_rail::content_ref_from_bytes(&attempt_bytes))?,
    )
    .map_err(core_error)?;
    let publication = transactional_reconciliation_core::plan_publication(
        &plan,
        &transactional_reconciliation_core::CurrentFacts::new(revision, desired, policy, prerequisites),
        desired,
    )
    .map_err(core_error)?;
    Ok(Transaction {
        operation,
        reservation,
        publication,
        stage_ref: stage.operation_ref.clone(),
    })
}

pub fn canonical(binding: &Transaction) -> crate::error::Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(
        "molten-world-witness-transaction-binding-v1",
        vec![
            crate::preserves_rail::string(&binding.stage_ref),
            crate::preserves_rail::string(reference(binding.reservation.plan_identity())),
            crate::preserves_rail::string(reference(binding.operation.idempotency_identity())),
            crate::preserves_rail::string(reference(binding.operation.input())),
            crate::preserves_rail::string(reference(binding.operation.subject())),
            crate::preserves_rail::string(reference(binding.operation.effect_scope())),
            crate::preserves_rail::string(reference(binding.reservation.attempt_identity())),
            crate::preserves_rail::string(reference(binding.publication.successor_content_identity())),
        ],
    ))
}

pub(super) fn identity(value: &str) -> crate::error::Result<transactional_reconciliation_core::Identity> {
    let hex = value
        .strip_prefix("blake3:")
        .filter(|hex| hex.len() == 64)
        .ok_or_else(|| crate::world_witness::store::failure("expected canonical BLAKE3 identity"))?;
    let mut bytes = [0; 32];
    for (index, pair) in hex.as_bytes().chunks_exact(2).enumerate() {
        let upper = char::from(pair[0])
            .to_digit(16)
            .ok_or_else(|| crate::world_witness::store::failure("invalid identity hex"))?;
        let lower = char::from(pair[1])
            .to_digit(16)
            .ok_or_else(|| crate::world_witness::store::failure("invalid identity hex"))?;
        bytes[index] = u8::try_from(upper * 16 + lower).map_err(crate::world_witness::store::failure)?;
    }
    transactional_reconciliation_core::Identity::new(bytes).map_err(core_error)
}

fn reference(value: transactional_reconciliation_core::Identity) -> String {
    format!("blake3:{}", blake3::Hash::from(*value.as_bytes()).to_hex())
}

fn core_error(value: transactional_reconciliation_core::CoreError) -> crate::error::Failure {
    crate::world_witness::store::failure(format!("{value:?}"))
}
