//! Independent test-only operator keys. These sign invocations, never source artifacts.

pub mod host;
pub mod intent;
pub mod scope;

/// The test-only operator error boundary.
pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

pub struct Issuer {
    issuer: basalt::ucan::Ed25519InMemorySigner,
    holder: basalt::ucan::Ed25519InMemorySigner,
    audience: basalt::ucan::AudienceDid,
    issuer_key: [u8; 32],
    holder_key: [u8; 32],
}

/// Explicit verification interval: host uses wall time; scope uses its operator trust time.
#[derive(Clone, Copy)]
pub struct Window {
    pub not_before: u64,
    pub expires: u64,
}

/// Every public operation re-authenticates the signed node source, which takes minutes on a loaded
/// host, so the operator window must outlast the whole supervised run; a one-hour window expires
/// between the first preview and the delivery that needs the same operator trust.
const OPERATOR_AUTHORITY_SECONDS: u64 = 12 * 3600;

impl Window {
    pub fn around_now() -> Result<Self> {
        // The operator interval is read through the admitted wall clock, not an ambient one.
        let now = molten::world_witness::WallClock::unix_seconds(&mut molten::world_witness::live_clock()?)
            .map_err(|error| format!("{error:?}"))?;
        Ok(Self {
            not_before: now.saturating_sub(60),
            expires: now.checked_add(OPERATOR_AUTHORITY_SECONDS).ok_or("time overflow")?,
        })
    }
}

impl Issuer {
    pub fn new(seed: [u8; 32]) -> Result<Self> {
        let holder_seed = *blake3::keyed_hash(&seed, b"collaboration-native-holder-v1").as_bytes();
        let issuer = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes(seed);
        let holder = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes(holder_seed);
        let audience = basalt::ucan::AudienceDid::from(
            basalt::ucan::TokenSigner::issuer(&holder).map_err(|error| format!("{error:?}"))?,
        );
        Ok(Self {
            issuer,
            holder,
            audience,
            issuer_key: basalt::ucan::derive_ed25519_public_key(&seed).map_err(|error| format!("{error:?}"))?,
            holder_key: basalt::ucan::derive_ed25519_public_key(&holder_seed).map_err(|error| format!("{error:?}"))?,
        })
    }

    pub fn issuer_did(&self) -> Result<String> {
        Ok(basalt::ucan::TokenSigner::issuer(&self.issuer)
            .map_err(|error| format!("{error:?}"))?
            .as_str()
            .to_owned())
    }

    pub fn holder_did(&self) -> &str {
        self.audience.as_str()
    }

    pub fn subject(&self) -> Result<molten::collaboration_scope::SubjectRef> {
        Ok(molten::collaboration_scope::subject_ref(&self.issuer_did()?, self.holder_did())?)
    }

    fn token(&self, resource: &str, ability: &str, window: Window) -> Result<basalt::ucan::CompactToken> {
        let capability = basalt::ucan::CapabilityDocument::new(resource.to_owned(), ability.to_owned())
            .map_err(|error| format!("{error:?}"))?;
        let capabilities = basalt::ucan::CapabilitySet::new(vec![capability]).map_err(|error| format!("{error:?}"))?;
        let time = basalt::ucan::TokenTimeBounds::new(window.not_before, window.expires)
            .map_err(|error| format!("{error:?}"))?;
        Ok(basalt::ucan::issue_token_with_signer(
            &self.issuer,
            &self.audience,
            &capabilities,
            &basalt::ucan::ProofReferences::empty(),
            time,
        )
        .map_err(|error| format!("{error:?}"))?)
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[usize::from(byte >> 4)] as char);
        output.push(DIGITS[usize::from(byte & 15)] as char);
    }
    output
}
