//! Owned scope-admission faults after genuine route, scope, and exact host authoring.

pub(in crate::driver::scenarios) enum Mode {
    Missing,
    Malformed,
    Stale,
}

impl Mode {
    pub(super) fn label(&self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Malformed => "malformed",
            Self::Stale => "stale",
        }
    }

    /// Returns only the actual stored current trust reference, never authority material.
    pub(super) fn apply(
        self,
        delivery: &crate::driver::flow::Delivery<'_>,
        request: &molten::node_service_hosting::OperationRequest,
    ) -> crate::producer::Result<String> {
        let root = &delivery.fixture.root;
        let before = current_reference(root)?;
        if before != delivery.intent.observation.currentness.revocation.as_str() {
            return Err("current trust moved before the declared scope fault".into());
        }
        match self {
            Self::Missing => {
                root.remove_regular_file(&credential_path(delivery)?)?;
                let after = current_reference(root)?;
                if after != before {
                    return Err("missing scope credential changed current scope trust".into());
                }
                Ok(after)
            }
            Self::Malformed => {
                // Valid canonical bytes with an intentionally invalid credential schema.
                root.write(
                    &credential_path(delivery)?,
                    &molten::preserves_rail::canonical_bytes(&molten::preserves_rail::record(
                        "invalid-scope-credential-v1",
                        vec![],
                    ))?,
                )?;
                let after = current_reference(root)?;
                if after != before {
                    return Err("malformed scope credential changed current scope trust".into());
                }
                Ok(after)
            }
            Self::Stale => {
                let reference =
                    molten::node_service_host::authority::request_ref(&delivery.fixture.bundle_ref, request)?;
                let path = molten::node_service_host::authority::credential_path(&reference)?;
                let credential =
                    molten::node_service_host::authority::wire::decode_credential(&root.read(&path, 1_048_576)?)?;
                let observation = &delivery.intent.observation;
                // Revoke the real HOST token in SCOPE trust only. Its distinct resource
                // leaves the genuine scope grant valid; only current root identity moves.
                delivery.node.issuer.provision_scope(root, &crate::authority::scope::TrustInput {
                    policies: &[crate::driver::setup::SCOPE_POLICY],
                    revoked_tokens: &[credential.token.as_str()],
                    logical_position: observation.currentness.logical_now,
                    unix_seconds: observation.unix_seconds,
                })?;
                let after = current_reference(root)?;
                if after == before {
                    return Err("stale scope fault did not rotate the current trust reference".into());
                }
                Ok(after)
            }
        }
    }
}

fn credential_path(
    delivery: &crate::driver::flow::Delivery<'_>,
) -> crate::producer::Result<molten::node_state::RelativePath> {
    molten::preserves_rail::validate_content_ref(&delivery.intent.prepared.effect_ref)?;
    let digest = delivery.intent.prepared.effect_ref.strip_prefix("blake3:").ok_or("effect digest")?;
    Ok(molten::node_state::RelativePath::parse(&format!(
        "operator-scope-authority/credentials/{digest}.preserves",
    ))?)
}

fn current_reference(root: &molten::node_state::DirectoryRoot) -> crate::producer::Result<String> {
    let path = molten::node_state::RelativePath::parse("operator-scope-authority/current.preserves")?;
    let value = molten::preserves_rail::parse_canonical_bytes(&root.read(&path, 1_048_576)?)?;
    Ok(molten::preserves_rail::canonical_hash(&value)?)
}
