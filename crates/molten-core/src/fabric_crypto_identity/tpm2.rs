//! Public TPM policy data only; no device, process, key bytes or TPM library.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tpm2IdentityFailure {
    InvalidPolicy,
    ProfileDenied,
    CapabilityUnavailable,
    ToolIdentityMismatch,
    ObjectMismatch,
    ProcessUnavailable,
    TimedOut,
    OutputLimit,
    UnsealDenied,
    MalformedRecord,
    HandleMismatch,
    RotationDenied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tpm2ProviderClass {
    LinuxResourceManager,
    SwtpmWorkspace,
}

impl Tpm2ProviderClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LinuxResourceManager => "linux-tpm-resource-manager",
            Self::SwtpmWorkspace => "disposable-swtpm-workspace",
        }
    }
}

/// Exact provisioned public object and independently selected provider/tool cohort.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tpm2SealedPolicy {
    pub pcrs: Vec<u8>,
    pub public_object: Vec<u8>,
    pub persistent_handle: u32,
    pub provider_class: Tpm2ProviderClass,
    pub provider_ref: String,
    pub executable_ref: String,
}

impl Tpm2SealedPolicy {
    pub fn validate(&self) -> Result<(), Tpm2IdentityFailure> {
        let p = &self.public_object;
        // TPM2B_PUBLIC: SHA256 keyed-hash, fixedTPM|fixedParent, policy-only,
        // SHA256 authPolicy, NULL keyed-hash scheme and SHA256 unique value.
        // In particular userWithAuth MUST NOT permit empty-password unseal.
        if p.len() != 80
            || p[..12] != [0, 78, 0, 8, 0, 11, 0, 0, 0, 18, 0, 32]
            || p[12..44].iter().all(|byte| *byte == 0)
            || p[44..48] != [0, 16, 0, 32]
            || self.pcrs.is_empty()
            || self.pcrs.len() > 24
            || self.pcrs.iter().any(|pcr| *pcr > 23)
            || self.pcrs.windows(2).any(|pair| pair[0] >= pair[1])
            || !(0x8100_0000..=0x81ff_ffff).contains(&self.persistent_handle)
            || !crate::fabric::valid_blake3_ref(&self.provider_ref)
            || !crate::fabric::valid_blake3_ref(&self.executable_ref)
        {
            return Err(Tpm2IdentityFailure::InvalidPolicy);
        }
        Ok(())
    }

    pub fn pcr_selection(&self) -> String {
        format!("sha256:{}", self.pcrs.iter().map(u8::to_string).collect::<Vec<_>>().join(","))
    }
}
