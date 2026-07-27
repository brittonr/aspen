mod binding;
mod model;
mod semantic;

pub use binding::classify_generation;
pub use binding::diagnose_retirement;
pub use binding::diagnose_transition;
pub use binding::plan_molten_cutover;
pub use binding::resolve_system_extension_callback;
pub use binding::resolve_unit_once;
pub use binding::retirement_is_complete;
pub use binding::validate_source_pins;
pub use model::*;
pub use semantic::admit_directional_compatibility;
pub use semantic::derive_semantic_subject_identity;
pub use semantic::diagnose_semantic_mismatch;
pub use semantic::validate_exact_handler;
pub use semantic::validate_semantic_surfaces;

#[cfg(test)]
mod tests;
