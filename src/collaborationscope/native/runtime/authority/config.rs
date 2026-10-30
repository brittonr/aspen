/// Parsed operator authority declaration, not yet bound to node roots.
pub(crate) struct Unbound;

pub(crate) struct Bound {
    root: crate::node_state::DirectoryRoot,
    profile: crate::fabric_time::AdmittedTimeProfile,
    replay: std::rc::Rc<crate::collaboration_scope::NonceLedger>,
}

pub(crate) fn parse(value: &preserves::IOValue) -> crate::error::Result<Unbound> {
    let f = crate::collaboration_scope::decode::fields(value, "collaboration-native-authority-v1", 1)?;
    if crate::collaboration_scope::decode::string(&f[0])? != "operator-scope-authority" {
        return Err(crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Authority));
    }
    Ok(Unbound)
}

impl Unbound {
    pub(crate) fn bind(
        self,
        root: &crate::node_state::DirectoryRoot,
        profile: &crate::fabric_time::AdmittedTimeProfile,
        store: crate::collaboration_scope::Database,
    ) -> crate::error::Result<Bound> {
        let bound = Bound {
            root: root.clone(),
            profile: profile.clone(),
            replay: std::rc::Rc::new(crate::collaboration_scope::NonceLedger::new(store)),
        };
        bound.trust()?;
        Ok(bound)
    }
}

impl Bound {
    pub(super) fn root(&self) -> &crate::node_state::DirectoryRoot {
        &self.root
    }
    pub(super) fn time_profile(&self) -> &crate::fabric_time::AdmittedTimeProfile {
        &self.profile
    }
    pub(super) fn replay(&self) -> std::rc::Rc<crate::collaboration_scope::NonceLedger> {
        self.replay.clone()
    }
    pub(super) fn reserve_probe(
        &self,
        intent: &str,
        credential: &Credential,
    ) -> crate::error::Result<crate::collaboration_scope::RequestRef> {
        let value = crate::preserves_rail::record("collaboration-fresh-probe-v1", vec![
            crate::preserves_rail::string(intent),
            crate::preserves_rail::string(crate::preserves_rail::content_ref_from_bytes(credential.nonce.as_bytes())),
        ]);
        let hash = crate::preserves_rail::canonical_hash(&value)?;
        let key = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
            "collaboration-probe-reservation-v1",
            vec![crate::preserves_rail::string(&hash)],
        ))?;
        self.replay
            .reserve(&key, &hash)
            .map_err(|_| crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Replay))?;
        crate::collaboration_scope::RequestRef::new(hash)
            .map_err(|_| crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Domain))
    }
    pub(super) fn trust(&self) -> crate::error::Result<Trust> {
        let value = read(self.root(), "operator-scope-authority/current.preserves")?;
        let f = crate::collaboration_scope::decode::fields(&value, "collaboration-operator-trust-v1", 8)?;
        let issuer = crate::collaboration_scope::decode::string(&f[0])?;
        let holder = crate::collaboration_scope::decode::string(&f[2])?;
        let keys = vec![key(&issuer, &f[1])?, key(&holder, &f[3])?];
        let policies = crate::collaboration_scope::decode::items(&f[4], 6)?
            .iter()
            .map(crate::collaboration_scope::decode::string)
            .collect::<crate::error::Result<Vec<_>>>()?;
        if policies.is_empty() {
            return Err(crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Authority));
        }
        let revoked = crate::collaboration_scope::decode::items(&f[5], crate::collaboration_scope::MAX_OPERATIONS)?
            .iter()
            .map(|v| {
                let token = basalt::ucan::CompactToken::new(&crate::collaboration_scope::decode::string(v)?)
                    .map_err(|_| crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Authority))?;
                Ok(basalt::ucan::proof_reference(&token))
            })
            .collect::<crate::error::Result<Vec<_>>>()?;
        Ok(Trust {
            reference: molten_core::nominal::RevocationRef::new(crate::preserves_rail::canonical_hash(&value)?)
                .map_err(|_| crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Domain))?,
            issuer,
            holder,
            keys: basalt::ucan::KeyResolutionContext::new(keys),
            policies,
            revocations: basalt::ucan::RevocationList::from_references(revoked),
            position: crate::collaboration_scope::decode::integer(&f[6])?,
            unix_seconds: crate::collaboration_scope::decode::integer(&f[7])?,
        })
    }
    pub(super) fn credential(&self, effect: &str) -> crate::error::Result<Credential> {
        crate::preserves_rail::validate_content_ref(effect)?;
        let digest = effect
            .strip_prefix("blake3:")
            .ok_or_else(|| crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Domain))?;
        let value = read(self.root(), &format!("operator-scope-authority/credentials/{digest}.preserves"))?;
        let f = crate::collaboration_scope::decode::fields(&value, "collaboration-operator-credential-v1", 5)?;
        if crate::collaboration_scope::decode::string(&f[0])? != effect {
            return Err(crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Authority));
        }
        Ok(Credential {
            read: crate::collaboration_scope::RequestRef::new(crate::collaboration_scope::decode::string(&f[1])?)
                .map_err(|_| crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Domain))?,
            token: crate::collaboration_scope::decode::string(&f[2])?,
            nonce: basalt::ucan::HolderInvocationNonce::from_bytes(hex(&crate::collaboration_scope::decode::string(
                &f[3],
            )?)?),
            signature: basalt::ucan::SignerSignature::from_bytes(hex(&crate::collaboration_scope::decode::string(
                &f[4],
            )?)?),
        })
    }
}

