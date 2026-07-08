
#[derive(Debug, Clone, Copy)]
pub struct StatusAssertionInput<'a> {
    pub service: &'a str,
    pub key: &'a str,
    pub read_consistency_mode: &'a str,
    pub fact: &'a IoValue,
    pub state_ref: &'a str,
    pub receipt_ref: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct ApplyReportValueInput<'a> {
    pub decision: &'a str,
    pub manifest_ref: &'a str,
    pub final_state_ref: &'a str,
    pub receipt_refs: &'a [String],
    pub assertion_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

pub fn coordination_service_manifest_value(input: &CoordinationServiceManifestInput) -> Result<IoValue> {
    validate_service_id(&input.service_id)?;
    validate_services(&input.services)?;
    validate_ref(&input.control_group_ref, "coordination control group ref")?;
    validate_refs(&input.policy_refs, "coordination manifest policy ref")?;
    validate_refs(&input.resource_refs, "coordination manifest resource ref")?;
    validate_capacity(input.queue_capacity, "coordination queue capacity")?;
    validate_capacity(input.semaphore_capacity, "coordination semaphore capacity")?;
    validate_capacity(input.rate_limit, "coordination rate limit")?;
    validate_capacity(input.barrier_parties, "coordination barrier parties")?;
    Ok(record("coordination-service-manifest-v1", vec![
        string(COORDINATION_SERVICE_MANIFEST_SCHEMA),
        record("service-id", vec![string(&input.service_id)]),
        record("services", vec![strings_sequence(&input.services)]),
        record("control-group", vec![string(&input.control_group_ref)]),
        record("queue-capacity", vec![u64_value(input.queue_capacity)]),
        record("semaphore-capacity", vec![u64_value(input.semaphore_capacity)]),
        record("rate-limit", vec![u64_value(input.rate_limit)]),
        record("barrier-parties", vec![u64_value(input.barrier_parties)]),
        record("policy", vec![strings_sequence(&input.policy_refs)]),
        record("resource", vec![strings_sequence(&input.resource_refs)]),
        checks_value(&[
            ("control-plane-service", "pass"),
            ("explicit-primitives", "pass"),
            ("no-actor-traffic", "pass"),
        ]),
    ]))
}

pub fn parse_coordination_service_manifest(value: &IoValue) -> Result<CoordinationServiceManifest> {
    let fields = simple_record(value, "coordination-service-manifest-v1", 11)?;
    require_schema(&fields[0], COORDINATION_SERVICE_MANIFEST_SCHEMA, "coordination manifest schema")?;
    let input = CoordinationServiceManifestInput {
        service_id: record_string(&fields[1], "service-id")?,
        services: record_string_sequence(&fields[2], "services")?,
        control_group_ref: record_ref(&fields[3], "control-group")?,
        queue_capacity: record_u64(&fields[4], "queue-capacity")?,
        semaphore_capacity: record_u64(&fields[5], "semaphore-capacity")?,
        rate_limit: record_u64(&fields[6], "rate-limit")?,
        barrier_parties: record_u64(&fields[7], "barrier-parties")?,
        policy_refs: record_ref_sequence(&fields[8], "policy")?,
        resource_refs: record_ref_sequence(&fields[9], "resource")?,
    };
    validate_service_id(&input.service_id)?;
    validate_services(&input.services)?;
    require_check(&parse_checks(&fields[10])?, "control-plane-service", "coordination manifest")?;
    Ok(CoordinationServiceManifest {
        manifest_ref: canonical_hash(value)?,
        service_id: input.service_id,
        services: input.services,
        control_group_ref: input.control_group_ref,
        queue_capacity: input.queue_capacity,
        semaphore_capacity: input.semaphore_capacity,
        rate_limit: input.rate_limit,
        barrier_parties: input.barrier_parties,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        value: value.clone(),
    })
}

