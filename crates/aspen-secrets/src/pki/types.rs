//! PKI secrets engine types.
//!
//! Data structures for certificate authority operations.

use serde::Deserialize;
use serde::Serialize;

/// Key type for CA keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PkiKeyType {
    /// RSA with the specified bit size (2048, 3072, 4096).
    #[default]
    Rsa2048,
    Rsa3072,
    Rsa4096,
    /// ECDSA with P-256 curve.
    EcdsaP256,
    /// ECDSA with P-384 curve.
    EcdsaP384,
    /// Ed25519 (not widely supported by browsers/clients).
    Ed25519,
}

impl PkiKeyType {
    /// Get the key type name.
    pub fn name(&self) -> &'static str {
        match self {
            PkiKeyType::Rsa2048 => "rsa-2048",
            PkiKeyType::Rsa3072 => "rsa-3072",
            PkiKeyType::Rsa4096 => "rsa-4096",
            PkiKeyType::EcdsaP256 => "ec-p256",
            PkiKeyType::EcdsaP384 => "ec-p384",
            PkiKeyType::Ed25519 => "ed25519",
        }
    }
}

/// Certificate Authority state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateAuthority {
    /// CA certificate in PEM format.
    pub certificate: String,
    /// CA private key in PEM format (encrypted).
    pub private_key: Vec<u8>,
    /// Key type used.
    pub key_type: PkiKeyType,
    /// Serial number for next issued certificate.
    pub next_serial: u64,
    /// Unix timestamp when the CA was created.
    pub created_time_unix_ms: u64,
    /// Unix timestamp when the CA certificate expires.
    pub expiry_time_unix_ms: u64,
    /// Whether this is a root CA (vs intermediate).
    pub is_root: bool,
    /// Issuing CA certificate chain (for intermediates).
    pub ca_chain: Vec<String>,
    /// Common name (for recreating CA cert for signing).
    pub common_name: String,
    /// Organization (for recreating CA cert for signing).
    pub organization: Option<String>,
    /// Organizational unit (for recreating CA cert for signing).
    pub ou: Option<String>,
    /// Country (for recreating CA cert for signing).
    pub country: Option<String>,
    /// Province/State (for recreating CA cert for signing).
    pub province: Option<String>,
    /// Locality (for recreating CA cert for signing).
    pub locality: Option<String>,
}

/// Certificate issuance role.
///
/// Roles define policies for certificate issuance, such as allowed domains,
/// key types, TTLs, and subject constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PkiRole {
    /// Role name.
    pub name: String,
    /// Allowed domains for this role.
    /// Certificates can only be issued for these domains.
    pub allowed_domains: Vec<String>,
    /// Allow subdomains of allowed_domains.
    pub allow_subdomains: bool,
    /// Allow bare (apex) domains.
    pub allow_bare_domains: bool,
    /// Allow localhost as a domain.
    pub allow_localhost: bool,
    /// Allow IP addresses in SANs.
    pub allow_ip_sans: bool,
    /// Allow wildcard certificates.
    pub allow_wildcard_certificates: bool,
    /// Allowed URI SANs patterns.
    pub allowed_uri_sans: Vec<String>,
    /// Key type for generated keys.
    pub key_type: PkiKeyType,
    /// Maximum TTL in seconds.
    pub max_ttl_secs: u64,
    /// Default TTL in seconds (used if not specified in request).
    pub ttl_secs: u64,
    /// Whether to generate the key (vs using CSR).
    pub generate_key: bool,
    /// Key usages to include.
    pub key_usages: Vec<String>,
    /// Extended key usages to include.
    pub ext_key_usages: Vec<String>,
    /// Whether the CN is required.
    pub require_cn: bool,
    /// Organization to use in subject.
    pub organization: Vec<String>,
    /// Organizational unit to use in subject.
    pub ou: Vec<String>,
    /// Country to use in subject.
    pub country: Vec<String>,
    /// State/province to use in subject.
    pub province: Vec<String>,
    /// Locality to use in subject.
    pub locality: Vec<String>,
    /// Whether to store issued certificates.
    pub no_store: bool,
    /// Unix timestamp when the role was created.
    pub created_time_unix_ms: u64,
}

impl Default for PkiRole {
    fn default() -> Self {
        Self {
            name: String::new(),
            allowed_domains: Vec::new(),
            allow_subdomains: false,
            allow_bare_domains: false,
            allow_localhost: false,
            allow_ip_sans: false,
            allow_wildcard_certificates: false,
            allowed_uri_sans: Vec::new(),
            key_type: PkiKeyType::default(),
            max_ttl_secs: crate::constants::DEFAULT_CERT_TTL_SECS,
            ttl_secs: crate::constants::DEFAULT_CERT_TTL_SECS,
            generate_key: true,
            key_usages: vec!["DigitalSignature".into(), "KeyEncipherment".into()],
            ext_key_usages: vec!["ServerAuth".into(), "ClientAuth".into()],
            require_cn: true,
            organization: Vec::new(),
            ou: Vec::new(),
            country: Vec::new(),
            province: Vec::new(),
            locality: Vec::new(),
            no_store: false,
            created_time_unix_ms: 0,
        }
    }
}

