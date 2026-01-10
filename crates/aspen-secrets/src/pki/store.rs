//! PKI secrets store implementation.
//!
//! Provides certificate authority functionality with role-based issuance,
//! certificate revocation, and CRL management.

use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use async_trait::async_trait;
use der::Decode;
use rcgen::BasicConstraints;
use rcgen::Certificate;
use rcgen::CertificateParams;
use rcgen::DnType;
use rcgen::ExtendedKeyUsagePurpose;
use rcgen::IsCa;
use rcgen::KeyPair;
use rcgen::KeyUsagePurpose;
use rcgen::SanType;
use tracing::debug;
use x509_cert::Certificate as X509Certificate;

use crate::backend::SecretsBackend;
use crate::constants::MAX_COMMON_NAME_LENGTH;
use crate::constants::MAX_ROLE_NAME_LENGTH;
use crate::constants::MAX_SAN_COUNT;
use crate::error::Result;
use crate::error::SecretsError;
use crate::pki::types::*;

/// PKI secrets engine store.
///
/// Provides certificate authority functionality:
/// - Root and intermediate CA generation
/// - Role-based certificate issuance
/// - Certificate revocation
/// - CRL management
#[async_trait]
pub trait PkiStore: Send + Sync {
    /// Generate a root CA.
    async fn generate_root(&self, request: GenerateRootRequest) -> Result<GenerateRootResponse>;

    /// Generate an intermediate CA CSR.
    async fn generate_intermediate(&self, request: GenerateIntermediateRequest)
    -> Result<GenerateIntermediateResponse>;

    /// Set a signed intermediate certificate.
    async fn set_signed_intermediate(&self, request: SetSignedIntermediateRequest) -> Result<()>;

    /// Read the CA certificate.
    async fn read_ca(&self) -> Result<Option<String>>;

    /// Read the CA chain.
    async fn read_ca_chain(&self) -> Result<Vec<String>>;

    /// Create a role.
    async fn create_role(&self, request: CreateRoleRequest) -> Result<PkiRole>;

    /// Read a role.
    async fn read_role(&self, name: &str) -> Result<Option<PkiRole>>;

    /// Delete a role.
    async fn delete_role(&self, name: &str) -> Result<bool>;

    /// List all roles.
    async fn list_roles(&self) -> Result<Vec<String>>;

    /// Issue a certificate.
    async fn issue(&self, request: IssueCertificateRequest) -> Result<IssueCertificateResponse>;

    /// Revoke a certificate.
    async fn revoke(&self, request: RevokeCertificateRequest) -> Result<()>;

    /// Read a certificate by serial.
    async fn read_certificate(&self, request: ReadCertificateRequest) -> Result<Option<IssuedCertificate>>;

    /// List issued certificates.
    async fn list_certificates(&self) -> Result<Vec<String>>;

    /// Get the CRL.
    async fn get_crl(&self) -> Result<CrlState>;

    /// Get the CA certificate status.
    async fn ca_status(&self) -> Result<Option<CertificateAuthority>>;
}

/// Default PKI store implementation using SecretsBackend.
pub struct DefaultPkiStore {
    /// Storage backend.
    backend: Arc<dyn SecretsBackend>,
}

impl DefaultPkiStore {
    /// Create a new PKI store with the given backend.
    pub fn new(backend: Arc<dyn SecretsBackend>) -> Self {
        Self { backend }
    }

