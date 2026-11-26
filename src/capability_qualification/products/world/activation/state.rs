//! The durable branch-authority state of one qualification node: its policy
//! document, revocation ledger, signed branch token and activation records.

use ed25519_dalek::Signer;
use ed25519_dalek::Verifier;

/// The branch-authority runtime of one qualification node.
pub(super) struct BranchRuntime {
    pub(super) root: crate::node_state::DirectoryRoot,
    pub(super) issuer: ed25519_dalek::SigningKey,
    pub(super) token: crate::capability_tokens::CapabilityToken,
    pub(super) signature: ed25519_dalek::Signature,
    pub(super) tick: u64,
    pub(super) observations: Vec<molten_core::world_branch_authority::CurrentAuthorityFacts>,
    pub(super) admissions: Vec<preserves::IOValue>,
    pub(super) realized: std::collections::BTreeMap<String, String>,
    pub(super) activation_calls: usize,
    pub(super) receipts: Vec<preserves::IOValue>,
}

impl BranchRuntime {
    pub(super) fn open(path: &std::path::Path, revocation_tick: Option<u64>) -> crate::error::Result<Self> {
        let node = super::super::fixture::NodeRoot::open(path, "branch-authority")?;
        let root = node.root;
        for directory in [
            super::GRANT_DIRECTORY,
            super::ACTIVATION_DIRECTORY,
            super::RECEIPT_DIRECTORY,
        ] {
            root.create_dir_all(&crate::node_state::RelativePath::parse(directory)?)?;
        }
        root.write(
            &crate::node_state::RelativePath::parse(super::POLICY_PATH)?,
            basalt::world_branch_authority::DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON.as_bytes(),
        )?;
        let issuer = super::super::runtime::fixture_key("world branch capability authority");
        let token = crate::capability_tokens::CapabilityToken {
            token_kind: super::TOKEN_KIND.to_string(),
            issuer_ref: super::super::runtime::key_ref(&issuer.verifying_key()),
            holder_ref: super::super::runtime::reference("aspen qualification branch operator"),
            session_ref: super::super::runtime::reference("aspen qualification branch session"),
            context_ref: super::source_branch_ref(),
            resource_ref: super::branch_authority_ref(),
            ability: super::BRANCH_ABILITY.to_string(),
            scope: super::SCOPE_RESOURCE.to_string(),
            attenuation: "exact".to_string(),
            caveats: Vec::new(),
            expires_at_tick: super::TOKEN_EXPIRY_TICK,
            revoked_refs: Vec::new(),
            policy_refs: Vec::new(),
            resource_refs: Vec::new(),
            delegation_refs: Vec::new(),
            evidence_refs: Vec::new(),
        };
        let signature = issuer
            .sign(&crate::preserves_rail::canonical_bytes(&crate::capability_tokens::capability_token_value(&token)?)?);
        let revocations = revocation_tick
            .map(|tick| {
                crate::preserves_rail::record("revocation", vec![
                    crate::preserves_rail::string(&token.issuer_ref),
                    crate::preserves_rail::u64_value(tick),
                ])
            })
            .into_iter()
            .collect();
        let ledger =
            crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(super::REVOCATION_RECORD, vec![
                crate::preserves_rail::sequence(revocations),
            ]))?;
        root.write(&crate::node_state::RelativePath::parse(super::REVOCATION_PATH)?, &ledger)?;
        Ok(Self {
            root,
            issuer,
            token,
            signature,
            tick: 0,
            observations: Vec::new(),
            admissions: Vec::new(),
            realized: std::collections::BTreeMap::new(),
            activation_calls: 0,
            receipts: Vec::new(),
        })
    }

    /// Revocations in force at `tick`, read from the current ledger.
    pub(super) fn revoked_at(&self, tick: u64) -> crate::error::Result<Vec<String>> {
        use preserves::ValueImpl as _;
        let bytes = self
            .root
            .read(&crate::node_state::RelativePath::parse(super::REVOCATION_PATH)?, super::MAX_STATE_BYTES)?;
        let value = crate::preserves_rail::parse_canonical_bytes(&bytes)?;
        let fields = crate::preserves_rail::simple_record_fields(&value, super::REVOCATION_RECORD, 1)?;
        let entries = fields[0]
            .collect_sequence()
            .ok_or_else(|| super::super::invalid("revocation ledger must be a sequence"))?;
        let mut revoked = Vec::with_capacity(entries.as_slice().len());
        for entry in entries.as_slice() {
            let entry = crate::preserves_rail::value_to_iovalue(entry);
            let pair = crate::preserves_rail::simple_record_fields(&entry, "revocation", 2)?;
            let effective = pair[1]
                .as_u64()
                .ok_or_else(|| super::super::invalid("revocation tick missing"))?
                .map_err(|error| super::super::invalid(format!("revocation tick: {error}")))?;
            if effective <= tick {
                revoked.push(crate::preserves_rail::required_string_field(&pair[0], "revoked ref")?);
            }
        }
        Ok(revoked)
    }

    pub(super) fn signature_verified(
        &self,
        token: &crate::capability_tokens::CapabilityToken,
        signature: &ed25519_dalek::Signature,
    ) -> crate::error::Result<bool> {
        let bytes = crate::preserves_rail::canonical_bytes(&crate::capability_tokens::capability_token_value(token)?)?;
        let issuer = self.issuer.verifying_key();
        Ok(token.issuer_ref == super::super::runtime::key_ref(&issuer) && issuer.verify(&bytes, signature).is_ok())
    }

    pub(super) fn activation_path(
        destination_branch_ref: &str,
    ) -> crate::error::Result<crate::node_state::RelativePath> {
        crate::node_state::RelativePath::parse(&format!(
            "{}/{}.preserves",
            super::ACTIVATION_DIRECTORY,
            crate::preserves_rail::content_ref_hex(destination_branch_ref)?
        ))
    }

    /// The durable activation record of a destination branch, if any.
    pub(super) fn activation_record(&self, destination_branch_ref: &str) -> crate::error::Result<Option<String>> {
        let path = Self::activation_path(destination_branch_ref)?;
        if !self.root.try_exists(&path)? {
            return Ok(None);
        }
        let value = crate::preserves_rail::parse_canonical_bytes(&self.root.read(&path, super::MAX_STATE_BYTES)?)?;
        let fields = crate::preserves_rail::simple_record_fields(&value, "aspen-branch-activation-v1", 3)?;
        Ok(Some(crate::preserves_rail::required_string_field(&fields[0], "activation decision")?))
    }

    pub(super) fn publish_exact(
        &self,
        path: &crate::node_state::RelativePath,
        bytes: &[u8],
    ) -> crate::error::Result<()> {
        if self.root.try_exists(path)? {
            if self.root.read(path, super::MAX_STATE_BYTES)? != bytes {
                return Err(super::super::invalid("branch-authority publication conflicts with retained bytes"));
            }
            return Ok(());
        }
        self.root.write_new(path, bytes)
    }
}
