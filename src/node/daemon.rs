//! Node daemon: local lifecycle, control dispatch, the control service, live Iroh ingress, and live
//! workflow bundles over a capability-scoped state root.
//!
//! Each submodule owns one responsibility and refers to its siblings by qualified path. The
//! `pub use` globs below are the public facade that `molten::node_daemon` re-exports.

#[path = "ingress/mod.rs"]
mod ingress;
#[path = "source-binding/mod.rs"]
mod source_binding;
pub use source_binding::AuthenticatedGate;
pub use source_binding::GateValidation;
pub use source_binding::SelectedNodeOutputValidation;
pub use source_binding::authenticated_gate_with_root;
pub use source_binding::gate_validation_with_root;
pub use source_binding::start_runtime_with_root;
pub use source_binding::verify_selected_node_output_with_root;

#[path = "daemon/workflow/acking.rs"]
mod acking;
#[path = "daemon/records/acknowledgements.rs"]
mod acknowledgements;
#[path = "daemon/encoding/acks.rs"]
mod acks;
#[path = "daemon/trust/admissibility.rs"]
mod admissibility;
#[path = "daemon/state/ambient.rs"]
mod ambient;
#[path = "daemon/workflow/applying.rs"]
mod applying;
#[path = "daemon/vocabulary/artifacts.rs"]
mod artifacts;
#[path = "daemon/listen/attach.rs"]
mod attach;
#[path = "daemon/live/attestation.rs"]
mod attestation;
#[path = "daemon/access/attestations.rs"]
mod attestations;
#[path = "daemon/live/backoff.rs"]
mod backoff;
#[path = "daemon/lifecycle/bootstrapping.rs"]
mod bootstrapping;
#[path = "daemon/vocabulary/bundling.rs"]
mod bundling;
#[path = "daemon/listen/chronicle.rs"]
mod chronicle;
#[path = "daemon/preflight/delivery.rs"]
mod delivery;
#[path = "daemon/workflow/discrepancies.rs"]
mod discrepancies;
#[path = "daemon/service/draining.rs"]
mod draining;
#[path = "daemon/live/duplicates.rs"]
mod duplicates;
#[path = "daemon/listen/endpoints.rs"]
mod endpoints;
#[path = "daemon/access/entitlements.rs"]
mod entitlements;
#[path = "daemon/workflow/exporting.rs"]
mod exporting;
#[path = "daemon/encoding/exports.rs"]
mod exports;
#[path = "daemon/records/fields.rs"]
mod fields;
#[path = "daemon/encoding/gates.rs"]
mod gates;
#[path = "daemon/dispatch/gating.rs"]
mod gating;
#[path = "daemon/records/guards.rs"]
mod guards;
#[path = "daemon/dispatch/handlers.rs"]
mod handlers;
#[path = "daemon/protocol/handshake.rs"]
mod handshake;
#[path = "daemon/access/importing.rs"]
mod importing;
#[path = "daemon/live/inbound.rs"]
mod inbound;
#[path = "daemon/stamping/ingresses.rs"]
mod ingresses;
#[path = "daemon/records/inspection.rs"]
mod inspection;
#[path = "daemon/dispatch/installs.rs"]
mod installs;
#[path = "daemon/protocol/intake.rs"]
mod intake;
#[path = "daemon/access/issuance.rs"]
mod issuance;
#[path = "daemon/dispatch/launches.rs"]
mod launches;
#[path = "daemon/state/leases.rs"]
mod leases;
#[path = "daemon/listen/listening.rs"]
mod listening;
#[path = "daemon/trust/mandates.rs"]
mod mandates;
#[path = "daemon/records/manifests.rs"]
mod manifests;
#[path = "daemon/stamping/narration.rs"]
mod narration;
#[path = "daemon/trust/newcomers.rs"]
mod newcomers;
#[path = "daemon/protocol/notes.rs"]
mod notes;
#[path = "daemon/vocabulary/offers.rs"]
mod offers;
#[path = "daemon/stamping/operations.rs"]
mod operations;
#[path = "daemon/vocabulary/outcomes.rs"]
mod outcomes;
#[path = "daemon/stamping/overview.rs"]
mod overview;
#[path = "daemon/state/paths.rs"]
mod paths;
#[path = "daemon/access/peering.rs"]
mod peering;
#[path = "daemon/stamping/phrasing.rs"]
mod phrasing;
#[path = "daemon/live/publishing.rs"]
mod publishing;
#[path = "daemon/records/receipts.rs"]
mod receipts;
#[path = "daemon/live/receiving.rs"]
mod receiving;
#[path = "daemon/encoding/reconciles.rs"]
mod reconciles;
#[path = "daemon/workflow/reconciling.rs"]
mod reconciling;
#[path = "daemon/vocabulary/requests.rs"]
mod requests;
#[path = "daemon/service/restarts.rs"]
mod restarts;
#[path = "daemon/lifecycle/running.rs"]
mod running;
#[path = "daemon/access/scoping.rs"]
mod scoping;
#[path = "daemon/live/sending.rs"]
mod sending;
#[path = "daemon/encoding/sends.rs"]
mod sends;
#[path = "daemon/protocol/sequencing.rs"]
mod sequencing;
#[path = "daemon/stamping/services.rs"]
mod services;
#[path = "daemon/service/serving.rs"]
mod serving;
#[path = "daemon/vocabulary/settings.rs"]
mod settings;
#[path = "daemon/preflight/shaping.rs"]
mod shaping;
#[path = "daemon/stamping/stamps.rs"]
mod stamps;
#[path = "daemon/workflow/steps.rs"]
mod steps;
#[path = "daemon/lifecycle/stopping.rs"]
mod stopping;
#[path = "daemon/state/storage.rs"]
mod storage;
#[path = "daemon/lifecycle/submission.rs"]
mod submission;
#[path = "daemon/lifecycle/supervision.rs"]
mod supervision;
#[cfg(test)]
#[path = "daemon/tests/mod.rs"]
mod tests;
#[path = "daemon/service/ticking.rs"]
mod ticking;
#[path = "daemon/state/tracking.rs"]
mod tracking;
#[path = "daemon/protocol/transcript.rs"]
mod transcript;
#[path = "daemon/vocabulary/transits.rs"]
mod transits;
#[path = "daemon/encoding/transports.rs"]
mod transports;
#[path = "daemon/preflight/tuning.rs"]
mod tuning;
#[path = "daemon/workflow/verifying.rs"]
mod verifying;
#[path = "daemon/preflight/vetting.rs"]
mod vetting;
#[path = "daemon/state/writers.rs"]
mod writers;

