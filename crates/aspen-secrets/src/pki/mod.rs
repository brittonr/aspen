//! PKI secrets engine (certificate authority).
//!
//! Provides X.509 certificate management:
//! - CA initialization (root or intermediate)
//! - Role-based certificate issuance policies
//! - Certificate revocation
//! - CRL management
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use aspen_secrets::pki::{PkiStore, DefaultPkiStore, GenerateRootRequest, CreateRoleRequest, IssueCertificateRequest};
//! use aspen_secrets::backend::InMemorySecretsBackend;
//! use std::sync::Arc;
//!
//! // Create a store
//! let backend = Arc::new(InMemorySecretsBackend::new());
//! let store = DefaultPkiStore::new(backend);
//!
//! // Generate root CA
//! store.generate_root(GenerateRootRequest::new("My Root CA")).await?;
//!
//! // Create a role for issuing certificates
//! store.create_role(
//!     CreateRoleRequest::new("web-servers")
//!         .with_allowed_domains(vec!["example.com".into()])
//!         .allow_subdomains()
//!         .allow_bare_domains()
//! ).await?;
//!
//! // Issue a certificate
//! let cert = store.issue(
//!     IssueCertificateRequest::new("web-servers", "www.example.com")
//! ).await?;
//! ```

mod store;
mod types;

pub use store::DefaultPkiStore;
pub use store::PkiStore;
pub use types::CertificateAuthority;
pub use types::CreateRoleRequest;
pub use types::CrlEntry;
pub use types::CrlState;
pub use types::GenerateIntermediateRequest;
pub use types::GenerateIntermediateResponse;
pub use types::GenerateRootRequest;
pub use types::GenerateRootResponse;
pub use types::IssueCertificateRequest;
pub use types::IssueCertificateResponse;
pub use types::IssuedCertificate;
pub use types::PkiConfig;
pub use types::PkiKeyType;
pub use types::PkiRole;
pub use types::ReadCertificateRequest;
pub use types::RevokeCertificateRequest;
pub use types::SetSignedIntermediateRequest;
