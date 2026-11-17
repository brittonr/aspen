// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit manifest installation for the standalone native pure profile.
mod host;
mod limits;
mod runtime;

pub use host::pure_net_host_bindings;
pub use limits::admit_process_limits;
pub use limits::observe_process_limits;
pub use limits::ObservedProcessLimits;
use molten_batch_application::Content;
use molten_batch_application::Error;
use molten_batch_core::*;
use molten_pure_net as net;
pub use runtime::PureNetRuntime;

use crate::CapabilityRoot;

const INSTALLED_MANIFEST: &str = "pure-net-manifest.preserves-frame";
const MAX_MANIFEST_BYTES: usize = 32768;

pub struct InstalledPureNet {
    pub manifest_ref: Id,
    pub manifest: net::Manifest,
    pub host: net::HostBindings,
    pub content: Content,
}

fn admit(policy: &Policy, bytes: Vec<u8>) -> Result<InstalledPureNet, Error> {
    validate_policy(policy)?;
    if policy.profile != PURE_NET_PROFILE || bytes.len() > MAX_MANIFEST_BYTES {
        return Err(Error::Denied);
    }
    let manifest = net::decode_manifest(&bytes).map_err(crate::invalid)?;
    let host = pure_net_host_bindings()?;
    if manifest.bindings.host != host {
        return Err(Error::Denied);
    }
    let content = crate::content(ContentKind::PureNetManifest, bytes)?;
    let manifest_ref = crate::content_id(&content)?;
    if !policy.allowed_pure_net_manifests.contains(&manifest_ref) {
        return Err(Error::Denied);
    }
    Ok(InstalledPureNet {
        manifest_ref,
        manifest,
        host,
        content,
    })
}

// r[impl molten.pure_net.profile]
// r[impl molten.pure_net.identity]
pub fn install_pure_net(root: &CapabilityRoot, bytes: Vec<u8>) -> Result<Id, Error> {
    let config: crate::NodeConfig = root.read_json("config.json")?;
    config.validate()?;
    if config.role == crate::Role::Client {
        return Err(Error::Denied);
    }
    let policy: Policy = root.read_json("policy.json")?;
    if config.cluster != policy.cluster || config.generation != policy.generation {
        return Err(Error::Denied);
    }
    let installed = admit(&policy, bytes)?;
    // One create-only durable publication. Partial/changed installation fails
    // startup rather than silently replacing a selected workload or grant.
    root.create_new(INSTALLED_MANIFEST, &installed.content.bytes)?;
    Ok(installed.manifest_ref)
}

pub(crate) fn load_pure_net(root: &CapabilityRoot, policy: &Policy) -> Result<InstalledPureNet, Error> {
    admit(policy, root.read(INSTALLED_MANIFEST, MAX_MANIFEST_BYTES)?)
}
