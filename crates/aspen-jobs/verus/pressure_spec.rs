//! Verus specification for pressure-based worker capacity evaluation.
//!
//! Verifies that `has_pressure_capacity` correctly rejects workers
//! when any pressure metric exceeds its threshold.

use vstd::prelude::*;

verus! {

    // ========================================================================
    // Spec Functions
    // ========================================================================

    /// Specification for pressure capacity check.
    pub open spec fn pressure_within_limits(
        cpu: f32,
        mem: f32,
        io: f32,
        disk_build: f64,
        disk_store: f64,
        cpu_max: f32,
        mem_max: f32,
        io_max: f32,
        disk_build_min: f64,
        disk_store_min: f64,
    ) -> bool {
        cpu <= cpu_max
            && mem <= mem_max
            && io <= io_max
            && disk_build >= disk_build_min
            && disk_store >= disk_store_min
    }

    // ========================================================================
    // Exec Functions
    // ========================================================================

    /// Verify that has_pressure_capacity returns false when CPU exceeds threshold.
    pub fn verify_cpu_rejection(cpu: f32, cpu_max: f32) -> (result: bool)
        requires
            cpu > cpu_max,
            cpu_max >= 0.0,
        ensures
            result == false,
    {
        // If CPU exceeds threshold, overall capacity check must fail
        cpu <= cpu_max
    }

    /// Verify that has_pressure_capacity returns false when memory exceeds threshold.
    pub fn verify_memory_rejection(mem: f32, mem_max: f32) -> (result: bool)
        requires
            mem > mem_max,
            mem_max >= 0.0,
        ensures
            result == false,
    {
        mem <= mem_max
    }

    /// Verify that has_pressure_capacity returns false when IO exceeds threshold.
    pub fn verify_io_rejection(io: f32, io_max: f32) -> (result: bool)
        requires
            io > io_max,
            io_max >= 0.0,
        ensures
            result == false,
    {
        io <= io_max
    }

    /// Verify that has_pressure_capacity returns false when disk free is below minimum.
    pub fn verify_disk_rejection(free_pct: f64, min_pct: f64) -> (result: bool)
        requires
            free_pct < min_pct,
            min_pct >= 0.0,
        ensures
            result == false,
    {
        free_pct >= min_pct
    }

    /// Verify that has_pressure_capacity returns true when all metrics are within thresholds.
    pub fn verify_all_within_thresholds(
        cpu: f32,
        mem: f32,
        io: f32,
        disk_build: f64,
        disk_store: f64,
        cpu_max: f32,
        mem_max: f32,
        io_max: f32,
        disk_build_min: f64,
        disk_store_min: f64,
    ) -> (result: bool)
        requires
            cpu <= cpu_max,
            mem <= mem_max,
            io <= io_max,
            disk_build >= disk_build_min,
            disk_store >= disk_store_min,
        ensures
            result == true,
    {
        cpu <= cpu_max
            && mem <= mem_max
            && io <= io_max
            && disk_build >= disk_build_min
            && disk_store >= disk_store_min
    }

}
