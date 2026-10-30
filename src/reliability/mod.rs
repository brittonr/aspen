//! Product-owned bounded black-box service-registry reliability adapter.
//! Live observations are supporting evidence only; production remains unadmitted.

pub mod authority;
#[path = "import/checker.rs"]
pub mod checker;
#[path = "public/client.rs"]
pub mod client;
#[path = "import/comparison.rs"]
pub mod comparison;
#[path = "import/evidence.rs"]
pub mod evidence;
#[path = "import/evidenceproof.rs"]
pub mod evidenceproof;
#[path = "model/fault.rs"]
pub mod fault;
#[path = "model/generator.rs"]
pub mod generator;
#[path = "model/history.rs"]
pub mod history;
#[path = "model/profile.rs"]
pub mod profile;
#[path = "public/protocol.rs"]
pub mod protocol;
pub mod readiness;
pub mod service;

#[cfg(test)]
#[path = "public/clienttests.rs"]
mod clienttests;

#[cfg(test)]
#[path = "public/publicfixtures.rs"]
mod publicfixtures;
