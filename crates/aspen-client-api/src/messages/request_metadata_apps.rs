//! Optional-app request metadata grouped by domain.
//!
//! Each domain owns its own request-to-app mapping so adding a Forge RPC does
//! not require editing a shared cross-domain table.

#[cfg(test)]
pub(crate) struct RequestAppNamespacePrefixContract {
    pub(crate) app: &'static str,
    pub(crate) variant_prefixes: &'static [&'static str],
}

#[cfg(test)]
pub(crate) const APP_REQUEST_NAMESPACE_PREFIX_CONTRACTS: &[RequestAppNamespacePrefixContract] = &[
    RequestAppNamespacePrefixContract {
        app: "automerge",
        variant_prefixes: &["Automerge"],
    },
    RequestAppNamespacePrefixContract {
        app: "calendar",
        variant_prefixes: &["Calendar"],
    },
    RequestAppNamespacePrefixContract {
        app: "ci",
        variant_prefixes: &["Ci"],
    },
    RequestAppNamespacePrefixContract {
        app: "contacts",
        variant_prefixes: &["Contacts", "Net"],
    },
    RequestAppNamespacePrefixContract {
        app: "deploy",
        variant_prefixes: &["ClusterDeploy", "ClusterRollback", "NodeRollback", "NodeUpgrade"],
    },
    RequestAppNamespacePrefixContract {
        app: "forge",
        variant_prefixes: &[
            "FederateRepository",
            "Federation",
            "Forge",
            "GetDiscoveredCluster",
            "GetFederationStatus",
            "GitBridge",
            "Gossip",
            "ListDiscoveredClusters",
            "ListFederatedRepositories",
            "StartGossip",
            "StopGossip",
            "TrustCluster",
            "UntrustCluster",
        ],
    },
    RequestAppNamespacePrefixContract {
        app: "hooks",
        variant_prefixes: &["Hook"],
    },
    RequestAppNamespacePrefixContract {
        app: "jobs",
        variant_prefixes: &["Job", "Worker"],
    },
    RequestAppNamespacePrefixContract {
        app: "secrets",
        variant_prefixes: &["Secrets"],
    },
    RequestAppNamespacePrefixContract {
        app: "snix",
        variant_prefixes: &["Cache", "NixCache", "Snix"],
    },
    RequestAppNamespacePrefixContract {
        app: "sql",
        variant_prefixes: &["ExecuteSql"],
    },
];

#[path = "request_metadata_apps/automerge.rs"]
mod automerge;
#[path = "request_metadata_apps/calendar.rs"]
mod calendar;
#[path = "request_metadata_apps/ci.rs"]
mod ci;
pub use ci::CI_REQUEST_VARIANTS;
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

#[cfg(test)]
pub(super) const REQUIRED_APP_VARIANT_GROUPS: &[&[&str]] = &[
    automerge::REQUIRED_APP_VARIANTS,
    calendar::REQUIRED_APP_VARIANTS,
    ci::REQUIRED_APP_VARIANTS,
    contacts::REQUIRED_APP_VARIANTS,
    deploy::REQUIRED_APP_VARIANTS,
    forge::REQUIRED_APP_VARIANTS,
    hooks::REQUIRED_APP_VARIANTS,
    jobs::REQUIRED_APP_VARIANTS,
    secrets::REQUIRED_APP_VARIANTS,
    snix::REQUIRED_APP_VARIANTS,
    sql::REQUIRED_APP_VARIANTS,
];

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