    /// Get current timestamp in milliseconds.
    fn now_unix_ms() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
    }

    /// Load CA from storage.
    async fn load_ca(&self) -> Result<Option<CertificateAuthority>> {
        let data = self.backend.get("ca").await?;
        match data {
            Some(bytes) => {
                let ca: CertificateAuthority = postcard::from_bytes(&bytes).map_err(|e| SecretsError::Internal {
                    reason: format!("corrupted CA data: {e}"),
                })?;
                Ok(Some(ca))
            }
            None => Ok(None),
        }
    }

    /// Save CA to storage.
    async fn save_ca(&self, ca: &CertificateAuthority) -> Result<()> {
        let bytes = postcard::to_allocvec(ca).map_err(|e| SecretsError::Serialization { reason: e.to_string() })?;
        self.backend.put("ca", &bytes).await
    }

    /// Load role from storage.
    async fn load_role(&self, name: &str) -> Result<Option<PkiRole>> {
        let path = format!("roles/{}", name);
        let data = self.backend.get(&path).await?;
        match data {
            Some(bytes) => {
                let role: PkiRole = postcard::from_bytes(&bytes).map_err(|e| SecretsError::Internal {
                    reason: format!("corrupted role data: {e}"),
                })?;
                Ok(Some(role))
            }
            None => Ok(None),
        }
    }

    /// Save role to storage.
    async fn save_role(&self, role: &PkiRole) -> Result<()> {
        let path = format!("roles/{}", role.name);
        let bytes = postcard::to_allocvec(role).map_err(|e| SecretsError::Serialization { reason: e.to_string() })?;
        self.backend.put(&path, &bytes).await
    }

    /// Load certificate from storage.
    async fn load_certificate(&self, serial: &str) -> Result<Option<IssuedCertificate>> {
        let path = format!("certs/{}", serial);
        let data = self.backend.get(&path).await?;
        match data {
            Some(bytes) => {
                let cert: IssuedCertificate = postcard::from_bytes(&bytes).map_err(|e| SecretsError::Internal {
                    reason: format!("corrupted certificate data: {e}"),
                })?;
                Ok(Some(cert))
            }
            None => Ok(None),
        }
    }

    /// Save certificate to storage.
    async fn save_certificate(&self, cert: &IssuedCertificate) -> Result<()> {
        let path = format!("certs/{}", cert.serial);
        let bytes = postcard::to_allocvec(cert).map_err(|e| SecretsError::Serialization { reason: e.to_string() })?;
        self.backend.put(&path, &bytes).await
    }

    /// Load CRL from storage.
    async fn load_crl(&self) -> Result<CrlState> {
        let data = self.backend.get("crl").await?;
        match data {
            Some(bytes) => {
                let crl: CrlState = postcard::from_bytes(&bytes).map_err(|e| SecretsError::Internal {
                    reason: format!("corrupted CRL data: {e}"),
                })?;
                Ok(crl)
            }
            None => Ok(CrlState::default()),
        }
    }

    /// Save CRL to storage.
    async fn save_crl(&self, crl: &CrlState) -> Result<()> {
        let bytes = postcard::to_allocvec(crl).map_err(|e| SecretsError::Serialization { reason: e.to_string() })?;
        self.backend.put("crl", &bytes).await
    }

    /// Generate a key pair for the given key type.
    fn generate_key_pair(key_type: PkiKeyType) -> Result<KeyPair> {
        // rcgen currently only supports RSA and ECDSA through ring
        // For simplicity, we'll use the default (ECDSA P-256) for now
        // In production, we'd use different algorithms based on key_type
        let algorithm = match key_type {
            PkiKeyType::EcdsaP256 => &rcgen::PKCS_ECDSA_P256_SHA256,
            PkiKeyType::EcdsaP384 => &rcgen::PKCS_ECDSA_P384_SHA384,
            PkiKeyType::Ed25519 => &rcgen::PKCS_ED25519,
            PkiKeyType::Rsa2048 | PkiKeyType::Rsa3072 | PkiKeyType::Rsa4096 => {
                // RSA requires different handling in rcgen
                // Fall back to ECDSA P-256 for now
                &rcgen::PKCS_ECDSA_P256_SHA256
            }
        };

        KeyPair::generate_for(algorithm).map_err(|e| SecretsError::CertificateGeneration { reason: e.to_string() })
    }

    /// Generate a serial number.
    fn generate_serial() -> String {
        use rand::Rng;
        let mut bytes = [0u8; 16];
        rand::rng().fill(&mut bytes);
        hex::encode(bytes)
    }

    /// Validate common name.
    fn validate_cn(cn: &str) -> Result<()> {
        if cn.is_empty() || cn.len() > MAX_COMMON_NAME_LENGTH {
            return Err(SecretsError::PathTooLong {
                length: cn.len(),
                max: MAX_COMMON_NAME_LENGTH,
            });
        }
        Ok(())
    }

    /// Validate role name.
    fn validate_role_name(name: &str) -> Result<()> {
        if name.is_empty() || name.len() > MAX_ROLE_NAME_LENGTH {
            return Err(SecretsError::PathTooLong {
                length: name.len(),
                max: MAX_ROLE_NAME_LENGTH,
            });
        }
        Ok(())
    }

    /// Parse a PEM-encoded certificate and extract validity timestamps.
    ///
    /// Returns (not_before_ms, not_after_ms) as Unix timestamps in milliseconds.
    fn parse_certificate_validity(pem: &str) -> Result<(u64, u64)> {
        // Extract the DER-encoded certificate from PEM
        let pem_bytes = pem.as_bytes();
        let (_, pem_doc) =
            x509_cert::der::pem::decode_vec(pem_bytes).map_err(|e| SecretsError::InvalidCertificate {
                reason: format!("failed to decode PEM: {e}"),
            })?;

        // Parse the X.509 certificate
        let cert = X509Certificate::from_der(&pem_doc).map_err(|e| SecretsError::InvalidCertificate {
            reason: format!("failed to parse certificate: {e}"),
        })?;

        // Extract validity period
        let validity = &cert.tbs_certificate.validity;

        // Convert to Unix timestamps in milliseconds
        let not_before = validity.not_before.to_unix_duration().as_millis() as u64;
        let not_after = validity.not_after.to_unix_duration().as_millis() as u64;

        Ok((not_before, not_after))
    }

    /// Recreate CA certificate from stored data for signing operations.
    ///
    /// This recreates the CA Certificate object needed for signing new certificates.
    fn recreate_ca_cert(ca: &CertificateAuthority, ca_key: &KeyPair) -> Result<Certificate> {
        let mut params = CertificateParams::default();
        params.distinguished_name.push(DnType::CommonName, &ca.common_name);

        if let Some(org) = &ca.organization {
            params.distinguished_name.push(DnType::OrganizationName, org);
        }
        if let Some(ou) = &ca.ou {
            params.distinguished_name.push(DnType::OrganizationalUnitName, ou);
        }
        if let Some(country) = &ca.country {
            params.distinguished_name.push(DnType::CountryName, country);
        }
        if let Some(province) = &ca.province {
            params.distinguished_name.push(DnType::StateOrProvinceName, province);
        }
        if let Some(locality) = &ca.locality {
            params.distinguished_name.push(DnType::LocalityName, locality);
        }

        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];

        // Set validity from stored timestamps
        let not_before = std::time::UNIX_EPOCH + Duration::from_millis(ca.created_time_unix_ms);
        let not_after = std::time::UNIX_EPOCH + Duration::from_millis(ca.expiry_time_unix_ms);
        params.not_before = rcgen::date_time_ymd(
            time::OffsetDateTime::from(not_before).year(),
            time::OffsetDateTime::from(not_before).month() as u8,
            time::OffsetDateTime::from(not_before).day(),
        );
        params.not_after = rcgen::date_time_ymd(
            time::OffsetDateTime::from(not_after).year(),
            time::OffsetDateTime::from(not_after).month() as u8,
            time::OffsetDateTime::from(not_after).day(),
        );

        params
            .self_signed(ca_key)
            .map_err(|e| SecretsError::CertificateGeneration { reason: e.to_string() })
    }
}

