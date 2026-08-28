use super::*;

const INVENTORY_IDENTITY_DOMAIN: &str = "molten.world-mutation-inventory.identity.v1";
const PROFILE_IDENTITY_DOMAIN: &str = "molten.world-fault-profile.identity.v1";
const SCHEDULE_IDENTITY_DOMAIN: &str = "molten.world-fault-schedule.identity.v1";
const BLAKE3_PREFIX: &str = "blake3:";
const BLAKE3_HEX_LENGTH: usize = 64;

struct IdentityEncoder {
    bytes: Vec<u8>,
    issue: Option<WorldFaultIssue>,
}

impl IdentityEncoder {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            issue: None,
        }
    }

    fn push_usize(&mut self, field: &'static str, value: usize) {
        match u64::try_from(value) {
            Ok(value) => self.push_u64(value),
            Err(_) => self.record_overflow(field),
        }
    }

    fn push_u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn push_string(&mut self, field: &'static str, value: &str) {
        match u64::try_from(value.len()) {
            Ok(length) => {
                self.push_u64(length);
                self.bytes.extend_from_slice(value.as_bytes());
            }
            Err(_) => self.record_overflow(field),
        }
    }

    fn finish(self) -> Result<Vec<u8>, Vec<WorldFaultIssue>> {
        match self.issue {
            Some(issue) => Err(vec![issue]),
            None => Ok(self.bytes),
        }
    }

    fn record_overflow(&mut self, field: &'static str) {
        if self.issue.is_none() {
            self.issue = Some(WorldFaultIssue::IdentityLengthOverflow(field));
        }
    }
}

// r[impl molten.world_faults.inventory]
pub fn identify_world_mutation_inventory(inventory: &WorldMutationInventory) -> Result<String, Vec<WorldFaultIssue>> {
    let issues = validate_world_mutation_inventory(inventory, &registered_world_mutation_names());
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut encoder = IdentityEncoder::new();
    encoder.push_string("identity-domain", INVENTORY_IDENTITY_DOMAIN);
    encoder.push_string("schema", inventory.schema);
    encoder.push_u64(u64::from(inventory.version));
    encoder.push_usize("inventory-rows", inventory.rows.len());
    for row in &inventory.rows {
        encoder.push_string("mutation", row.mutation.as_str());
        encoder.push_string("owner", row.owner.as_str());
        encoder.push_string("operation-domain", row.operation_domain.as_str());
        encoder.push_string("expected-pre-state", row.expected_pre_state);
        encoder.push_usize("effects", row.effects.len());
        for effect in &row.effects {
            encoder.push_string("effect", effect.as_str());
        }
        encoder.push_string("linearization-point", row.linearization_point.as_str());
        encoder.push_string("durable-record", row.durable_record.as_str());
        encoder.push_string("uncertain-window", row.uncertain_window.as_str());
        encoder.push_string("reconciliation-entry", row.reconciliation_entry.as_str());
        encoder.push_usize("required-phases", row.required_phases.len());
        for phase in &row.required_phases {
            encoder.push_string("phase", phase.as_str());
        }
        encoder.push_usize("required-cases", row.required_cases.len());
        for case in &row.required_cases {
            encoder.push_string("required-case", case.as_str());
        }
        encoder.push_string("support", match row.support {
            MutationSupport::Supported => "supported",
            MutationSupport::UnsupportedIndependentWitness => "unsupported-independent-witness",
        });
    }
    Ok(content_ref(&encoder.finish()?))
}

