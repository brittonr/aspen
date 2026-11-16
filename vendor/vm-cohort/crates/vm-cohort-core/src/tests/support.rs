use alloc::format;
use alloc::string::{String, ToString};

use super::super::*;

pub(super) const WORKERS: u32 = 2;
pub(super) const VCPU_COUNT: u32 = 2;
pub(super) const DEVICE_COUNT: u32 = 3;
pub(super) const MEMORY_BYTES: u64 = 8_388_608;
pub(super) const DISK_BYTES: u64 = 16_777_216;
pub(super) const PAGE_BYTES: u64 = 4_096;
pub(super) const MAX_ATTEMPTS: u32 = 3;
pub(super) const RETRY_DELAY_TICKS: u64 = 2;
pub(super) const MAX_RETRY_DELAY_TICKS: u64 = 8;

pub(super) fn reference(seed: char) -> String {
    format!("blake3:{}", seed.to_string().repeat(64))
}

pub(super) fn resource(seed: char) -> ResourceRef {
    ResourceRef::new(reference(seed)).expect("test resource ref")
}

pub(super) fn profile() -> CompatibilityProfile {
    CompatibilityProfile {
        profile_ref: ProfileRef::new(reference('a')).expect("profile ref"),
        architecture: "x86_64".to_string(),
        vcpu_state_ref: resource('b'),
        memory_layout_ref: resource('c'),
        kernel_ref: resource('d'),
        guest_image_ref: resource('e'),
        device_model_ref: resource('f'),
        disk_format_ref: resource('1'),
        runtime_ref: resource('2'),
        adapter_ref: resource('3'),
    }
}

pub(super) fn candidate() -> CheckpointCandidate {
    CheckpointCandidate {
        compatibility: profile(),
        effective_memory_base_ref: resource('4'),
        effective_disk_base_ref: resource('5'),
        memory_bytes: MEMORY_BYTES,
        disk_bytes: DISK_BYTES,
        vcpu_count: VCPU_COUNT,
        device_count: DEVICE_COUNT,
        complete: true,
        host_handles_present: false,
        bases_mutable: false,
    }
}

pub(super) fn limits() -> ResourceLimits {
    ResourceLimits {
        policy_ref: ProfileRef::new(reference('6')).expect("policy ref"),
        maximum: ResourceVector {
            workers: WORKERS,
            vcpus: WORKERS * VCPU_COUNT,
            memory_bytes: u64::from(WORKERS) * MEMORY_BYTES,
            storage_bytes: u64::from(WORKERS) * DISK_BYTES,
            mappings: WORKERS * 2,
            endpoints: WORKERS,
            in_flight: WORKERS * 6,
        },
        max_attempts: MAX_ATTEMPTS,
        retry_delay_ticks: RETRY_DELAY_TICKS,
        max_retry_delay_ticks: MAX_RETRY_DELAY_TICKS,
        memory_page_bytes: PAGE_BYTES,
        disk_page_bytes: PAGE_BYTES,
    }
}

pub(super) fn request() -> CohortRequest {
    let checkpoint = admit_checkpoint(&candidate()).expect("checkpoint");
    CohortRequest {
        expected_compatibility: checkpoint.compatibility.clone(),
        checkpoint,
        workers: WORKERS,
        limits: limits(),
        context_ref: resource('7'),
    }
}

pub(super) fn receipt(seed: char) -> ReceiptRef {
    ReceiptRef::new(reference(seed)).expect("receipt ref")
}