impl PkiRole {
    /// Create a new role with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Check if a domain is allowed by this role.
    pub fn allows_domain(&self, domain: &str) -> bool {
        // Check localhost
        if domain == "localhost" && self.allow_localhost {
            return true;
        }

        for allowed in &self.allowed_domains {
            // Exact match
            if allowed == domain {
                return self.allow_bare_domains;
            }

            // Subdomain match
            if self.allow_subdomains && domain.ends_with(&format!(".{}", allowed)) {
                return true;
            }

            // Wildcard check
            if self.allow_wildcard_certificates && domain.starts_with("*.") {
                let base = &domain[2..];
                if allowed == base || (self.allow_subdomains && base.ends_with(&format!(".{}", allowed))) {
                    return true;
                }
            }
        }

        false
    }
}

/// Issued certificate record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuedCertificate {
    /// Serial number (hex string).
    pub serial: String,
    /// Certificate in PEM format.
    pub certificate: String,
    /// Private key in PEM format (if generated).
    pub private_key: Option<String>,
    /// CA certificate chain in PEM format.
    pub ca_chain: Vec<String>,
    /// Common name.
    pub common_name: String,
    /// Subject Alternative Names.
    pub san: Vec<String>,
    /// Role used to issue this certificate.
    pub role: String,
    /// Unix timestamp when issued.
    pub issued_time_unix_ms: u64,
    /// Unix timestamp when the certificate expires.
    pub expiry_time_unix_ms: u64,
    /// Whether the certificate has been revoked.
    pub revoked: bool,
    /// Unix timestamp when revoked (if revoked).
    pub revocation_time_unix_ms: Option<u64>,
}

/// Certificate Revocation List entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrlEntry {
    /// Serial number of revoked certificate.
    pub serial: String,
    /// Unix timestamp when revoked.
    pub revocation_time_unix_ms: u64,
    /// Reason for revocation.
    pub reason: Option<String>,
}

/// CRL state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CrlState {
    /// Revoked certificates.
    pub entries: Vec<CrlEntry>,
    /// Unix timestamp when CRL was last updated.
    pub last_update_unix_ms: u64,
    /// Unix timestamp when CRL should be updated next.
    pub next_update_unix_ms: u64,
}

/// Request to generate a root CA.
#[derive(Debug, Clone)]
pub struct GenerateRootRequest {
    /// Common name for the CA.
    pub common_name: String,
    /// Key type to use.
    pub key_type: PkiKeyType,
    /// TTL for the CA certificate in seconds.
    pub ttl_secs: u64,
    /// Organization.
    pub organization: Option<String>,
    /// Organizational unit.
    pub ou: Option<String>,
    /// Country.
    pub country: Option<String>,
    /// State/province.
    pub province: Option<String>,
    /// Locality.
    pub locality: Option<String>,
}

impl GenerateRootRequest {
    /// Create a new root CA generation request.
    pub fn new(common_name: impl Into<String>) -> Self {
        Self {
            common_name: common_name.into(),
            key_type: PkiKeyType::default(),
            ttl_secs: 10 * 365 * 24 * 3600, // 10 years
            organization: None,
            ou: None,
            country: None,
            province: None,
            locality: None,
        }
    }

    /// Set the key type.
    pub fn with_key_type(mut self, key_type: PkiKeyType) -> Self {
        self.key_type = key_type;
        self
    }

    /// Set the TTL in seconds.
    pub fn with_ttl_secs(mut self, ttl: u64) -> Self {
        self.ttl_secs = ttl;
        self
    }
}

/// Response from generating a root CA.
#[derive(Debug, Clone)]
pub struct GenerateRootResponse {
    /// CA certificate in PEM format.
    pub certificate: String,
    /// Serial number (hex string).
    pub serial: String,
    /// Expiry time.
    pub expiry_time_unix_ms: u64,
}

/// Request to generate an intermediate CA CSR.
#[derive(Debug, Clone)]
pub struct GenerateIntermediateRequest {
    /// Common name for the intermediate CA.
    pub common_name: String,
    /// Key type to use.
    pub key_type: PkiKeyType,
    /// Organization.
    pub organization: Option<String>,
}

impl GenerateIntermediateRequest {
    /// Create a new intermediate CA CSR request.
    pub fn new(common_name: impl Into<String>) -> Self {
        Self {
            common_name: common_name.into(),
            key_type: PkiKeyType::default(),
            organization: None,
        }
    }
}

/// Response from generating an intermediate CA CSR.
#[derive(Debug, Clone)]
pub struct GenerateIntermediateResponse {
    /// CSR in PEM format (to be signed by root CA).
    pub csr: String,
}

