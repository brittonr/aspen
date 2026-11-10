//! Real status, work submission, delivery and exact effect authority.

/// Member facts the exact route credential must bind.
#[derive(Debug, Clone, Copy)]
pub struct Member {
    pub generation: u64,
    pub incarnation: u64,
    pub tick: u64,
}

pub struct Work {
    pub presentation: molten::system_extension::WorkPresentation,
    pub payload: molten::system_extension::NativeCallbackValue,
    pub actor: usize,
    pub submit_tick: u64,
}

pub struct Intent {
    pub prepared: crate::authority::intent::Prepared,
    pub observation: molten::collaboration_scope::native_runtime::Observation,
}

pub fn drive(cohort: &crate::producer::Cohort) -> crate::producer::Result<preserves::IOValue> {
    let mut session = super::scenarios::Session::open(cohort, intent)?;
    let (is_delivered, output) = super::flow::deliver(&session.delivery(cohort))?;
    session.require_live()?;
    if !is_delivered {
        return Err(format!("protected delivery did not complete: {output}").into());
    }
    finish_commit(cohort, &mut session.nodes, &session.intent, &session.work, &output)
}

pub(super) fn seed(index: u64) -> crate::producer::Result<[u8; 32]> {
    let mut seed = [0u8; 32];
    seed[0] = 0x5c;
    seed[31] = u8::try_from(index).map_err(|_| "cohort position exceeds one byte")?;
    Ok(seed)
}

/// One exact scope creation signed by the operator holder.
pub(super) fn intent(
    issuer: &crate::authority::Issuer,
    observation: molten::collaboration_scope::native_runtime::Observation,
) -> crate::producer::Result<Intent> {
    let owner = issuer.subject()?;
    let operation = super::operations::operation("collaboration-native-create")?;
    let scope = molten::collaboration_scope::CollaborationScope {
        reference: super::operations::scope("collaboration-native-scope")?,
        kind: molten::collaboration_scope::ScopeKind::Group,
        owner: owner.clone(),
        revision: 1,
        membership_epoch: 1,
        policy: molten::collaboration_scope::policy_ref(super::setup::SCOPE_POLICY)?,
        organization_floor: None,
        members: [(owner.clone(), molten::collaboration_scope::Membership {
            reference: super::operations::membership("collaboration-native-owner-membership")?,
            subject: owner,
            role: molten::collaboration_scope::MembershipRole::Owner,
            abilities: [
                molten::collaboration_scope::Ability::Read,
                molten::collaboration_scope::Ability::Use,
                molten::collaboration_scope::Ability::Share,
                molten::collaboration_scope::Ability::Bind,
                molten::collaboration_scope::Ability::Move,
                molten::collaboration_scope::Ability::Create,
                molten::collaboration_scope::Ability::Membership,
            ]
            .into_iter()
            .collect(),
            start_epoch: 1,
            expires_at: None,
            decision: super::operations::decision("collaboration-native-owner-decision")?,
            operation: operation.clone(),
        })]
        .into_iter()
        .collect(),
    };
    let request = issuer.mutation_request(
        operation,
        observation.state.clone(),
        observation.currentness.clone(),
        molten::collaboration_scope::Mutation::Create(scope),
    )?;
    Ok(Intent {
        prepared: crate::authority::intent::prepare(request)?,
        observation,
    })
}

pub mod wire {
    pub fn operation(name: &str, fields: Vec<preserves::IOValue>) -> preserves::IOValue {
        preserves::IOValue::record(preserves::IOValue::symbol(format!("node-service-host-{name}-v1")), fields)
    }

    pub fn work(value: &molten::system_extension::WorkPresentation) -> preserves::IOValue {
        molten::preserves_rail::record("node-service-host-work-v1", vec![
            molten::preserves_rail::string(&value.work_ref),
            molten::preserves_rail::string(value.class.as_str()),
            molten::preserves_rail::u64_value(value.generation),
            molten::preserves_rail::u64_value(value.incarnation),
        ])
    }

