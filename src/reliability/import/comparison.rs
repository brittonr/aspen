//! Comparison retains environmental scopes; it never merges evidence classes.

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize
)]
#[serde(rename_all = "kebab-case")]
pub enum Environment {
    Simulation,
    NixosVm,
    ChaoscontrolKvm,
    OnixosLive,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileEvidence {
    pub environment: Environment,
    pub receipt_ref: String,
    pub corpus_ref: String,
    pub invariant_ref: String,
    pub adapter_ref: String,
    pub fault_mechanism: String,
    pub time_source: String,
    pub checker_ref: String,
    pub verdict: String,
    pub non_claims: std::collections::BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub profiles: Vec<ProfileEvidence>,
    pub scope: String,
    pub authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issue {
    Bounds,
    Identity,
    Corpus,
    DuplicateEnvironment,
    Scope,
    Claim,
}

// r[impl molten.testing.live_reliability_claim_boundary]
pub fn compare(profiles: &[ProfileEvidence]) -> Result<Report, Issue> {
    if profiles.is_empty() || profiles.len() > 4 {
        return Err(Issue::Bounds);
    }
    let mut environments = std::collections::BTreeSet::new();
    let required: std::collections::BTreeSet<_> =
        crate::live_reliability::evidence::NON_CLAIMS.iter().map(|v| (*v).to_owned()).collect();
    for profile in profiles {
        if !environments.insert(profile.environment) {
            return Err(Issue::DuplicateEnvironment);
        }
        if [
            &profile.receipt_ref,
            &profile.corpus_ref,
            &profile.invariant_ref,
            &profile.adapter_ref,
            &profile.checker_ref,
        ]
        .iter()
        .any(|r| !crate::live_reliability::checker::reference(r))
        {
            return Err(Issue::Identity);
        }
        if profile.corpus_ref != profiles[0].corpus_ref || profile.invariant_ref != profiles[0].invariant_ref {
            return Err(Issue::Corpus);
        }
        if profile.fault_mechanism.is_empty()
            || profile.fault_mechanism.len() > 256
            || profile.time_source.is_empty()
            || profile.time_source.len() > 256
            || !matches!(profile.verdict.as_str(), "valid" | "invalid" | "unknown")
        {
            return Err(Issue::Scope);
        }
        if profile.non_claims != required {
            return Err(Issue::Claim);
        }
    }
    let mut profiles = profiles.to_vec();
    profiles.sort_by_key(|p| p.environment);
    Ok(Report {
        profiles,
        scope: "separate finite histories; no profile substitutes for another".into(),
        authority: false,
    })
}

pub fn admit_claim(role: &str) -> Result<(), Issue> {
    if role == "external-finite-live-observation" {
        Ok(())
    } else {
        Err(Issue::Claim)
    }
}
