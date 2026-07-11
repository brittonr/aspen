use super::MAX_FABRIC_COLLECTION_ITEMS;
use super::has_duplicates;

const REQUIRED_SYSTEM_EXTENSION_EVIDENCE_COUNT: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExtensionTier {
    SandboxedPlugin,
    SystemExtension,
    ApplicationWorkload,
}

impl ExtensionTier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SandboxedPlugin => "sandboxed-plugin",
            Self::SystemExtension => "system-extension",
            Self::ApplicationWorkload => "application-workload",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FabricAuthority {
    PluginHostcall,
    ApplicationServiceUse,
    ProtocolOwnership,
    Transport,
    DurableState,
    Time,
    Scheduling,
    Membership,
    Placement,
    Consistency,
    Supervision,
    Policy,
    Resources,
    Simulation,
    Evidence,
}

impl FabricAuthority {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PluginHostcall => "plugin-hostcall",
            Self::ApplicationServiceUse => "application-service-use",
            Self::ProtocolOwnership => "protocol-ownership",
            Self::Transport => "transport",
            Self::DurableState => "durable-state",
            Self::Time => "time",
            Self::Scheduling => "scheduling",
            Self::Membership => "membership",
            Self::Placement => "placement",
            Self::Consistency => "consistency",
            Self::Supervision => "supervision",
            Self::Policy => "policy",
            Self::Resources => "resources",
            Self::Simulation => "simulation",
            Self::Evidence => "evidence",
        }
    }

    pub const fn requires_system_extension(self) -> bool {
        !matches!(self, Self::PluginHostcall | Self::ApplicationServiceUse)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TierAdmissionEvidence {
    SystemExtensionManifest,
    PolicyPass,
    ProvenancePass,
    ExplicitPortBindings,
    ResourceGrants,
    LifecycleAdmission,
}

impl TierAdmissionEvidence {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SystemExtensionManifest => "system-extension-manifest",
            Self::PolicyPass => "policy-pass",
            Self::ProvenancePass => "provenance-pass",
            Self::ExplicitPortBindings => "explicit-port-bindings",
            Self::ResourceGrants => "resource-grants",
            Self::LifecycleAdmission => "lifecycle-admission",
        }
    }
}