/// Request to set a signed intermediate certificate.
#[derive(Debug, Clone)]
pub struct SetSignedIntermediateRequest {
    /// Signed certificate in PEM format.
    pub certificate: String,
}

/// Request to create a role.
#[derive(Debug, Clone)]
pub struct CreateRoleRequest {
    /// Role name.
    pub name: String,
    /// Role configuration.
    pub config: PkiRole,
}

impl CreateRoleRequest {
    /// Create a new role creation request.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            name: name.clone(),
            config: PkiRole::new(name),
        }
    }

    /// Set allowed domains.
    pub fn with_allowed_domains(mut self, domains: Vec<String>) -> Self {
        self.config.allowed_domains = domains;
        self
    }

    /// Allow subdomains.
    pub fn allow_subdomains(mut self) -> Self {
        self.config.allow_subdomains = true;
        self
    }

    /// Allow bare domains.
    pub fn allow_bare_domains(mut self) -> Self {
        self.config.allow_bare_domains = true;
        self
    }

    /// Set max TTL.
    pub fn with_max_ttl_secs(mut self, ttl: u64) -> Self {
        self.config.max_ttl_secs = ttl;
        self
    }

    /// Set default TTL.
    pub fn with_ttl_secs(mut self, ttl: u64) -> Self {
        self.config.ttl_secs = ttl;
        self
    }
}

/// Request to issue a certificate.
#[derive(Debug, Clone)]
pub struct IssueCertificateRequest {
    /// Role to use for issuance.
    pub role: String,
    /// Common name.
    pub common_name: String,
    /// Alternative names (DNS, IP, URI).
    pub alt_names: Vec<String>,
    /// IP SANs.
    pub ip_sans: Vec<String>,
    /// URI SANs.
    pub uri_sans: Vec<String>,
    /// TTL in seconds (uses role default if not specified).
    pub ttl_secs: Option<u64>,
    /// Exclude CN from SANs.
    pub exclude_cn_from_sans: bool,
}

impl IssueCertificateRequest {
    /// Create a new certificate issuance request.
    pub fn new(role: impl Into<String>, common_name: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            common_name: common_name.into(),
            alt_names: Vec::new(),
            ip_sans: Vec::new(),
            uri_sans: Vec::new(),
            ttl_secs: None,
            exclude_cn_from_sans: false,
        }
    }

    /// Add alternative names.
    pub fn with_alt_names(mut self, names: Vec<String>) -> Self {
        self.alt_names = names;
        self
    }

    /// Set TTL.
    pub fn with_ttl_secs(mut self, ttl: u64) -> Self {
        self.ttl_secs = Some(ttl);
        self
    }
}

/// Response from issuing a certificate.
#[derive(Debug, Clone)]
pub struct IssueCertificateResponse {
    /// Serial number (hex string).
    pub serial: String,
    /// Certificate in PEM format.
    pub certificate: String,
    /// Private key in PEM format (if generated).
    pub private_key: Option<String>,
    /// CA certificate chain in PEM format.
    pub ca_chain: Vec<String>,
    /// Expiry time.
    pub expiry_time_unix_ms: u64,
}

/// Request to revoke a certificate.
#[derive(Debug, Clone)]
pub struct RevokeCertificateRequest {
    /// Serial number to revoke.
    pub serial: String,
}

impl RevokeCertificateRequest {
    /// Create a new revocation request.
    pub fn new(serial: impl Into<String>) -> Self {
        Self { serial: serial.into() }
    }
}

/// Request to read a certificate.
#[derive(Debug, Clone)]
pub struct ReadCertificateRequest {
    /// Serial number.
    pub serial: String,
}

impl ReadCertificateRequest {
    /// Create a new read request.
    pub fn new(serial: impl Into<String>) -> Self {
        Self { serial: serial.into() }
    }
}

/// PKI engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PkiConfig {
    /// Default TTL for issued certificates (seconds).
    pub default_ttl_secs: u64,
    /// Maximum TTL for issued certificates (seconds).
    pub max_ttl_secs: u64,
    /// CRL distribution points to include in certificates.
    pub crl_distribution_points: Vec<String>,
    /// OCSP servers to include in certificates.
    pub ocsp_servers: Vec<String>,
    /// Issuing certificates URLs to include in certificates.
    pub issuing_certificates: Vec<String>,
}

/// Pending intermediate CA data.
///
/// Stored temporarily between `generate_intermediate` and `set_signed_intermediate`
/// calls to preserve the private key generated during CSR creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingIntermediateCa {
    /// Private key in PEM format.
    pub private_key: Vec<u8>,
    /// Key type used.
    pub key_type: PkiKeyType,
    /// Common name from the CSR.
    pub common_name: String,
    /// Organization from the CSR.
    pub organization: Option<String>,
}