// r[impl molten.world_faults.profile]
pub fn identify_world_fault_profile(profile: &WorldFaultProfile) -> Result<String, Vec<WorldFaultIssue>> {
    let issues = validate_world_fault_profile(profile);
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut encoder = IdentityEncoder::new();
    encoder.push_string("identity-domain", PROFILE_IDENTITY_DOMAIN);
    encoder.push_string("schema", profile.schema);
    encoder.push_string("profile-name", &profile.profile_name);
    encoder.push_string("source-revision", &profile.source_revision);
    encoder.push_string("inventory-ref", &profile.inventory_ref);
    encode_limits(&mut encoder, profile.limits);
    encoder.push_usize("adapters", profile.adapters.len());
    for adapter in &profile.adapters {
        encoder.push_string("adapter-id", &adapter.adapter_id);
        encoder.push_string("adapter-owner", adapter.owner.as_str());
        encoder.push_string("adapter-profile", &adapter.profile);
        encoder.push_string("adapter-implementation-ref", &adapter.implementation_ref);
        encoder.push_string("semantic-phase-map-ref", &adapter.semantic_phase_map_ref);
    }
    encoder.push_usize("cases", profile.cases.len());
    for case in &profile.cases {
        encoder.push_string("case-id", &case.case_id);
        encoder.push_string("case-mutation", case.mutation.as_str());
        encoder.push_string("case-operation-id", &case.operation_id);
        encoder.push_string("case-phase", case.phase.as_str());
        encoder.push_string("case-adapter-id", &case.adapter_id);
        encoder.push_u64(case.expected_generation);
        encoder.push_string("case-pre-state-ref", &case.pre_state_ref);
        encoder.push_string("case-expected-decision", case.expected_decision.as_str());
    }
    encoder.push_usize("schedules", profile.schedules.len());
    for schedule in &profile.schedules {
        encode_schedule(&mut encoder, schedule);
    }
    Ok(content_ref(&encoder.finish()?))
}

pub fn identify_concurrent_schedule(schedule: &ConcurrentSchedule) -> Result<String, Vec<WorldFaultIssue>> {
    let mut encoder = IdentityEncoder::new();
    encoder.push_string("identity-domain", SCHEDULE_IDENTITY_DOMAIN);
    encode_schedule(&mut encoder, schedule);
    Ok(content_ref(&encoder.finish()?))
}

pub(crate) fn is_blake3_ref(value: &str) -> bool {
    let Some(hex) = value.strip_prefix(BLAKE3_PREFIX) else {
        return false;
    };
    hex.len() == BLAKE3_HEX_LENGTH && hex.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(crate) fn reference(label: &str) -> String {
    content_ref(label.as_bytes())
}

fn content_ref(bytes: &[u8]) -> String {
    format!("{BLAKE3_PREFIX}{}", blake3::hash(bytes).to_hex())
}

fn encode_limits(encoder: &mut IdentityEncoder, limits: WorldFaultLimits) {
    encoder.push_usize("max-cases", limits.max_cases);
    encoder.push_usize("max-schedules", limits.max_schedules);
    encoder.push_usize("max-schedule-steps", limits.max_schedule_steps);
    encoder.push_usize("max-adapters", limits.max_adapters);
    encoder.push_usize("max-observations", limits.max_observations);
    encoder.push_usize("max-unsupported-rows", limits.max_unsupported_rows);
    encoder.push_u64(u64::from(limits.max_restarts));
}

fn encode_schedule(encoder: &mut IdentityEncoder, schedule: &ConcurrentSchedule) {
    encoder.push_string("schedule-id", &schedule.schedule_id);
    encoder.push_string("schedule-mutation", schedule.mutation.as_str());
    encoder.push_usize("schedule-steps", schedule.steps.len());
    for step in &schedule.steps {
        encoder.push_u64(u64::from(step.position));
        encoder.push_string("schedule-operation-id", &step.operation_id);
        encoder.push_string("schedule-step-mutation", step.mutation.as_str());
        encoder.push_u64(step.expected_generation);
        encoder.push_string("schedule-pre-state-ref", &step.pre_state_ref);
        encoder.push_string("schedule-interleaving", step.interleaving.as_str());
        encoder.push_string("schedule-node-id", &step.node_id);
        encoder.push_u64(step.node_generation);
    }
}
