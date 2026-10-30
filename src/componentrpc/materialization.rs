//! No compiler/composer/virtualizer is reachable here. Reuse the published verifier.
use cap_fs_ext::DirExt;

use crate::wasm_component;

pub struct Bundle {
    client: wasm_component::VerifiedBundle,
    server: wasm_component::VerifiedBundle,
    reference: String,
    wit: String,
}
struct Input<'a> {
    pub root: &'a cap_std::fs::Dir,
    pub staging: &'a cap_std::fs::Dir,
    pub client_json: &'a [u8],
    pub server_json: &'a [u8],
    pub client_profile: &'a wasm_component::RuntimeProfile,
    pub server_profile: &'a wasm_component::RuntimeProfile,
}

// r[impl molten.component_rpc.materialization]
impl Bundle {
    pub fn load_package(package: &cap_std::fs::Dir, staging: &cap_std::fs::Dir) -> super::Result<Self> {
        let bytes = super::package::read(package, "manifest.json")?;
        let manifest: serde_json::Value = serde_json::from_slice(&bytes).map_err(super::failure)?;
        super::package::validate(&manifest)?;
        let (client_json, client_profile) = super::package::metadata(package, &manifest, "client")?;
        let (server_json, server_profile) = super::package::metadata(package, &manifest, "server")?;
        let root = package.open_dir_nofollow("capability").map_err(super::failure)?;
        Self::load(Input {
            root: &root,
            staging,
            client_json: &client_json,
            server_json: &server_json,
            client_profile: &client_profile,
            server_profile: &server_profile,
        })
    }

    /// Both bundle DTOs and all referenced members must belong to the authorized root.
    fn load(input: Input<'_>) -> super::Result<Self> {
        let client = wasm_component::VerifiedBundle::load(
            input.root,
            input.staging,
            "rpc-client-snapshot",
            input.client_json,
            input.client_profile,
        )
        .map_err(super::failure)?;
        let server = wasm_component::VerifiedBundle::load(
            input.root,
            input.staging,
            "rpc-server-snapshot",
            input.server_json,
            input.server_profile,
        )
        .map_err(super::failure)?;
        let reference = super::hash(&crate::codec::record("component-rpc-materialization-v1", vec![
            crate::codec::string(super::REVISION),
            crate::codec::string(client.bundle_ref()),
            crate::codec::string(server.bundle_ref()),
        ]))?;
        let wit = super::hash(&crate::codec::record("component-rpc-wit-pair-v1", vec![
            crate::codec::string(super::reference(client.wit_bytes())),
            crate::codec::string(super::reference(server.wit_bytes())),
        ]))?;
        Ok(Self {
            client,
            server,
            reference,
            wit,
        })
    }
    pub fn reference(&self) -> &str {
        &self.reference
    }
    pub fn wit(&self) -> &str {
        &self.wit
    }
    pub fn client(&self) -> &[u8] {
        self.client.component_bytes()
    }
    pub fn server(&self) -> &[u8] {
        self.server.component_bytes()
    }
    pub fn check(&self, scope: &super::bridge::Scope) -> super::Result<()> {
        if scope.bundle != self.reference
            || scope.wit != self.wit
            || scope.caller != super::reference(self.client())
            || scope.callee != super::reference(self.server())
        {
            return Err("invocation differs from measured Mantle client/server/WIT bundle".into());
        }
        Ok(())
    }
}
