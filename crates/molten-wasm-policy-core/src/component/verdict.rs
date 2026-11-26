//! The classified component admission denial shared by every policy stage.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenialClass {
    FuelExhausted,
    DeadlineExceeded,
    InvalidPreservesPayload,
    ComponentCompilationDenied,
    ComponentInstantiationDenied,
    GuestDenial,
    ComponentTrap,
    ResourceDenial,
    AuthorityDenial,
    ProfileDenial,
    MaterializationDenial,
    ComponentAdmissionDenial,
}

impl DenialClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FuelExhausted => "fuel-exhausted",
            Self::DeadlineExceeded => "deadline-exceeded",
            Self::InvalidPreservesPayload => "invalid-preserves-payload",
            Self::ComponentCompilationDenied => "component-compilation-denied",
            Self::ComponentInstantiationDenied => "component-instantiation-denied",
            Self::GuestDenial => "guest-denial",
            Self::ComponentTrap => "component-trap",
            Self::ResourceDenial => "resource-denial",
            Self::AuthorityDenial => "authority-denial",
            Self::ProfileDenial => "profile-denial",
            Self::MaterializationDenial => "materialization-denial",
            Self::ComponentAdmissionDenial => "component-admission-denial",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "fuel-exhausted" => Some(Self::FuelExhausted),
            "deadline-exceeded" => Some(Self::DeadlineExceeded),
            "invalid-preserves-payload" => Some(Self::InvalidPreservesPayload),
            "component-compilation-denied" => Some(Self::ComponentCompilationDenied),
            "component-instantiation-denied" => Some(Self::ComponentInstantiationDenied),
            "guest-denial" => Some(Self::GuestDenial),
            "component-trap" => Some(Self::ComponentTrap),
            "resource-denial" => Some(Self::ResourceDenial),
            "authority-denial" => Some(Self::AuthorityDenial),
            "profile-denial" => Some(Self::ProfileDenial),
            "materialization-denial" => Some(Self::MaterializationDenial),
            "component-admission-denial" => Some(Self::ComponentAdmissionDenial),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Denial {
    pub blockers: alloc::vec::Vec<alloc::string::String>,
    class: DenialClass,
}

impl Denial {
    pub fn new(blocker: impl Into<alloc::string::String>) -> Self {
        Self::classified(DenialClass::ComponentAdmissionDenial, blocker)
    }

    pub fn classified(class: DenialClass, blocker: impl Into<alloc::string::String>) -> Self {
        Self {
            blockers: alloc::vec![blocker.into()],
            class,
        }
    }

    pub fn from_blockers(blockers: impl IntoIterator<Item = alloc::string::String>) -> Self {
        Self::from_classified_blockers(DenialClass::ComponentAdmissionDenial, blockers)
    }

    pub fn from_classified_blockers(
        class: DenialClass,
        blockers: impl IntoIterator<Item = alloc::string::String>,
    ) -> Self {
        let mut blockers = blockers.into_iter().collect::<alloc::vec::Vec<_>>();
        blockers.sort();
        blockers.dedup();
        Self { blockers, class }
    }

    pub const fn canonical_class(&self) -> &'static str {
        self.class.as_str()
    }
}

impl core::fmt::Display for Denial {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{}", self.blockers.join("; "))
    }
}

impl core::error::Error for Denial {}

pub type Result<T, E = Denial> = core::result::Result<T, E>;