// r[impl molten.coordination.read_consistency_modes]
pub fn coordination_request_value(input: &CoordinationRequestInput) -> Result<IoValue> {
    validate_request_input(input)?;
    Ok(record("coordination-request-v1", vec![
        string(COORDINATION_REQUEST_SCHEMA),
        record("service", vec![string(&input.service)]),
        record("operation", vec![string(&input.operation)]),
        record("key", vec![string(&input.key)]),
        record("client-session", vec![string(&input.client_session)]),
        record("operation-id", vec![string(&input.operation_id_ref)]),
        record("read-consistency", vec![string(&input.read_consistency_mode)]),
        record("payload", vec![optional_value(input.payload.as_ref())]),
        record("authority", vec![strings_sequence(&input.authority_refs)]),
        record("resource", vec![strings_sequence(&input.resource_refs)]),
        record("policy", vec![strings_sequence(&input.policy_refs)]),
        checks_value(&[
            ("control-plane-command", "pass"),
            ("operation-id-bound", "pass"),
            ("read-consistency-declared", "pass"),
            ("deny-by-default-authority", "pass"),
        ]),
    ]))
}

pub fn parse_coordination_request(value: &IoValue) -> Result<CoordinationRequest> {
    let fields = simple_record(value, "coordination-request-v1", 12)?;
    require_schema(&fields[0], COORDINATION_REQUEST_SCHEMA, "coordination request schema")?;
    let input = CoordinationRequestInput {
        service: record_string(&fields[1], "service")?,
        operation: record_string(&fields[2], "operation")?,
        key: record_string(&fields[3], "key")?,
        client_session: record_string(&fields[4], "client-session")?,
        operation_id_ref: record_ref(&fields[5], "operation-id")?,
        read_consistency_mode: record_string(&fields[6], "read-consistency")?,
        payload: record_optional_value(&fields[7], "payload")?,
        authority_refs: record_ref_sequence(&fields[8], "authority")?,
        resource_refs: record_ref_sequence(&fields[9], "resource")?,
        policy_refs: record_ref_sequence(&fields[10], "policy")?,
    };
    validate_request_input(&input)?;
    require_check(&parse_checks(&fields[11])?, "control-plane-command", "coordination request")?;
    Ok(CoordinationRequest {
        request_ref: canonical_hash(value)?,
        service: input.service,
        operation: input.operation,
        key: input.key,
        client_session: input.client_session,
        operation_id_ref: input.operation_id_ref,
        read_consistency_mode: input.read_consistency_mode,
        payload: input.payload,
        authority_refs: input.authority_refs,
        resource_refs: input.resource_refs,
        policy_refs: input.policy_refs,
        value: value.clone(),
    })
}

pub fn fencing_token_value(
    key: &str,
    owner: &str,
    token: u64,
    lease_epoch: u64,
    commit_receipt_ref: &str,
) -> Result<IoValue> {
    validate_key(key)?;
    validate_session(owner)?;
    validate_ref(commit_receipt_ref, "coordination token commit receipt ref")?;
    Ok(record("fencing-token-v1", vec![
        string(COORDINATION_FENCING_TOKEN_SCHEMA),
        record("key", vec![string(key)]),
        record("owner", vec![string(owner)]),
        record("token", vec![u64_value(token)]),
        record("lease-epoch", vec![u64_value(lease_epoch)]),
        record("commit", vec![string(commit_receipt_ref)]),
        checks_value(&[("fencing-token-monotonic", "pass"), ("commit-bound", "pass")]),
    ]))
}

pub fn parse_fencing_token(value: &IoValue) -> Result<FencingToken> {
    let fields = simple_record(value, "fencing-token-v1", 7)?;
    require_schema(&fields[0], COORDINATION_FENCING_TOKEN_SCHEMA, "coordination fencing token schema")?;
    let key = record_string(&fields[1], "key")?;
    let owner = record_string(&fields[2], "owner")?;
    let token = record_u64(&fields[3], "token")?;
    let lease_epoch = record_u64(&fields[4], "lease-epoch")?;
    let commit_receipt_ref = record_ref(&fields[5], "commit")?;
    require_check(&parse_checks(&fields[6])?, "fencing-token-monotonic", "coordination fencing token")?;
    Ok(FencingToken {
        token_ref: canonical_hash(value)?,
        key,
        owner,
        token,
        lease_epoch,
        commit_receipt_ref,
        value: value.clone(),
    })
}

