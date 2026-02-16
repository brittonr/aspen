// Coordination primitive response types.

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockResultResponse {
    pub success: bool,
    pub fencing_token: Option<u64>,
    pub holder_id: Option<String>,
    pub deadline_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterResultResponse {
    pub success: bool,
    pub value: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedCounterResultResponse {
    pub success: bool,
    pub value: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceResultResponse {
    pub success: bool,
    pub value: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterResultResponse {
    pub success: bool,
    pub tokens_remaining: Option<u64>,
    pub retry_after_ms: Option<u64>,
    pub error: Option<String>,
}

// Barrier types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarrierResultResponse {
    pub success: bool,
    pub current_count: Option<u32>,
    pub required_count: Option<u32>,
    pub phase: Option<String>,
    pub error: Option<String>,
}

// Semaphore types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemaphoreResultResponse {
    pub success: bool,
    pub permits_acquired: Option<u32>,
    pub available: Option<u32>,
    pub capacity_permits: Option<u32>,
    pub retry_after_ms: Option<u64>,
    pub error: Option<String>,
}

// Read-Write Lock types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RWLockResultResponse {
    pub success: bool,
    pub mode: Option<String>,
    pub fencing_token: Option<u64>,
    pub deadline_ms: Option<u64>,
    pub reader_count: Option<u32>,
    pub writer_holder: Option<String>,
    pub error: Option<String>,
}
