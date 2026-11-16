/// A system-extension generation.
///
/// This retains `u64` semantics intentionally; unlike upstream
/// database-friendly generation crates, it does not restrict values to
/// `0..=2^63-1`. Conversion to and from `u64` is explicit at
/// serialization and API boundaries.
///
/// ```compile_fail
/// use molten_core::system_extension::{Generation, LifecycleEvent, LifecycleEventKind};
/// let generation = Generation::new(1);
/// let _ = LifecycleEvent::simple(LifecycleEventKind::Install, generation, generation);
/// ```
///
/// ```compile_fail
/// use molten_core::system_extension::{Generation, Incarnation, LifecycleEvent, LifecycleEventKind};
/// use molten_core::system_extension::{LifecycleState, ResourceUsage, plan_lifecycle_transition};
/// let state = LifecycleState { incarnation: Generation::new(0), ..LifecycleState::absent() };
/// let event = LifecycleEvent::simple(LifecycleEventKind::Install, Generation::new(1), Incarnation::new(0));
/// let _ = plan_lifecycle_transition(&state, &event, ResourceUsage::default(), 3);
/// ```
///
/// ```compile_fail
/// use molten_core::system_extension::{Generation, Incarnation, LifecycleEvent, LifecycleEventKind};
/// use molten_core::system_extension::{LifecycleState, ResourceUsage, plan_lifecycle_transition};
/// let event = LifecycleEvent {
///     next_generation: Some(Incarnation::new(2)),
///     ..LifecycleEvent::simple(LifecycleEventKind::Install, Generation::new(1), Incarnation::new(0))
/// };
/// let _ = plan_lifecycle_transition(&LifecycleState::absent(), &event, ResourceUsage::default(), 3);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Generation(u64);

/// A system-extension incarnation.
///
/// This retains `u64` semantics intentionally. Conversion to and from
/// `u64` is explicit at serialization and API boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Incarnation(u64);

impl Generation {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub fn checked_add(self, rhs: u64) -> Option<Self> {
        self.0.checked_add(rhs).map(Self)
    }
}

impl Incarnation {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub fn checked_add(self, rhs: u64) -> Option<Self> {
        self.0.checked_add(rhs).map(Self)
    }
}

impl std::fmt::Display for Generation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for Incarnation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
