use super::WorldHeadReferenceError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldBranchClass {
    Local,
    Candidate,
    Release,
}

impl WorldBranchClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Candidate => "candidate",
            Self::Release => "release",
        }
    }

    pub fn parse(value: &str) -> Result<Self, WorldHeadReferenceError> {
        match value {
            "local" => Ok(Self::Local),
            "candidate" => Ok(Self::Candidate),
            "release" => Ok(Self::Release),
            _ => Err(WorldHeadReferenceError::InvalidCharacter),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldHeadPurpose {
    Create,
    Advance,
    Merge,
    Recovery,
}

impl WorldHeadPurpose {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Advance => "advance",
            Self::Merge => "merge",
            Self::Recovery => "recovery",
        }
    }

    pub fn parse(value: &str) -> Result<Self, WorldHeadReferenceError> {
        match value {
            "create" => Ok(Self::Create),
            "advance" => Ok(Self::Advance),
            "merge" => Ok(Self::Merge),
            "recovery" => Ok(Self::Recovery),
            _ => Err(WorldHeadReferenceError::InvalidCharacter),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldHeadSignerRole {
    Maintainer,
    Release,
    Recovery,
}

impl WorldHeadSignerRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Maintainer => "maintainer",
            Self::Release => "release",
            Self::Recovery => "recovery",
        }
    }

    pub fn parse(value: &str) -> Result<Self, WorldHeadReferenceError> {
        match value {
            "maintainer" => Ok(Self::Maintainer),
            "release" => Ok(Self::Release),
            "recovery" => Ok(Self::Recovery),
            _ => Err(WorldHeadReferenceError::InvalidCharacter),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldHeadCurrentnessClass {
    RelativeToObservedStore,
    IndependentObservation,
    WholeStoreRollbackUnproven,
}

impl WorldHeadCurrentnessClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RelativeToObservedStore => "relative-to-observed-store",
            Self::IndependentObservation => "independent-observation",
            Self::WholeStoreRollbackUnproven => "whole-store-rollback-unproven",
        }
    }
}
