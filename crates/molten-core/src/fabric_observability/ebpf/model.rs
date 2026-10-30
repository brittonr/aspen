//! Closed normalized observation contract shared with the OnixOS producer.
//! Capabilities and exact independently selected bindings remain mandatory.
use super::super::*;

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct EbpfBinding {
    pub machine: String,
    pub generation: String,
    pub pack: String,
    pub program: String,
    pub object: String,
    pub kernel: String,
    pub btf: String,
    pub schema: String,
    pub scope: String,
    pub accounting: String,
    pub adapter: String,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Deserialize,
    serde::Serialize
)]
#[serde(rename_all = "kebab-case")]
pub enum EbpfSignal {
    Process,
    Scheduler,
    Block,
    Network,
    Pressure,
    AdapterHealth,
}

impl EbpfSignal {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Process => "process",
            Self::Scheduler => "scheduler",
            Self::Block => "block",
            Self::Network => "network",
            Self::Pressure => "pressure",
            Self::AdapterHealth => "adapter-health",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbpfMapping {
    pub signal: EbpfSignal,
    pub descriptor: MetricDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbpfProfile {
    pub binding: EbpfBinding,
    pub mappings: Vec<EbpfMapping>,
    pub max_records: usize,
    pub max_bytes: usize,
    pub max_streams: usize,
    pub window_ticks: u64,
    pub freshness_ticks: u64,
    pub required: bool,
}

/// No strings, labels, payload, kernel pointers or arbitrary fields cross this DTO.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct EbpfRecord {
    pub stream: u32,
    pub sequence: u64,
    pub tick: u64,
    pub signal: EbpfSignal,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct EbpfAccounting {
    pub attempts: u64,
    pub submitted: u64,
    pub producer_dropped: u64,
    pub consumer_dropped: u64,
    pub pending: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct EbpfWindow {
    pub schema: String,
    pub binding: EbpfBinding,
    pub generation: u64,
    pub start: u64,
    pub end: u64,
    pub records: Vec<EbpfRecord>,
    #[serde(skip)]
    pub encoded_bytes: usize,
    pub accounting: Option<EbpfAccounting>,
    pub final_close_succeeded: bool,
    pub cancelled: bool,
    pub cross_stream_total_order_claimed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbpfIssue {
    Binding,
    Bounds,
    Mapping,
    Scope,
    Stale,
    Ordering,
    Sequence,
    Accounting,
    Overflow,
    Canonical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbpfProjection {
    pub samples: Vec<MetricSample>,
    pub health: HealthInput,
    pub complete: bool,
}
