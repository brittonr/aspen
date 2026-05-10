//! Verus specification for pressure-based worker capacity evaluation.
//!
//! Verifies the deterministic threshold-composition kernel with scaled integer
//! pressure values. Runtime `f32`/`f64` measurement collection and scaling remain
//! shell concerns; once scaled, capacity is a pure conjunction of rejection gates.

use vstd::prelude::*;

verus! {

pub open spec fn pressure_within_limits(
    cpu: u64,
    mem: u64,
    io: u64,
    disk_build: u64,
    disk_store: u64,
    cpu_max: u64,
    mem_max: u64,
    io_max: u64,
    disk_build_min: u64,
    disk_store_min: u64,
) -> bool {
    cpu <= cpu_max
        && mem <= mem_max
        && io <= io_max
        && disk_build >= disk_build_min
        && disk_store >= disk_store_min
}

pub fn verify_cpu_rejection(cpu: u64, cpu_max: u64) -> (result: bool)
    requires cpu > cpu_max
    ensures result == false
{
    cpu <= cpu_max
}

pub fn verify_memory_rejection(mem: u64, mem_max: u64) -> (result: bool)
    requires mem > mem_max
    ensures result == false
{
    mem <= mem_max
}

pub fn verify_io_rejection(io: u64, io_max: u64) -> (result: bool)
    requires io > io_max
    ensures result == false
{
    io <= io_max
}

pub fn verify_disk_rejection(free_pct: u64, min_pct: u64) -> (result: bool)
    requires free_pct < min_pct
    ensures result == false
{
    free_pct >= min_pct
}

pub fn verify_all_within_thresholds(
    cpu: u64,
    mem: u64,
    io: u64,
    disk_build: u64,
    disk_store: u64,
    cpu_max: u64,
    mem_max: u64,
    io_max: u64,
    disk_build_min: u64,
    disk_store_min: u64,
) -> (result: bool)
    requires
        cpu <= cpu_max,
        mem <= mem_max,
        io <= io_max,
        disk_build >= disk_build_min,
        disk_store >= disk_store_min,
    ensures result == true
{
    cpu <= cpu_max
        && mem <= mem_max
        && io <= io_max
        && disk_build >= disk_build_min
        && disk_store >= disk_store_min
}

pub proof fn cpu_above_limit_rejects(
    cpu: u64,
    mem: u64,
    io: u64,
    disk_build: u64,
    disk_store: u64,
    cpu_max: u64,
    mem_max: u64,
    io_max: u64,
    disk_build_min: u64,
    disk_store_min: u64,
)
    requires cpu > cpu_max
    ensures !pressure_within_limits(cpu, mem, io, disk_build, disk_store, cpu_max, mem_max, io_max, disk_build_min, disk_store_min)
{
}

pub proof fn disk_below_minimum_rejects(
    cpu: u64,
    mem: u64,
    io: u64,
    disk_build: u64,
    disk_store: u64,
    cpu_max: u64,
    mem_max: u64,
    io_max: u64,
    disk_build_min: u64,
    disk_store_min: u64,
)
    requires disk_build < disk_build_min || disk_store < disk_store_min
    ensures !pressure_within_limits(cpu, mem, io, disk_build, disk_store, cpu_max, mem_max, io_max, disk_build_min, disk_store_min)
{
}

}