pub fn coordination_state_snapshot_value(state: &CoordinationState) -> Result<IoValue> {
    let records = state_records(state)?;
    let retention_refs = retention_refs_for_state(state)?;
    Ok(record("coordination-state-snapshot-v1", vec![
        string(COORDINATION_STATE_SNAPSHOT_SCHEMA),
        record("locks", vec![sequence(records.locks)]),
        record("queues", vec![sequence(records.queues)]),
        record("semaphores", vec![sequence(records.semaphores)]),
        record("rates", vec![sequence(records.rates)]),
        record("elections", vec![sequence(records.elections)]),
        record("barriers", vec![sequence(records.barriers)]),
        record("registry", vec![sequence(records.registry)]),
        record("next-fencing-token", vec![u64_value(state.next_fencing_token)]),
        record("retention", vec![strings_sequence(&retention_refs)]),
        checks_value(&[
            ("deterministic-state-order", "pass"),
            ("active-state-retention-pins", "pass"),
            ("control-plane-reflection-source", "pass"),
        ]),
    ]))
}

struct StateRecords {
    locks: Vec<IoValue>,
    queues: Vec<IoValue>,
    semaphores: Vec<IoValue>,
    rates: Vec<IoValue>,
    elections: Vec<IoValue>,
    barriers: Vec<IoValue>,
    registry: Vec<IoValue>,
}

fn state_records(state: &CoordinationState) -> Result<StateRecords> {
    ensure_count_at_most(state.locks.len(), MAX_COORDINATION_ITEMS, "coordination locks")?;
    ensure_count_at_most(state.queues.len(), MAX_COORDINATION_ITEMS, "coordination queues")?;
    ensure_count_at_most(state.semaphores.len(), MAX_COORDINATION_ITEMS, "coordination semaphores")?;
    ensure_count_at_most(state.registry.len(), MAX_COORDINATION_ITEMS, "coordination registry")?;
    Ok(StateRecords {
        locks: lock_records(&state.locks),
        queues: queue_records(&state.queues),
        semaphores: semaphore_records(&state.semaphores),
        rates: rate_records(&state.rates),
        elections: election_records(&state.elections),
        barriers: barrier_records(&state.barriers),
        registry: registry_records(&state.registry),
    })
}

fn lock_records(locks: &OrderedMap<String, LockState>) -> Vec<IoValue> {
    locks
        .iter()
        .map(|(key, lock)| {
            record("lock", vec![
                string(key),
                string(&lock.owner),
                u64_value(lock.token),
                string(&lock.token_ref),
                u64_value(lock.lease_epoch),
            ])
        })
        .collect()
}

fn queue_records(queues: &OrderedMap<String, Vec<String>>) -> Vec<IoValue> {
    queues
        .iter()
        .map(|(key, items)| record("queue", vec![string(key), strings_sequence(items)]))
        .collect()
}

fn semaphore_records(semaphores: &OrderedMap<String, OrderedSet<String>>) -> Vec<IoValue> {
    semaphores
        .iter()
        .map(|(key, holders)| {
            let values = holders.iter().cloned().collect::<Vec<_>>();
            record("semaphore", vec![string(key), strings_sequence(&values)])
        })
        .collect()
}

fn rate_records(rates: &OrderedMap<String, u64>) -> Vec<IoValue> {
    rates.iter().map(|(key, used)| record("rate", vec![string(key), u64_value(*used)])).collect()
}

fn election_records(elections: &OrderedMap<String, ElectionState>) -> Vec<IoValue> {
    elections
        .iter()
        .map(|(key, election)| {
            record("election", vec![
                string(key),
                string(&election.leader),
                u64_value(election.token),
                string(&election.token_ref),
            ])
        })
        .collect()
}

fn barrier_records(barriers: &OrderedMap<String, BarrierState>) -> Vec<IoValue> {
    barriers
        .iter()
        .map(|(key, barrier)| {
            let participants = barrier.participants.iter().cloned().collect::<Vec<_>>();
            record("barrier", vec![
                string(key),
                strings_sequence(&participants),
                u64_value(barrier.required),
                string(if barrier.is_released { "released" } else { "waiting" }),
            ])
        })
        .collect()
}

fn registry_records(registry: &OrderedMap<String, RegistryEntry>) -> Vec<IoValue> {
    registry
        .iter()
        .map(|(key, entry)| {
            record("registry-entry", vec![string(key), string(&entry.endpoint_ref), string(&entry.evidence_ref)])
        })
        .collect()
}