    pub fn payload(value: &molten::system_extension::NativeCallbackValue) -> preserves::IOValue {
        molten::preserves_rail::record("node-service-host-value-v1", vec![
            molten::preserves_rail::string(&value.value_ref),
            molten::preserves_rail::sequence(
                value.bytes.iter().map(|byte| molten::preserves_rail::u64_value(u64::from(*byte))).collect(),
            ),
        ])
    }

    pub fn text(value: &preserves::Value<preserves::IOValue>) -> crate::producer::Result<String> {
        Ok(molten::preserves_rail::required_string_field(value, "actual node observation")?)
    }

    pub fn number(value: &preserves::Value<preserves::IOValue>) -> crate::producer::Result<u64> {
        Ok(value.as_u64().ok_or("unsigned observation")??)
    }
}

pub fn scope(label: &str) -> crate::producer::Result<molten::collaboration_scope::ScopeRef> {
    Ok(molten::collaboration_scope::ScopeRef::new(crate::producer::reference(label))
        .map_err(|error| format!("{error:?}"))?)
}

pub fn membership(label: &str) -> crate::producer::Result<molten::collaboration_scope::MembershipRef> {
    Ok(molten::collaboration_scope::MembershipRef::new(crate::producer::reference(label))
        .map_err(|error| format!("{error:?}"))?)
}

pub fn decision(label: &str) -> crate::producer::Result<molten::collaboration_scope::DecisionRef> {
    Ok(molten::collaboration_scope::DecisionRef::new(crate::producer::reference(label))
        .map_err(|error| format!("{error:?}"))?)
}

pub fn operation(label: &str) -> crate::producer::Result<molten_core::nominal::OperationRef> {
    Ok(molten_core::nominal::OperationRef::new(label).map_err(|error| format!("{error:?}"))?)
}

fn finish_commit(
    cohort: &crate::producer::Cohort,
    nodes: &mut [super::Node],
    intent: &Intent,
    work: &Work,
    output: &str,
) -> crate::producer::Result<preserves::IOValue> {
    let effect = super::status::routed_completion(output)?;
    let actor = work.actor;
    let completed = super::status::completion(
        &nodes[actor],
        &cohort.nodes[actor],
        &work.presentation.work_ref,
        work.submit_tick + 3,
    )?;
    for node in nodes.iter_mut() {
        super::finish(node)?;
    }
    let observation = super::status::observed_commit(&cohort.nodes[actor], intent, &effect)?;
    let reopened = super::process::reopened_status(&nodes[actor], &cohort.nodes[actor], work.submit_tick + 15)?;
    if super::status::completed(&reopened, &work.presentation.work_ref)?.as_deref() != Some(completed.as_str()) {
        return Err("the exact work completion did not survive owner shutdown and source-admitted reopen".into());
    }
    retain(cohort, nodes, observation, output, &reopened)
}

fn retain(
    cohort: &crate::producer::Cohort,
    nodes: &[super::Node],
    observation: preserves::IOValue,
    delivered: &str,
    reopened: &str,
) -> crate::producer::Result<preserves::IOValue> {
    if nodes.len() != cohort.nodes.len() {
        return Err("serving owner set differs from the admitted cohort".into());
    }
    let mut ready = Vec::with_capacity(nodes.len());
    for (fixture, owner) in cohort.nodes.iter().zip(nodes) {
        let name = owner.log.file_name().and_then(|name| name.to_str()).ok_or("owned serving log name")?;
        let log = fixture.root.read(&molten::node_state::RelativePath::parse(name)?, 16 * 1024 * 1024)?;
        let text = std::str::from_utf8(&log)?;
        ready.push(molten::preserves_rail::parse_text(super::flow::value(text, super::READY)?)?);
    }
    Ok(molten::preserves_rail::record("collaboration-native-consumer-observations-v1", vec![
        molten::preserves_rail::string(cohort.signed_main.to_str().ok_or("source executable encoding")?),
        molten::preserves_rail::sequence(ready),
        observation,
        molten::preserves_rail::parse_text(super::flow::value(delivered, "node-service-host-receipt-v1")?)?,
        molten::preserves_rail::parse_text(super::flow::value(reopened, "node-service-host-receipt-v1")?)?,
        molten::preserves_rail::string(
            "bounded-experimental-observations-not-authority-or-production-or-global-exactly-once-proof",
        ),
    ]))
}