pub use self::acking::*;
pub use self::acknowledgements::*;
pub use self::ambient::*;
pub use self::applying::*;
pub use self::artifacts::*;
pub use self::attach::*;
pub use self::attestation::*;
pub use self::bootstrapping::*;
pub use self::bundling::*;
pub use self::chronicle::*;
pub use self::delivery::*;
pub use self::entitlements::*;
pub use self::exporting::*;
pub use self::handshake::*;
pub use self::importing::*;
pub use self::ingresses::*;
pub use self::inspection::*;
pub use self::intake::*;
pub use self::issuance::*;
pub use self::listening::*;
pub use self::manifests::*;
pub use self::offers::*;
pub use self::outcomes::*;
pub use self::overview::*;
pub use self::peering::*;
pub use self::publishing::*;
pub use self::receipts::*;
pub use self::receiving::*;
pub use self::reconciling::*;
pub use self::requests::*;
pub use self::running::*;
pub use self::scoping::*;
pub use self::sending::*;
pub use self::sequencing::*;
pub use self::serving::*;
pub use self::settings::*;
pub use self::shaping::*;
pub use self::stopping::*;
pub use self::submission::*;
pub use self::supervision::*;
pub use self::transits::*;
pub use self::verifying::*;
pub use self::vetting::*;
