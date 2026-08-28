#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompatibilityStatus {
    Compatible,
    Adapted,
    Intentional,
    Unsupported,
    EngineGap,
}

impl CompatibilityStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::Adapted => "adapted",
            Self::Intentional => "intentional",
            Self::Unsupported => "unsupported",
            Self::EngineGap => "engine-gap",
        }
    }

    pub const fn requires_issue(self) -> bool {
        matches!(self, Self::Unsupported | Self::EngineGap)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityRow {
    pub id: String,
    pub source_contract: String,
    pub status: CompatibilityStatus,
    pub evidence_ref: String,
    pub fixture: String,
    pub issue: Option<String>,
    pub explanation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompatibilityLimits {
    pub max_adapted: usize,
    pub max_intentional: usize,
    pub max_unsupported: usize,
    pub max_engine_gap: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilitySummary {
    pub compatible: usize,
    pub adapted: usize,
    pub intentional: usize,
    pub unsupported: usize,
    pub engine_gap: usize,
}
