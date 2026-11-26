//! Record-shape readers, typed ids and their denials; published as [`crate::view`].

mod checkset;
mod denial;
mod ids;
mod readers;

pub use checkset::ADMITTED_CHECK_STATUSES;
pub use checkset::ParsedCheck;
pub use checkset::parse_checks_record;
pub use checkset::require_checks_present;
pub use denial::NESTED_INVALID_HARNESS_PREFIX;
pub use denial::ViewError;
pub use ids::CheckStatus;
pub use ids::Decision;
pub use ids::OperationId;
pub use ids::ProfileId;
pub use ids::ReplayClass;
pub use ids::SchemaId;
pub use ids::StableId;
pub use ids::validate_stable_id;
pub use readers::optional_content_ref;
pub use readers::optional_content_ref_string;
pub use readers::record_content_ref;
pub use readers::record_content_ref_sequence;
pub use readers::record_content_ref_string;
pub use readers::record_content_ref_strings;
pub use readers::record_string_field;
pub use readers::required_content_ref;
pub use readers::required_content_ref_string;
pub use readers::required_sequence_field;
pub use readers::required_string_field;
pub use readers::simple_record_fields;

#[cfg(test)]
mod tests;
