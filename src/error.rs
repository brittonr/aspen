use std::fmt;

pub type Result<T> = std::result::Result<T, MoltenError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessDivergence {
    pub kind: String,
    pub step: Option<u64>,
    pub expected: String,
    pub actual: String,
    pub detail: String,
}

impl HarnessDivergence {
    pub fn new(
        kind: impl Into<String>,
        step: Option<u64>,
        expected: impl Into<String>,
        actual: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            step,
            expected: expected.into(),
            actual: actual.into(),
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoltenError {
    Io(String),
    Preserves(String),
    InvalidHarness(String),
    HarnessDivergence(HarnessDivergence),
}

impl MoltenError {
    pub fn invalid_harness(message: impl Into<String>) -> Self {
        Self::InvalidHarness(message.into())
    }

    pub fn harness_divergence(divergence: HarnessDivergence) -> Self {
        Self::HarnessDivergence(divergence)
    }
}

impl fmt::Display for MoltenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoltenError::Io(message) => write!(f, "io error: {message}"),
            MoltenError::Preserves(message) => write!(f, "preserves error: {message}"),
            MoltenError::InvalidHarness(message) => write!(f, "invalid harness artifact: {message}"),
            MoltenError::HarnessDivergence(divergence) => {
                if let Some(step) = divergence.step {
                    write!(
                        f,
                        "divergence kind={} step={} expected={} actual={} detail={}",
                        divergence.kind, step, divergence.expected, divergence.actual, divergence.detail
                    )
                } else {
                    write!(
                        f,
                        "divergence kind={} expected={} actual={} detail={}",
                        divergence.kind, divergence.expected, divergence.actual, divergence.detail
                    )
                }
            }
        }
    }
}

impl std::error::Error for MoltenError {}

impl From<std::io::Error> for MoltenError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}
