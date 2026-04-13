//! Optional-app request metadata grouped by domain.
//!
//! Each domain owns its own request-to-app mapping so adding a Forge RPC does
//! not require editing a shared cross-domain table.

#[path = "request_metadata_apps/automerge.rs"]
mod automerge;
#[path = "request_metadata_apps/calendar.rs"]
mod calendar;
#[path = "request_metadata_apps/ci.rs"]
mod ci;
#[path = "request_metadata_apps/contacts.rs"]
mod contacts;
#[path = "request_metadata_apps/deploy.rs"]
mod deploy;
#[path = "request_metadata_apps/forge.rs"]
mod forge;
#[path = "request_metadata_apps/hooks.rs"]
mod hooks;
#[path = "request_metadata_apps/jobs.rs"]
mod jobs;
#[path = "request_metadata_apps/secrets.rs"]
mod secrets;
#[path = "request_metadata_apps/snix.rs"]
mod snix;
#[path = "request_metadata_apps/sql.rs"]
mod sql;

pub(crate) fn request_required_app_for_variant(variant_name: &str) -> Option<&'static str> {
    automerge::required_app(variant_name)
        .or_else(|| calendar::required_app(variant_name))
        .or_else(|| ci::required_app(variant_name))
        .or_else(|| contacts::required_app(variant_name))
        .or_else(|| deploy::required_app(variant_name))
        .or_else(|| forge::required_app(variant_name))
        .or_else(|| hooks::required_app(variant_name))
        .or_else(|| jobs::required_app(variant_name))
        .or_else(|| secrets::required_app(variant_name))
        .or_else(|| snix::required_app(variant_name))
        .or_else(|| sql::required_app(variant_name))
}