pub(super) struct Trust {
    pub reference: molten_core::nominal::RevocationRef,
    pub issuer: String,
    pub holder: String,
    pub keys: basalt::ucan::KeyResolutionContext,
    pub policies: Vec<String>,
    pub revocations: basalt::ucan::RevocationList,
    pub position: u64,
    pub unix_seconds: u64,
}
pub(super) struct Credential {
    pub read: crate::collaboration_scope::RequestRef,
    pub token: String,
    pub nonce: basalt::ucan::HolderInvocationNonce,
    pub signature: basalt::ucan::SignerSignature,
}
fn read(root: &crate::node_state::DirectoryRoot, path: &str) -> crate::error::Result<preserves::IOValue> {
    let path = crate::node_state::RelativePath::parse(path)?;
    crate::preserves_rail::parse_canonical_bytes(&root.read(&path, crate::collaboration_scope::MAX_RECORD_BYTES)?)
}
fn key(did: &str, value: &preserves::IOValue) -> crate::error::Result<basalt::ucan::Ed25519VerificationKey> {
    let did = basalt::ucan::IssuerDid::new(did)
        .map_err(|_| crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Authority))?;
    basalt::ucan::Ed25519VerificationKey::new(
        did,
        basalt::ucan::Ed25519PublicKey::from_bytes(hex(&crate::collaboration_scope::decode::string(value)?)?),
    )
    .map_err(|_| crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Authority))
}
fn hex<const N: usize>(value: &str) -> crate::error::Result<[u8; N]> {
    if value.len() != N * 2 {
        return Err(crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Domain));
    }
    let mut bytes = [0; N];
    for (out, pair) in bytes.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
        let digit = |v: u8| match v {
            b'0'..=b'9' => Some(v - b'0'),
            b'a'..=b'f' => Some(v - b'a' + 10),
            _ => None,
        };
        *out = digit(pair[0])
            .zip(digit(pair[1]))
            .map(|(a, b)| a * 16 + b)
            .ok_or_else(|| crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Domain))?;
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_cannot_select_credential_or_ambient_roots() {
        for path in [
            "../operator-scope-authority",
            "/etc/credentials",
            "source-artifact",
            "operator-scope-authority/credentials",
        ] {
            let value = crate::preserves_rail::record("collaboration-native-authority-v1", vec![
                crate::preserves_rail::string(path),
            ]);
            assert!(parse(&value).is_err());
        }
    }

    #[test]
    fn independent_issuer_key_must_match_actual_did() {
        let issuer = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([71; 32]);
        let holder = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([73; 32]);
        let did = basalt::ucan::TokenSigner::issuer(&issuer).expect("issuer");
        let other = basalt::ucan::TokenSigner::issuer(&holder).expect("holder");
        assert_ne!(did, other);
        // All-zero bytes are a different key from the independently rooted DID.
        assert!(key(did.as_str(), &crate::preserves_rail::string("00".repeat(32))).is_err());
    }

    #[test]
    fn signature_and_nonce_bytes_reject_noncanonical_or_wrong_length_inputs() {
        assert!(hex::<64>(&"00".repeat(63)).is_err());
        assert!(hex::<32>(&"AA".repeat(32)).is_err());
        assert!(hex::<32>(&"0g".repeat(32)).is_err());
        assert_eq!(hex::<32>(&"a5".repeat(32)).expect("canonical bytes"), [0xa5; 32]);
    }
}