pub const REQUIRED_SYSTEM_EXTENSION_EVIDENCE: [TierAdmissionEvidence; REQUIRED_SYSTEM_EXTENSION_EVIDENCE_COUNT] = [
    TierAdmissionEvidence::SystemExtensionManifest,
    TierAdmissionEvidence::PolicyPass,
    TierAdmissionEvidence::ProvenancePass,
    TierAdmissionEvidence::ExplicitPortBindings,
    TierAdmissionEvidence::ResourceGrants,
    TierAdmissionEvidence::LifecycleAdmission,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionTierRequest {
    pub tier: ExtensionTier,
    pub requested_authorities: Vec<FabricAuthority>,
    pub admission_evidence: Vec<TierAdmissionEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionTierAdmission {
    pub tier: ExtensionTier,
    pub admitted_authorities: Vec<FabricAuthority>,
    pub supporting_evidence: Vec<TierAdmissionEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionTierIssue {
    TooManyAuthorities {
        actual: usize,
        maximum: usize,
    },
    DuplicateAuthority,
    TooManyEvidenceEntries {
        actual: usize,
        maximum: usize,
    },
    DuplicateEvidence,
    AuthorityRequiresSystemExtension {
        tier: ExtensionTier,
        authority: FabricAuthority,
    },
    PluginAuthorityOutsidePluginTier(ExtensionTier),
    ApplicationAuthorityOutsideApplicationTier(ExtensionTier),
    MissingSystemExtensionEvidence(TierAdmissionEvidence),
}

// r[impl molten.fabric_boundary.extension_tiers]
pub fn validate_extension_tier(
    request: &ExtensionTierRequest,
) -> Result<ExtensionTierAdmission, Vec<ExtensionTierIssue>> {
    let mut issues = Vec::new();
    validate_tier_bounds(request, &mut issues);
    validate_authority_tier(request, &mut issues);
    validate_system_extension_evidence(request, &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut admitted_authorities = request.requested_authorities.clone();
    admitted_authorities.sort();
    let mut supporting_evidence = request.admission_evidence.clone();
    supporting_evidence.sort();
    Ok(ExtensionTierAdmission {
        tier: request.tier,
        admitted_authorities,
        supporting_evidence,
    })
}

fn validate_tier_bounds(request: &ExtensionTierRequest, issues: &mut Vec<ExtensionTierIssue>) {
    if request.requested_authorities.len() > MAX_FABRIC_COLLECTION_ITEMS {
        issues.push(ExtensionTierIssue::TooManyAuthorities {
            actual: request.requested_authorities.len(),
            maximum: MAX_FABRIC_COLLECTION_ITEMS,
        });
    }
    if has_duplicates(&request.requested_authorities) {
        issues.push(ExtensionTierIssue::DuplicateAuthority);
    }
    if request.admission_evidence.len() > MAX_FABRIC_COLLECTION_ITEMS {
        issues.push(ExtensionTierIssue::TooManyEvidenceEntries {
            actual: request.admission_evidence.len(),
            maximum: MAX_FABRIC_COLLECTION_ITEMS,
        });
    }
    if has_duplicates(&request.admission_evidence) {
        issues.push(ExtensionTierIssue::DuplicateEvidence);
    }
}

fn validate_authority_tier(request: &ExtensionTierRequest, issues: &mut Vec<ExtensionTierIssue>) {
    for authority in &request.requested_authorities {
        if authority.requires_system_extension() && request.tier != ExtensionTier::SystemExtension {
            issues.push(ExtensionTierIssue::AuthorityRequiresSystemExtension {
                tier: request.tier,
                authority: *authority,
            });
        }
        if *authority == FabricAuthority::PluginHostcall && request.tier != ExtensionTier::SandboxedPlugin {
            issues.push(ExtensionTierIssue::PluginAuthorityOutsidePluginTier(request.tier));
        }
        if *authority == FabricAuthority::ApplicationServiceUse && request.tier != ExtensionTier::ApplicationWorkload {
            issues.push(ExtensionTierIssue::ApplicationAuthorityOutsideApplicationTier(request.tier));
        }
    }
}

fn validate_system_extension_evidence(request: &ExtensionTierRequest, issues: &mut Vec<ExtensionTierIssue>) {
    if request.tier != ExtensionTier::SystemExtension {
        return;
    }
    for required in REQUIRED_SYSTEM_EXTENSION_EVIDENCE {
        if !request.admission_evidence.contains(&required) {
            issues.push(ExtensionTierIssue::MissingSystemExtensionEvidence(required));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_system_extension_request() -> ExtensionTierRequest {
        ExtensionTierRequest {
            tier: ExtensionTier::SystemExtension,
            requested_authorities: vec![FabricAuthority::ProtocolOwnership, FabricAuthority::DurableState],
            admission_evidence: REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
        }
    }

    // r[verify molten.fabric_boundary.extension_tiers]
    #[test]
    fn reviewed_system_extension_receives_only_declared_authority() {
        let admission = validate_extension_tier(&valid_system_extension_request()).expect("reviewed extension passes");

        assert_eq!(admission.tier, ExtensionTier::SystemExtension);
        assert_eq!(admission.admitted_authorities, vec![
            FabricAuthority::ProtocolOwnership,
            FabricAuthority::DurableState
        ]);
        assert_eq!(admission.supporting_evidence, REQUIRED_SYSTEM_EXTENSION_EVIDENCE);
    }

    // r[verify molten.fabric_boundary.extension_tiers]
    #[test]
    fn sandboxed_plugin_cannot_gain_system_authority_from_operation_shape() {
        let request = ExtensionTierRequest {
            tier: ExtensionTier::SandboxedPlugin,
            requested_authorities: vec![FabricAuthority::DurableState, FabricAuthority::ProtocolOwnership],
            admission_evidence: Vec::new(),
        };

        let issues = validate_extension_tier(&request).expect_err("plugin system authority must deny");

        assert!(issues.contains(&ExtensionTierIssue::AuthorityRequiresSystemExtension {
            tier: ExtensionTier::SandboxedPlugin,
            authority: FabricAuthority::DurableState,
        }));
        assert!(issues.contains(&ExtensionTierIssue::AuthorityRequiresSystemExtension {
            tier: ExtensionTier::SandboxedPlugin,
            authority: FabricAuthority::ProtocolOwnership,
        }));
    }

    // r[verify molten.fabric_boundary.extension_tiers]
    #[test]
    fn system_extension_denies_when_lifecycle_evidence_is_missing() {
        let mut request = valid_system_extension_request();
        request.admission_evidence.retain(|evidence| *evidence != TierAdmissionEvidence::LifecycleAdmission);

        let issues = validate_extension_tier(&request).expect_err("missing lifecycle admission must deny");

        assert_eq!(issues, vec![ExtensionTierIssue::MissingSystemExtensionEvidence(
            TierAdmissionEvidence::LifecycleAdmission
        )]);
    }
}
