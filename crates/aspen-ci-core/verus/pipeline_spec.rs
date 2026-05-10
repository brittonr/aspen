//! Verus specifications for CI pipeline validation helpers.
//!
//! This module proves small deterministic kernels behind pipeline admission and
//! ordering without pulling production strings, Vec<String> errors, or async CI
//! orchestration into Verus.

use vstd::prelude::*;

verus! {

pub const LIMIT_OK: u8 = 0;
pub const LIMIT_TOO_MANY_STAGES: u8 = 1;
pub const LIMIT_TOO_MANY_JOBS: u8 = 2;

pub open spec fn contains_u32_spec(values: Seq<u32>, needle: u32) -> bool {
    exists|i: int| #![auto] 0 <= i < values.len() && values[i] == needle
}

pub open spec fn dependency_ready_spec(
    dependencies: Seq<u32>,
    completed_stages: Seq<u32>,
) -> bool {
    forall|i: int| #![auto] 0 <= i < dependencies.len()
        ==> contains_u32_spec(completed_stages, dependencies[i])
}

pub open spec fn stage_ready_spec(
    stage: u32,
    dependencies: Seq<u32>,
    completed_stages: Seq<u32>,
    started_stages: Seq<u32>,
) -> bool {
    !contains_u32_spec(started_stages, stage)
        && dependency_ready_spec(dependencies, completed_stages)
}

pub open spec fn pipeline_limit_outcome_spec(
    stage_count: u32,
    job_count: u32,
    max_stages: u32,
    max_jobs: u32,
) -> u8 {
    if stage_count > max_stages {
        LIMIT_TOO_MANY_STAGES
    } else if job_count > max_jobs {
        LIMIT_TOO_MANY_JOBS
    } else {
        LIMIT_OK
    }
}

pub open spec fn saturating_add_u32_spec(left: u32, right: u32) -> u32 {
    if left as int + right as int > 4294967295int {
        4294967295u32
    } else {
        (left + right) as u32
    }
}

pub open spec fn edge_order_spec(order: Seq<u32>, dependency: u32, dependent: u32) -> bool {
    exists|dep_pos: int, stage_pos: int| #![auto]
        0 <= dep_pos < stage_pos < order.len()
        && order[dep_pos] == dependency
        && order[stage_pos] == dependent
}

pub fn contains_u32_exec(values: &[u32], needle: u32) -> (result: bool)
    ensures result == contains_u32_spec(values@, needle)
{
    let mut found = false;
    let mut index: usize = 0;
    while index < values.len()
        invariant
            0 <= index <= values.len(),
            found == exists|i: int| #![auto] 0 <= i < index && values@[i] == needle,
        decreases values.len() - index
    {
        if values[index] == needle {
            found = true;
        }
        index += 1;
    }
    assert(found == exists|i: int| #![auto] 0 <= i < values@.len() && values@[i] == needle);
    found
}

pub fn are_dependencies_met_exec(dependencies: &[u32], completed_stages: &[u32]) -> (result: bool)
    ensures result == dependency_ready_spec(dependencies@, completed_stages@)
{
    let mut all_met = true;
    let mut index: usize = 0;
    while index < dependencies.len()
        invariant
            0 <= index <= dependencies.len(),
            all_met == forall|i: int| #![auto] 0 <= i < index
                ==> contains_u32_spec(completed_stages@, dependencies@[i]),
        decreases dependencies.len() - index
    {
        if !contains_u32_exec(completed_stages, dependencies[index]) {
            all_met = false;
        }
        index += 1;
    }
    assert(all_met == dependency_ready_spec(dependencies@, completed_stages@));
    all_met
}

pub fn is_stage_ready_exec(
    stage: u32,
    dependencies: &[u32],
    completed_stages: &[u32],
    started_stages: &[u32],
) -> (result: bool)
    ensures result == stage_ready_spec(stage, dependencies@, completed_stages@, started_stages@)
{
    !contains_u32_exec(started_stages, stage) && are_dependencies_met_exec(dependencies, completed_stages)
}

pub fn pipeline_limit_outcome_exec(
    stage_count: u32,
    job_count: u32,
    max_stages: u32,
    max_jobs: u32,
) -> (result: u8)
    ensures result == pipeline_limit_outcome_spec(stage_count, job_count, max_stages, max_jobs)
{
    if stage_count > max_stages {
        LIMIT_TOO_MANY_STAGES
    } else if job_count > max_jobs {
        LIMIT_TOO_MANY_JOBS
    } else {
        LIMIT_OK
    }
}

pub fn saturating_add_u32_exec(left: u32, right: u32) -> (result: u32)
    ensures
        result == saturating_add_u32_spec(left, right),
        result >= left,
        result >= right,
{
    if left > 4294967295u32 - right {
        4294967295u32
    } else {
        left + right
    }
}

pub proof fn limit_checks_stage_before_jobs(
    stage_count: u32,
    job_count: u32,
    max_stages: u32,
    max_jobs: u32,
)
    ensures
        stage_count > max_stages ==> pipeline_limit_outcome_spec(stage_count, job_count, max_stages, max_jobs) == LIMIT_TOO_MANY_STAGES,
        stage_count <= max_stages && job_count > max_jobs ==> pipeline_limit_outcome_spec(stage_count, job_count, max_stages, max_jobs) == LIMIT_TOO_MANY_JOBS,
        stage_count <= max_stages && job_count <= max_jobs ==> pipeline_limit_outcome_spec(stage_count, job_count, max_stages, max_jobs) == LIMIT_OK,
{
}

pub proof fn no_dependencies_are_ready(completed_stages: Seq<u32>)
    ensures dependency_ready_spec(Seq::<u32>::empty(), completed_stages)
{
}

pub proof fn started_stage_is_never_ready(
    stage: u32,
    dependencies: Seq<u32>,
    completed_stages: Seq<u32>,
    started_stages: Seq<u32>,
)
    requires contains_u32_spec(started_stages, stage)
    ensures !stage_ready_spec(stage, dependencies, completed_stages, started_stages)
{
}

pub proof fn edge_order_implies_dependency_position_before_stage(
    order: Seq<u32>,
    dependency: u32,
    dependent: u32,
)
    requires edge_order_spec(order, dependency, dependent)
    ensures exists|dep_pos: int, stage_pos: int| #![auto]
        0 <= dep_pos < stage_pos < order.len()
        && order[dep_pos] == dependency
        && order[stage_pos] == dependent
{
}

} // verus!
