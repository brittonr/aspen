pub use molten_composition_core::judgment::PropertyOutcome;
pub use molten_composition_core::judgment::ServiceProperty;
pub use molten_composition_core::judgment::ServiceRole;
pub use molten_composition_core::judgment::evaluate_properties;
pub use molten_composition_core::quantum::SaturatingWork;
pub use molten_composition_core::quantum::SaturationSlice;
pub use molten_composition_core::vocabulary::FailureScenario;
pub use molten_composition_core::vocabulary::ScenarioFacts;
pub use molten_composition_core::vocabulary::ScenarioVariant;

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/composition/parts/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/composition/parts/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/composition/parts/p002/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/composition/parts/p004/body.rs"));
#[path = "upgrade.rs"]
mod upgrade;