#[async_trait]
impl PkiStore for DefaultPkiStore {
    async fn generate_root(&self, request: GenerateRootRequest) -> Result<GenerateRootResponse> {
        Self::validate_cn(&request.common_name)?;

        // Check if CA already exists
        if self.load_ca().await?.is_some() {
            return Err(SecretsError::CaAlreadyInitialized { mount: "pki".into() });
        }

        let now = Self::now_unix_ms();
        let expiry_ms = now + (request.ttl_secs * 1000);

        // Generate key pair
        let key_pair = Self::generate_key_pair(request.key_type)?;

        // Create certificate params
        let mut params = CertificateParams::default();
        params.distinguished_name.push(DnType::CommonName, &request.common_name);

        if let Some(org) = &request.organization {
            params.distinguished_name.push(DnType::OrganizationName, org);
        }
        if let Some(ou) = &request.ou {
            params.distinguished_name.push(DnType::OrganizationalUnitName, ou);
        }
        if let Some(country) = &request.country {
            params.distinguished_name.push(DnType::CountryName, country);
        }
        if let Some(province) = &request.province {
            params.distinguished_name.push(DnType::StateOrProvinceName, province);
        }
        if let Some(locality) = &request.locality {
            params.distinguished_name.push(DnType::LocalityName, locality);
        }

        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];

        // Set validity
        let not_before = std::time::SystemTime::now();
        let not_after = not_before + Duration::from_secs(request.ttl_secs);
        params.not_before = rcgen::date_time_ymd(
            time::OffsetDateTime::from(not_before).year(),
            time::OffsetDateTime::from(not_before).month() as u8,
            time::OffsetDateTime::from(not_before).day(),
        );
        params.not_after = rcgen::date_time_ymd(
            time::OffsetDateTime::from(not_after).year(),
            time::OffsetDateTime::from(not_after).month() as u8,
            time::OffsetDateTime::from(not_after).day(),
        );

        let serial = Self::generate_serial();

