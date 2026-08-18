//! Aspen adapter pilot for product-neutral schema identity.
//!
//! Aspen retains Preserves DTOs, registries, policy, storage, receipts, and
//! migration execution. This adapter supplies explicit product facts to the
//! shared pure core.

const TEXT_LIMIT_BYTES: u16 = 64;
const MAXIMUM_NODES: u32 = 16;
const MAXIMUM_MEMBERS: u32 = 32;
const ROOT_NODE: schema_identity_core::NodeId = schema_identity_core::NodeId::new(1);
const VALUE_NODE: schema_identity_core::NodeId = schema_identity_core::NodeId::new(2);

/// Aspen facts selected for one bounded nominal profile descriptor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MoltenProfileSchema<'a> {
    /// Product owner domain for the durable lineage.
    pub owner: &'a str,
    /// Product-local durable lineage key.
    pub local_key: &'a str,
    /// Stable member identity, separate from display names.
    pub stable_member_id: &'a str,
}

/// Admits one bounded Aspen profile through the shared pure core.
///
/// # Errors
///
/// Returns an error when text, limits, graph shape, member identity, or framing
/// is invalid.
pub fn admit_profile(
    input: MoltenProfileSchema<'_>,
) -> Result<schema_identity_core::AdmittedSchema, schema_identity_core::SchemaIdentityError> {
    let text_limit = schema_identity_core::TextLimit::new(TEXT_LIMIT_BYTES)?;
    let lineage = schema_identity_core::NominalScope::new(input.owner, input.local_key, text_limit)?.lineage_id()?;
    let descriptor = schema_identity_core::SchemaDescriptor::new(
        schema_identity_core::IdentityMode::Nominal { lineage },
        schema_identity_core::Openness::Closed,
        ROOT_NODE,
        vec![
            schema_identity_core::SchemaNode::new(ROOT_NODE, schema_identity_core::SchemaNodeKind::Record {
                members: vec![schema_identity_core::RecordMember::new(
                    schema_identity_core::MemberId::new(input.stable_member_id, text_limit)?,
                    VALUE_NODE,
                    true,
                    None,
                )],
            }),
            schema_identity_core::SchemaNode::new(
                VALUE_NODE,
                schema_identity_core::SchemaNodeKind::Scalar(schema_identity_core::ScalarKind::Text),
            ),
        ],
        None,
    );
    schema_identity_core::admit_schema(
        descriptor,
        schema_identity_core::AdmissionLimits::new(
            schema_identity_core::NodeLimit::new(MAXIMUM_NODES)?,
            schema_identity_core::MemberLimit::new(MAXIMUM_MEMBERS)?,
        ),
    )
}

#[cfg(test)]
mod tests {
    const EXPECTED_FIXTURE_SCHEMA: &str = "faa587362daee612565efb5632f9209997946737aefd657d9f801f316dce9c88";

    #[test]
    fn aspen_adapter_matches_the_published_nominal_vector() -> Result<(), Box<dyn std::error::Error>> {
        let admitted = super::admit_profile(super::MoltenProfileSchema {
            owner: "fixture-owner",
            local_key: "expected",
            stable_member_id: "field:value",
        })?;
        assert_eq!(admitted.schema_id().to_hex(), EXPECTED_FIXTURE_SCHEMA);
        schema_identity_conformance::verify_fixture_json(schema_identity_conformance::CHECKED_VECTORS_JSON)?;
        Ok(())
    }

    #[test]
    fn crossed_owner_changes_the_nominal_identity() -> Result<(), schema_identity_core::SchemaIdentityError> {
        let first = super::admit_profile(super::MoltenProfileSchema {
            owner: "owner:a",
            local_key: "profile",
            stable_member_id: "field:value",
        })?;
        let crossed = super::admit_profile(super::MoltenProfileSchema {
            owner: "owner:b",
            local_key: "profile",
            stable_member_id: "field:value",
        })?;
        assert_ne!(first.schema_id(), crossed.schema_id());
        Ok(())
    }
}
