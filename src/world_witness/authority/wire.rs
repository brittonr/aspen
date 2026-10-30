#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trust {
    pub issuer: String,
    pub issuer_key: [u8; 32],
    pub holder: String,
    pub holder_key: [u8; 32],
    pub policy: String,
    pub head_policy_ref: String,
    pub witness_policy_ref: String,
    pub not_before: u64,
    pub expires: u64,
    pub authority_generation: u64,
    pub revoked: Vec<[u8; 32]>,
    pub producer_id: String,
    pub key_id: String,
    pub artifact_public_key: [u8; 32],
    pub key_generation: u64,
    pub role: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credential {
    pub token: String,
    pub nonce: [u8; 32],
    pub signature: Vec<u8>,
    pub proofs: Vec<String>,
    pub artifact_signature: Vec<u8>,
}

impl Trust {
    pub fn identity(&self) -> crate::error::Result<String> {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("world-head-operator-trust-v1", vec![
            crate::preserves_rail::string(&self.issuer),
            preserves::IOValue::bytes(self.issuer_key),
            crate::preserves_rail::string(&self.holder),
            preserves::IOValue::bytes(self.holder_key),
            crate::preserves_rail::string(&self.policy),
            crate::preserves_rail::string(&self.head_policy_ref),
            crate::preserves_rail::string(&self.witness_policy_ref),
            crate::preserves_rail::u64_value(self.not_before),
            crate::preserves_rail::u64_value(self.expires),
            crate::preserves_rail::u64_value(self.authority_generation),
            crate::preserves_rail::sequence(
                self.revoked.iter().map(|reference| preserves::IOValue::bytes(*reference)).collect(),
            ),
            crate::preserves_rail::string(&self.producer_id),
            crate::preserves_rail::string(&self.key_id),
            preserves::IOValue::bytes(self.artifact_public_key),
            crate::preserves_rail::u64_value(self.key_generation),
            crate::preserves_rail::string(&self.role),
        ]))
    }
}

pub fn trust_path() -> crate::error::Result<crate::node_state::RelativePath> {
    crate::node_state::RelativePath::parse("world-head-trust.json")
}

pub fn credential_path(claim_ref: &str) -> crate::error::Result<crate::node_state::RelativePath> {
    crate::preserves_rail::ContentRef::parse(claim_ref)?;
    crate::node_state::RelativePath::parse(&format!("world-head-credential-{}.json", &claim_ref[7..]))
}

pub fn install_trust(root: &crate::node_state::DirectoryRoot, trust: &Trust) -> crate::error::Result<()> {
    install(root, &trust_path()?, trust)
}

pub fn install_credential(
    root: &crate::node_state::DirectoryRoot,
    claim_ref: &str,
    credential: &Credential,
) -> crate::error::Result<()> {
    install(root, &credential_path(claim_ref)?, credential)
}

fn install<T: serde::Serialize>(
    root: &crate::node_state::DirectoryRoot,
    path: &crate::node_state::RelativePath,
    value: &T,
) -> crate::error::Result<()> {
    let bytes = serde_json::to_vec(value).map_err(|_| super::denied())?;
    if bytes.len() > 262_144 {
        return Err(super::denied());
    }
    root.write_new(path, &bytes)
}

pub(super) fn load<T: serde::de::DeserializeOwned>(
    root: &crate::node_state::DirectoryRoot,
    path: &crate::node_state::RelativePath,
) -> crate::error::Result<T> {
    let bytes = root.read(path, 262_144)?;
    serde_json::from_slice(&bytes).map_err(|_| super::denied())
}
