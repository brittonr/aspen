// SPDX-License-Identifier: AGPL-3.0-or-later
/// Finite logical caps. Process caps are whole-worker cumulative Linux limits,
/// checked by the host, not per-evaluation meters or sandbox claims.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub max_program_bytes: u64,
    pub max_input_bytes: u64,
    pub max_source_bytes: u32,
    pub max_source_depth: u32,
    pub max_source_nodes: u32,
    pub max_frontend_work: u32,
    pub max_extraction_work: u64,
    pub max_ir_nodes: u64,
    pub max_ir_bytes: u64,
    pub max_inputs: u32,
    pub max_outputs: u32,
    pub max_agent_slots: u64,
    pub max_wires: u64,
    pub max_queue_entries: u64,
    pub max_memory_bytes: u64,
    pub max_plan_scratch_bytes: u64,
    pub max_setup_work: u64,
    pub max_validation_work: u64,
    pub interaction_fuel: u64,
    pub reference_fuel: u64,
    pub max_readback_work: u64,
    pub max_observations: u32,
    pub quantum_work: u64,
    /// Largest indivisible reserved producer step. A local turn is bounded by
    /// quantum_work + max_atomic_work, independently from total attempt work.
    pub max_atomic_work: u64,
    pub max_total_work: u64,
    pub max_attempts: u32,
    pub max_aggregate_attempt_work: u64,
    pub max_concurrent_evaluations: u32,
    pub process_address_space_bytes: u64,
    pub process_cpu_soft_seconds: u64,
    pub process_cpu_hard_seconds: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_program_bytes: 131_072,
            max_input_bytes: 32_768,
            max_source_bytes: 8192,
            max_source_depth: 32,
            max_source_nodes: 2048,
            max_frontend_work: 100_000,
            max_extraction_work: 100_000,
            max_ir_nodes: 1024,
            max_ir_bytes: 65_536,
            max_inputs: 256,
            max_outputs: 256,
            max_agent_slots: 8192,
            max_wires: 16_384,
            max_queue_entries: 8192,
            max_memory_bytes: 64 * 1024 * 1024,
            max_plan_scratch_bytes: 16_384,
            max_setup_work: 2_000_000,
            max_validation_work: 128_000_000,
            interaction_fuel: 100_000,
            reference_fuel: 100_000,
            max_readback_work: 16_384,
            max_observations: 1024,
            quantum_work: 256,
            max_atomic_work: 16_000_000,
            max_total_work: 256_000_000,
            max_attempts: 4,
            max_aggregate_attempt_work: 1_024_000_000,
            max_concurrent_evaluations: 2,
            process_address_space_bytes: 8 * 1024 * 1024 * 1024,
            process_cpu_soft_seconds: 3600,
            process_cpu_hard_seconds: 3660,
        }
    }
}
impl Limits {
    pub fn validate(&self) -> Result<(), crate::types::Error> {
        let d = Self::default();
        macro_rules! bounded { ($($f:ident),+ $(,)?) => { $(if self.$f == 0 || self.$f > d.$f { return Err(crate::types::Error::InvalidLimits); })+ }; }
        bounded!(
            max_program_bytes,
            max_input_bytes,
            max_source_bytes,
            max_source_depth,
            max_source_nodes,
            max_frontend_work,
            max_extraction_work,
            max_ir_nodes,
            max_ir_bytes,
            max_inputs,
            max_outputs,
            max_agent_slots,
            max_wires,
            max_queue_entries,
            max_memory_bytes,
            max_plan_scratch_bytes,
            max_setup_work,
            max_validation_work,
            max_readback_work,
            max_observations,
            quantum_work,
            max_atomic_work,
            max_total_work,
            max_attempts,
            max_aggregate_attempt_work,
            max_concurrent_evaluations,
            process_address_space_bytes,
            process_cpu_soft_seconds,
            process_cpu_hard_seconds
        );
        if self.interaction_fuel > d.interaction_fuel
            || self.reference_fuel > d.reference_fuel
            || self.process_cpu_soft_seconds > self.process_cpu_hard_seconds
            || self
                .max_total_work
                .checked_mul(u64::from(self.max_attempts))
                .is_none_or(|v| v > self.max_aggregate_attempt_work)
        {
            return Err(crate::types::Error::InvalidLimits);
        }
        Ok(())
    }
}