        // Generate self-signed certificate
        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| SecretsError::CertificateGeneration { reason: e.to_string() })?;

        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        // Store CA
        let ca = CertificateAuthority {
            certificate: cert_pem.clone(),
            private_key: key_pem.into_bytes(),
            key_type: request.key_type,
            next_serial: 2, // 1 is the CA itself
            created_time_unix_ms: now,
            expiry_time_unix_ms: expiry_ms,
            is_root: true,
            ca_chain: vec![],
            common_name: request.common_name.clone(),
            organization: request.organization.clone(),
            ou: request.ou.clone(),
            country: request.country.clone(),
            province: request.province.clone(),
            locality: request.locality.clone(),
        };

        self.save_ca(&ca).await?;

        debug!(cn = %request.common_name, "Generated root CA");

        Ok(GenerateRootResponse {
            certificate: cert_pem,
            serial,
            expiry_time_unix_ms: expiry_ms,
        })
    }

    async fn generate_intermediate(
        &self,
        request: GenerateIntermediateRequest,
    ) -> Result<GenerateIntermediateResponse> {
        Self::validate_cn(&request.common_name)?;

        // Check if CA already exists
        if self.load_ca().await?.is_some() {
            return Err(SecretsError::CaAlreadyInitialized { mount: "pki".into() });
        }

        // Generate key pair for the intermediate CA
        let key_pair = Self::generate_key_pair(request.key_type)?;

        // Create certificate params for CSR
        // NOTE: CSRs only support distinguished name and SANs.
        // The CA extensions (is_ca, key_usages) are added by the signing CA,
        // not included in the CSR itself.
        let mut params = CertificateParams::default();
        params.distinguished_name.push(DnType::CommonName, &request.common_name);

        if let Some(org) = &request.organization {
            params.distinguished_name.push(DnType::OrganizationName, org);
        }

        // Generate the CSR (without CA extensions - those are added during signing)
        let csr = params
            .serialize_request(&key_pair)
            .map_err(|e| SecretsError::CertificateGeneration { reason: e.to_string() })?;
        let csr_pem = csr.pem().map_err(|e| SecretsError::CertificateGeneration { reason: e.to_string() })?;

        // Store the pending intermediate CA data (key pair and metadata)
        // This will be used when set_signed_intermediate is called
        let pending = PendingIntermediateCa {
            private_key: key_pair.serialize_pem().into_bytes(),
            key_type: request.key_type,
            common_name: request.common_name.clone(),
            organization: request.organization.clone(),
        };

        let bytes =
            postcard::to_allocvec(&pending).map_err(|e| SecretsError::Serialization { reason: e.to_string() })?;
        self.backend.put("intermediate_pending", &bytes).await?;

        debug!(cn = %request.common_name, "Generated intermediate CA CSR");

        Ok(GenerateIntermediateResponse { csr: csr_pem })
    }

    async fn set_signed_intermediate(&self, request: SetSignedIntermediateRequest) -> Result<()> {
        // Check if CA already exists
        if self.load_ca().await?.is_some() {
            return Err(SecretsError::CaAlreadyInitialized { mount: "pki".into() });
        }

        // Load pending intermediate CA data
        let pending_data = self.backend.get("intermediate_pending").await?.ok_or_else(|| SecretsError::Internal {
            reason: "No pending intermediate CA found. Call generate_intermediate first.".into(),
        })?;

        let pending: PendingIntermediateCa =
            postcard::from_bytes(&pending_data).map_err(|e| SecretsError::Internal {
                reason: format!("corrupted pending intermediate data: {e}"),
            })?;

        // Parse the signed certificate to extract validity information
        // We'll store the certificate as-is and extract metadata
        let cert_pem = request.certificate.trim();
        if !cert_pem.contains("BEGIN CERTIFICATE") {
            return Err(SecretsError::InvalidCertificate {
                reason: "Certificate must be in PEM format".into(),
            });
        }

        // Parse the certificate to extract expiry using x509-parser
        let (not_before, not_after) = Self::parse_certificate_validity(cert_pem)?;

        let now = Self::now_unix_ms();

        // Create the CA record
        let ca = CertificateAuthority {
            certificate: cert_pem.to_string(),
            private_key: pending.private_key,
            key_type: pending.key_type,
            next_serial: 1,
            created_time_unix_ms: now,
            expiry_time_unix_ms: not_after,
            is_root: false,
            // The CA chain should include the signing CA's certificate
            // For now, we store an empty chain - the user should provide the chain
            ca_chain: vec![],
            common_name: pending.common_name.clone(),
            organization: pending.organization,
            ou: None,
            country: None,
            province: None,
            locality: None,
        };

        // Validate that not_before is in the past (certificate is valid)
        if not_before > now {
            return Err(SecretsError::InvalidCertificate {
                reason: "Certificate is not yet valid".into(),
            });
        }

        // Validate that not_after is in the future (certificate has not expired)
        if not_after <= now {
            return Err(SecretsError::InvalidCertificate {
                reason: "Certificate has expired".into(),
            });
        }

        self.save_ca(&ca).await?;

        // Clean up pending data
        self.backend.delete("intermediate_pending").await?;

        debug!(cn = %pending.common_name, "Set signed intermediate CA certificate");

        Ok(())
    }

    async fn read_ca(&self) -> Result<Option<String>> {
        let ca = self.load_ca().await?;
        Ok(ca.map(|c| c.certificate))
    }

    async fn read_ca_chain(&self) -> Result<Vec<String>> {
        let ca = self.load_ca().await?;
        match ca {
            Some(c) => {
                let mut chain = vec![c.certificate];
                chain.extend(c.ca_chain);
                Ok(chain)
            }
            None => Ok(vec![]),
        }
    }

    async fn create_role(&self, request: CreateRoleRequest) -> Result<PkiRole> {
        Self::validate_role_name(&request.name)?;

        // Check if CA is initialized
        if self.load_ca().await?.is_none() {
            return Err(SecretsError::CaNotInitialized);
        }

        // Check if role already exists
        if self.load_role(&request.name).await?.is_some() {
            return Err(SecretsError::RoleExists { name: request.name });
        }

        let now = Self::now_unix_ms();
        let mut role = request.config;
        role.created_time_unix_ms = now;

        self.save_role(&role).await?;

        debug!(name = %request.name, "Created PKI role");

        Ok(role)
    }

    async fn read_role(&self, name: &str) -> Result<Option<PkiRole>> {
        Self::validate_role_name(name)?;
        self.load_role(name).await
    }

    async fn delete_role(&self, name: &str) -> Result<bool> {
        Self::validate_role_name(name)?;
        let path = format!("roles/{}", name);
        self.backend.delete(&path).await
    }

    async fn list_roles(&self) -> Result<Vec<String>> {
        let roles = self.backend.list("roles/").await?;
        Ok(roles.into_iter().filter(|r| !r.ends_with('/')).collect())
    }

    async fn issue(&self, request: IssueCertificateRequest) -> Result<IssueCertificateResponse> {
        Self::validate_cn(&request.common_name)?;

        // Validate SAN count
        let total_sans = request.alt_names.len() + request.ip_sans.len() + request.uri_sans.len();
        if total_sans > MAX_SAN_COUNT as usize {
            return Err(SecretsError::TooManySans {
                count: total_sans as u32,
                max: MAX_SAN_COUNT,
            });
        }

        // Load CA
        let mut ca = self.load_ca().await?.ok_or(SecretsError::CaNotInitialized)?;

        // Load role
        let role = self.load_role(&request.role).await?.ok_or_else(|| SecretsError::RoleNotFound {
            name: request.role.clone(),
        })?;

        // Validate common name against role
        if !role.allows_domain(&request.common_name) {
            return Err(SecretsError::CommonNameNotAllowed {
                cn: request.common_name,
                role: request.role,
            });
        }

        // Validate alt names
        for name in &request.alt_names {
            if !role.allows_domain(name) {
                return Err(SecretsError::SanNotAllowed {
                    san: name.clone(),
                    role: request.role.clone(),
                });
            }
        }

        // Validate IP SANs
        if !request.ip_sans.is_empty() && !role.allow_ip_sans {
            return Err(SecretsError::SanNotAllowed {
                san: request.ip_sans[0].clone(),
                role: request.role.clone(),
            });
        }

        // Determine TTL
        let ttl = request.ttl_secs.unwrap_or(role.ttl_secs).min(role.max_ttl_secs);
        if ttl > crate::constants::MAX_CERT_TTL_SECS {
            return Err(SecretsError::TtlExceedsMax {
                role: request.role,
                requested_secs: ttl,
                max_secs: crate::constants::MAX_CERT_TTL_SECS,
            });
        }

        let now = Self::now_unix_ms();
        let expiry_ms = now + (ttl * 1000);

        // Generate key pair for the certificate
        let key_pair = Self::generate_key_pair(role.key_type)?;

        // Create certificate params
        let mut params = CertificateParams::default();
        params.distinguished_name.push(DnType::CommonName, &request.common_name);

        for org in &role.organization {
            params.distinguished_name.push(DnType::OrganizationName, org);
        }
        for ou in &role.ou {
            params.distinguished_name.push(DnType::OrganizationalUnitName, ou);
        }
        for country in &role.country {
            params.distinguished_name.push(DnType::CountryName, country);
        }

        // Add SANs
        let mut sans = Vec::new();
        if !request.exclude_cn_from_sans {
            sans.push(SanType::DnsName(request.common_name.clone().try_into().map_err(|_| {
                SecretsError::CommonNameNotAllowed {
                    cn: request.common_name.clone(),
                    role: request.role.clone(),
                }
            })?));
        }
        for name in &request.alt_names {
            sans.push(SanType::DnsName(name.clone().try_into().map_err(|_| SecretsError::SanNotAllowed {
                san: name.clone(),
                role: request.role.clone(),
            })?));
        }
        for ip in &request.ip_sans {
            let addr: std::net::IpAddr = ip.parse().map_err(|_| SecretsError::SanNotAllowed {
                san: ip.clone(),
                role: request.role.clone(),
            })?;
            sans.push(SanType::IpAddress(addr));
        }
        params.subject_alt_names = sans;

        // Key usages
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature, KeyUsagePurpose::KeyEncipherment];
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth, ExtendedKeyUsagePurpose::ClientAuth];

        params.is_ca = IsCa::NoCa;

        // Set validity
        let not_before = std::time::SystemTime::now();
        let not_after = not_before + Duration::from_secs(ttl);
        params.not_before = rcgen::date_time_ymd(
            time::OffsetDateTime::from(not_before).year(),
            time::OffsetDateTime::from(not_before).month() as u8,
            time::OffsetDateTime::from(not_before).day(),
        );
        params.not_after = rcgen::date_time_ymd(
            time::OffsetDateTime::from(not_after).year(),
            time::OffsetDateTime::from(not_after).month() as u8,
            time::OffsetDateTime::from(not_after).day(),
        );

        // Load CA key pair
        let ca_key_pem = String::from_utf8(ca.private_key.clone()).map_err(|e| SecretsError::Internal {
            reason: format!("CA key encoding error: {e}"),
        })?;
        let ca_key = KeyPair::from_pem(&ca_key_pem).map_err(|e| SecretsError::Internal {
            reason: format!("CA key parse error: {e}"),
        })?;

        // Recreate CA cert from stored data
        let ca_cert = Self::recreate_ca_cert(&ca, &ca_key)?;

        // Sign the certificate
        let signed = params
            .signed_by(&key_pair, &ca_cert, &ca_key)
            .map_err(|e| SecretsError::CertificateGeneration { reason: e.to_string() })?;

        let serial = Self::generate_serial();
        let cert_pem = signed.pem();
        let key_pem = key_pair.serialize_pem();

        // Build SAN list for storage
        let mut san_list: Vec<String> = request.alt_names.clone();
        san_list.extend(request.ip_sans.iter().cloned());
        san_list.extend(request.uri_sans.iter().cloned());

        // Store certificate (if not no_store)
        if !role.no_store {
            let issued = IssuedCertificate {
                serial: serial.clone(),
                certificate: cert_pem.clone(),
                private_key: Some(key_pem.clone()),
                ca_chain: vec![ca.certificate.clone()],
                common_name: request.common_name.clone(),
                san: san_list,
                role: request.role.clone(),
                issued_time_unix_ms: now,
                expiry_time_unix_ms: expiry_ms,
                revoked: false,
                revocation_time_unix_ms: None,
            };
            self.save_certificate(&issued).await?;
        }

        // Update CA serial counter
        ca.next_serial += 1;
        self.save_ca(&ca).await?;

        debug!(cn = %request.common_name, role = %request.role, "Issued certificate");

        Ok(IssueCertificateResponse {
            serial,
            certificate: cert_pem,
            private_key: Some(key_pem),
            ca_chain: vec![ca.certificate],
            expiry_time_unix_ms: expiry_ms,
        })
    }

    async fn revoke(&self, request: RevokeCertificateRequest) -> Result<()> {
        // Load certificate
        let mut cert =
            self.load_certificate(&request.serial).await?.ok_or_else(|| SecretsError::CertificateNotFound {
                serial: request.serial.clone(),
            })?;

        if cert.revoked {
            return Err(SecretsError::CertificateAlreadyRevoked { serial: request.serial });
        }

        let now = Self::now_unix_ms();

        // Mark as revoked
        cert.revoked = true;
        cert.revocation_time_unix_ms = Some(now);
        self.save_certificate(&cert).await?;

        // Add to CRL
        let mut crl = self.load_crl().await?;
        crl.entries.push(CrlEntry {
            serial: request.serial.clone(),
            revocation_time_unix_ms: now,
            reason: None,
        });
        crl.last_update_unix_ms = now;
        crl.next_update_unix_ms = now + (24 * 60 * 60 * 1000); // 24 hours
        self.save_crl(&crl).await?;

        debug!(serial = %request.serial, "Revoked certificate");

        Ok(())
    }

    async fn read_certificate(&self, request: ReadCertificateRequest) -> Result<Option<IssuedCertificate>> {
        self.load_certificate(&request.serial).await
    }

    async fn list_certificates(&self) -> Result<Vec<String>> {
        let certs = self.backend.list("certs/").await?;
        Ok(certs.into_iter().filter(|c| !c.ends_with('/')).collect())
    }

    async fn get_crl(&self) -> Result<CrlState> {
        self.load_crl().await
    }

    async fn ca_status(&self) -> Result<Option<CertificateAuthority>> {
        self.load_ca().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::InMemorySecretsBackend;

    fn make_store() -> DefaultPkiStore {
        let backend = Arc::new(InMemorySecretsBackend::new());
        DefaultPkiStore::new(backend)
    }

    #[tokio::test]
    async fn test_generate_root() {
        let store = make_store();

        let req = GenerateRootRequest::new("Test CA")
            .with_key_type(PkiKeyType::EcdsaP256)
            .with_ttl_secs(365 * 24 * 3600);

        let res = store.generate_root(req).await.unwrap();

        assert!(res.certificate.contains("BEGIN CERTIFICATE"));
        assert!(!res.serial.is_empty());

        // Read CA back
        let ca_cert = store.read_ca().await.unwrap().unwrap();
        assert_eq!(ca_cert, res.certificate);
    }

    #[tokio::test]
    async fn test_create_role() {
        let store = make_store();

        // Generate CA first
        store.generate_root(GenerateRootRequest::new("Test CA")).await.unwrap();

        // Create role
        let req = CreateRoleRequest::new("web-server")
            .with_allowed_domains(vec!["example.com".into()])
            .allow_subdomains()
            .allow_bare_domains();

        let role = store.create_role(req).await.unwrap();
        assert_eq!(role.name, "web-server");
        assert!(role.allowed_domains.contains(&"example.com".to_string()));

        // Read role back
        let read_role = store.read_role("web-server").await.unwrap().unwrap();
        assert_eq!(read_role.name, "web-server");
    }

    #[tokio::test]
    async fn test_issue_certificate() {
        let store = make_store();

        // Generate CA
        store.generate_root(GenerateRootRequest::new("Test CA")).await.unwrap();

        // Create role
        store
            .create_role(
                CreateRoleRequest::new("web")
                    .with_allowed_domains(vec!["example.com".into()])
                    .allow_subdomains()
                    .allow_bare_domains(),
            )
            .await
            .unwrap();

        // Issue certificate
        let req = IssueCertificateRequest::new("web", "www.example.com").with_ttl_secs(30 * 24 * 3600);

        let res = store.issue(req).await.unwrap();

        assert!(res.certificate.contains("BEGIN CERTIFICATE"));
        assert!(res.private_key.unwrap().contains("BEGIN PRIVATE KEY"));
        assert!(!res.serial.is_empty());
    }

    #[tokio::test]
    async fn test_revoke_certificate() {
        let store = make_store();

        // Setup CA and role
        store.generate_root(GenerateRootRequest::new("Test CA")).await.unwrap();
        store
            .create_role(
                CreateRoleRequest::new("web").with_allowed_domains(vec!["example.com".into()]).allow_bare_domains(),
            )
            .await
            .unwrap();

        // Issue certificate
        let issued = store.issue(IssueCertificateRequest::new("web", "example.com")).await.unwrap();

        // Revoke it
        store.revoke(RevokeCertificateRequest::new(&issued.serial)).await.unwrap();

        // Check it's revoked
        let cert = store.read_certificate(ReadCertificateRequest::new(&issued.serial)).await.unwrap().unwrap();
        assert!(cert.revoked);

        // Check CRL
        let crl = store.get_crl().await.unwrap();
        assert!(crl.entries.iter().any(|e| e.serial == issued.serial));
    }

    #[tokio::test]
    async fn test_domain_validation() {
        let store = make_store();

        store.generate_root(GenerateRootRequest::new("Test CA")).await.unwrap();
        store
            .create_role(CreateRoleRequest::new("web").with_allowed_domains(vec!["example.com".into()]))
            .await
            .unwrap();

        // Should fail - domain not allowed (bare domains not allowed)
        let res = store.issue(IssueCertificateRequest::new("web", "example.com")).await;
        assert!(matches!(res.unwrap_err(), SecretsError::CommonNameNotAllowed { .. }));
    }

    #[tokio::test]
    async fn test_generate_intermediate_csr() {
        let store = make_store();

        // Generate intermediate CA CSR
        let req = GenerateIntermediateRequest::new("Intermediate CA");
        let res = store.generate_intermediate(req).await.unwrap();

        // Verify CSR format
        assert!(res.csr.contains("BEGIN CERTIFICATE REQUEST"));
        assert!(res.csr.contains("END CERTIFICATE REQUEST"));

        // Verify pending data was stored
        let pending_data = store.backend.get("intermediate_pending").await.unwrap();
        assert!(pending_data.is_some());
    }

    #[tokio::test]
    async fn test_generate_intermediate_fails_if_ca_exists() {
        let store = make_store();

        // Generate root CA first
        store.generate_root(GenerateRootRequest::new("Root CA")).await.unwrap();

        // Generating intermediate should fail
        let req = GenerateIntermediateRequest::new("Intermediate CA");
        let res = store.generate_intermediate(req).await;
        assert!(matches!(res.unwrap_err(), SecretsError::CaAlreadyInitialized { .. }));
    }

    #[tokio::test]
    async fn test_set_signed_intermediate_without_pending() {
        let store = make_store();

        // Try to set signed intermediate without generating CSR first
        let req = SetSignedIntermediateRequest {
            certificate: "-----BEGIN CERTIFICATE-----\nMIIB...\n-----END CERTIFICATE-----".into(),
        };
        let res = store.set_signed_intermediate(req).await;
        assert!(matches!(res.unwrap_err(), SecretsError::Internal { .. }));
    }

    #[tokio::test]
    async fn test_intermediate_ca_full_flow() {
        // This test simulates the full intermediate CA flow:
        // 1. Generate intermediate CSR
        // 2. Sign CSR with a root CA
        // 3. Set the signed certificate

        let intermediate_store = make_store();
        let root_store = make_store();

        // Generate root CA in separate store (simulating external root)
        let _root_res = root_store.generate_root(GenerateRootRequest::new("Root CA")).await.unwrap();

        // Generate intermediate CSR
        let csr_res = intermediate_store
            .generate_intermediate(GenerateIntermediateRequest::new("Intermediate CA"))
            .await
            .unwrap();

        // Load the CSR and sign it with the root CA
        // We need to parse the CSR and create a certificate signed by the root CA
        let csr_pem = &csr_res.csr;

        // Parse the CSR and add CA extensions (not included in CSR)
        let mut csr = rcgen::CertificateSigningRequestParams::from_pem(csr_pem).unwrap();

        // Add CA extensions to the params (these are added by the signing CA)
        csr.params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        csr.params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];

        // Load root CA key
        let root_ca = root_store.load_ca().await.unwrap().unwrap();
        let root_key_pem = String::from_utf8(root_ca.private_key.clone()).unwrap();
        let root_key = KeyPair::from_pem(&root_key_pem).unwrap();

        // Recreate root CA cert for signing
        let root_cert = DefaultPkiStore::recreate_ca_cert(&root_ca, &root_key).unwrap();

        // Sign the CSR to create intermediate certificate
        let signed_cert = csr.signed_by(&root_cert, &root_key).unwrap();
        let signed_cert_pem = signed_cert.pem();

        // Set the signed intermediate certificate
        let set_req = SetSignedIntermediateRequest {
            certificate: signed_cert_pem,
        };
        intermediate_store.set_signed_intermediate(set_req).await.unwrap();

        // Verify CA is now set up
        let ca_status = intermediate_store.ca_status().await.unwrap().unwrap();
        assert!(!ca_status.is_root);
        assert!(ca_status.certificate.contains("BEGIN CERTIFICATE"));

        // Verify we can read the CA certificate
        let ca_cert = intermediate_store.read_ca().await.unwrap().unwrap();
        assert!(ca_cert.contains("BEGIN CERTIFICATE"));

        // Verify pending data was cleaned up
        let pending_data = intermediate_store.backend.get("intermediate_pending").await.unwrap();
        assert!(pending_data.is_none());
    }

    #[tokio::test]
    async fn test_intermediate_ca_can_issue_certificates() {
        // Test that an intermediate CA can issue end-entity certificates
        let intermediate_store = make_store();
        let root_store = make_store();

        // Set up root CA
        root_store.generate_root(GenerateRootRequest::new("Root CA")).await.unwrap();

        // Generate and sign intermediate CA
        let csr_res = intermediate_store
            .generate_intermediate(GenerateIntermediateRequest::new("Intermediate CA"))
            .await
            .unwrap();

        let root_ca = root_store.load_ca().await.unwrap().unwrap();
        let root_key_pem = String::from_utf8(root_ca.private_key.clone()).unwrap();
        let root_key = KeyPair::from_pem(&root_key_pem).unwrap();
        let root_cert = DefaultPkiStore::recreate_ca_cert(&root_ca, &root_key).unwrap();

        // Parse CSR and add CA extensions
        let mut csr = rcgen::CertificateSigningRequestParams::from_pem(&csr_res.csr).unwrap();
        csr.params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        csr.params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];
        let signed_cert = csr.signed_by(&root_cert, &root_key).unwrap();

        intermediate_store
            .set_signed_intermediate(SetSignedIntermediateRequest {
                certificate: signed_cert.pem(),
            })
            .await
            .unwrap();

        // Create role on intermediate CA
        intermediate_store
            .create_role(
                CreateRoleRequest::new("web").with_allowed_domains(vec!["example.com".into()]).allow_bare_domains(),
            )
            .await
            .unwrap();

        // Issue certificate from intermediate CA
        let issued = intermediate_store.issue(IssueCertificateRequest::new("web", "example.com")).await.unwrap();

        assert!(issued.certificate.contains("BEGIN CERTIFICATE"));
        assert!(issued.private_key.is_some());
    }
}
