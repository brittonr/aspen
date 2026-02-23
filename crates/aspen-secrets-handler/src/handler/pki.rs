//! PKI handler functions.
//!
//! Handles certificate authority operations (issue cert, list certs, revoke).

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::SecretsPkiCertificateResultResponse;
use aspen_client_api::SecretsPkiCrlResultResponse;
use aspen_client_api::SecretsPkiListResultResponse;
use aspen_client_api::SecretsPkiRevokeResultResponse;
use aspen_client_api::SecretsPkiRoleConfig;
use aspen_client_api::SecretsPkiRoleResultResponse;
use aspen_secrets::pki::CreateRoleRequest;
use aspen_secrets::pki::GenerateIntermediateRequest;
use aspen_secrets::pki::GenerateRootRequest;
use aspen_secrets::pki::IssueCertificateRequest;
use aspen_secrets::pki::PkiRole;
use aspen_secrets::pki::RevokeCertificateRequest;
use aspen_secrets::pki::SetSignedIntermediateRequest;
use tracing::debug;
use tracing::warn;

use super::SecretsService;
use super::sanitize_secrets_error;

/// Sub-handler for PKI secrets operations.
pub(crate) struct PkiSecretsHandler;

impl PkiSecretsHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::SecretsPkiGenerateRoot { .. }
                | ClientRpcRequest::SecretsPkiGenerateIntermediate { .. }
                | ClientRpcRequest::SecretsPkiSetSignedIntermediate { .. }
                | ClientRpcRequest::SecretsPkiCreateRole { .. }
                | ClientRpcRequest::SecretsPkiIssue { .. }
                | ClientRpcRequest::SecretsPkiRevoke { .. }
                | ClientRpcRequest::SecretsPkiGetCrl { .. }
                | ClientRpcRequest::SecretsPkiListCerts { .. }
                | ClientRpcRequest::SecretsPkiGetRole { .. }
                | ClientRpcRequest::SecretsPkiListRoles { .. }
        )
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        service: &SecretsService,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::SecretsPkiGenerateRoot {
                mount,
                common_name,
                ttl_days,
            } => handle_pki_generate_root(service, &mount, common_name, ttl_days).await,
            ClientRpcRequest::SecretsPkiGenerateIntermediate { mount, common_name } => {
                handle_pki_generate_intermediate(service, &mount, common_name).await
            }
            ClientRpcRequest::SecretsPkiSetSignedIntermediate { mount, certificate } => {
                handle_pki_set_signed_intermediate(service, &mount, certificate).await
            }
            ClientRpcRequest::SecretsPkiCreateRole {
                mount,
                name,
                allowed_domains,
                max_ttl_days,
                allow_bare_domains,
                allow_wildcards,
                allow_subdomains,
            } => {
                handle_pki_create_role(
                    service,
                    &mount,
                    name,
                    allowed_domains,
                    max_ttl_days,
                    allow_bare_domains,
                    allow_wildcards,
                    allow_subdomains,
                )
                .await
            }
            ClientRpcRequest::SecretsPkiIssue {
                mount,
                role,
                common_name,
                alt_names,
                ttl_days,
            } => handle_pki_issue(service, &mount, role, common_name, alt_names, ttl_days).await,
            ClientRpcRequest::SecretsPkiRevoke { mount, serial } => handle_pki_revoke(service, &mount, serial).await,
            ClientRpcRequest::SecretsPkiGetCrl { mount } => handle_pki_get_crl(service, &mount).await,
            ClientRpcRequest::SecretsPkiListCerts { mount } => handle_pki_list_certs(service, &mount).await,
            ClientRpcRequest::SecretsPkiGetRole { mount, name } => handle_pki_get_role(service, &mount, name).await,
            ClientRpcRequest::SecretsPkiListRoles { mount } => handle_pki_list_roles(service, &mount).await,
            _ => Err(anyhow::anyhow!("request not handled by PkiSecretsHandler")),
        }
    }
}

async fn handle_pki_generate_root(
    service: &SecretsService,
    mount: &str,
    common_name: String,
    ttl_days: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, common_name = %common_name, ttl_days = ?ttl_days, "PKI generate root request");

    let store = service.get_pki_store(mount).await?;
    let ttl_secs = ttl_days.map(|d| d as u64 * 24 * 3600).unwrap_or(10 * 365 * 24 * 3600);
    let request = GenerateRootRequest::new(common_name).with_ttl_secs(ttl_secs);

    match store.generate_root(request).await {
        Ok(response) => Ok(ClientRpcResponse::SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse {
            is_success: true,
            certificate: Some(response.certificate),
            private_key: None, // Root CA private key is stored internally
            serial: Some(response.serial),
            csr: None,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "PKI generate root failed");
            Ok(ClientRpcResponse::SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse {
                is_success: false,
                certificate: None,
                private_key: None,
                serial: None,
                csr: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_generate_intermediate(
    service: &SecretsService,
    mount: &str,
    common_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, common_name = %common_name, "PKI generate intermediate request");

    let store = service.get_pki_store(mount).await?;
    let request = GenerateIntermediateRequest::new(common_name);

    match store.generate_intermediate(request).await {
        Ok(response) => Ok(ClientRpcResponse::SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse {
            is_success: true,
            certificate: None,
            private_key: None,
            serial: None,
            csr: Some(response.csr),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "PKI generate intermediate failed");
            Ok(ClientRpcResponse::SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse {
                is_success: false,
                certificate: None,
                private_key: None,
                serial: None,
                csr: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_set_signed_intermediate(
    service: &SecretsService,
    mount: &str,
    certificate: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, "PKI set signed intermediate request");

    let store = service.get_pki_store(mount).await?;
    let request = SetSignedIntermediateRequest {
        certificate: certificate.clone(),
    };

    match store.set_signed_intermediate(request).await {
        Ok(()) => Ok(ClientRpcResponse::SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse {
            is_success: true,
            certificate: Some(certificate),
            private_key: None,
            serial: None,
            csr: None,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "PKI set signed intermediate failed");
            Ok(ClientRpcResponse::SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse {
                is_success: false,
                certificate: None,
                private_key: None,
                serial: None,
                csr: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_pki_create_role(
    service: &SecretsService,
    mount: &str,
    name: String,
    allowed_domains: Vec<String>,
    max_ttl_days: u32,
    allow_bare_domains: bool,
    allow_wildcards: bool,
    allow_subdomains: bool,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, name = %name, "PKI create role request");

    let store = service.get_pki_store(mount).await?;
    let mut role = PkiRole::new(name.clone());
    role.allowed_domains = allowed_domains.clone();
    role.max_ttl_secs = max_ttl_days as u64 * 24 * 3600;
    role.allow_bare_domains = allow_bare_domains;
    role.allow_wildcard_certificates = allow_wildcards;
    role.allow_subdomains = allow_subdomains;

    let request = CreateRoleRequest {
        name: name.clone(),
        config: role,
    };

    match store.create_role(request).await {
        Ok(created_role) => Ok(ClientRpcResponse::SecretsPkiRoleResult(SecretsPkiRoleResultResponse {
            is_success: true,
            role: Some(SecretsPkiRoleConfig {
                name: created_role.name,
                allowed_domains: created_role.allowed_domains,
                max_ttl_days: (created_role.max_ttl_secs / (24 * 3600)) as u32,
                allow_bare_domains: created_role.allow_bare_domains,
                allow_wildcards: created_role.allow_wildcard_certificates,
            }),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "PKI create role failed");
            Ok(ClientRpcResponse::SecretsPkiRoleResult(SecretsPkiRoleResultResponse {
                is_success: false,
                role: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_issue(
    service: &SecretsService,
    mount: &str,
    role: String,
    common_name: String,
    alt_names: Vec<String>,
    ttl_days: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, role = %role, common_name = %common_name, "PKI issue request");

    let store = service.get_pki_store(mount).await?;
    let request = IssueCertificateRequest {
        role,
        common_name,
        alt_names,
        ip_sans: vec![],
        uri_sans: vec![],
        ttl_secs: ttl_days.map(|d| d as u64 * 24 * 3600),
        exclude_cn_from_sans: false,
    };

    match store.issue(request).await {
        Ok(response) => Ok(ClientRpcResponse::SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse {
            is_success: true,
            certificate: Some(response.certificate),
            private_key: response.private_key,
            serial: Some(response.serial),
            csr: None,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "PKI issue failed");
            Ok(ClientRpcResponse::SecretsPkiCertificateResult(SecretsPkiCertificateResultResponse {
                is_success: false,
                certificate: None,
                private_key: None,
                serial: None,
                csr: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_revoke(service: &SecretsService, mount: &str, serial: String) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, serial = %serial, "PKI revoke request");

    let store = service.get_pki_store(mount).await?;
    let request = RevokeCertificateRequest { serial: serial.clone() };

    match store.revoke(request).await {
        Ok(()) => Ok(ClientRpcResponse::SecretsPkiRevokeResult(SecretsPkiRevokeResultResponse {
            is_success: true,
            serial: Some(serial),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "PKI revoke failed");
            Ok(ClientRpcResponse::SecretsPkiRevokeResult(SecretsPkiRevokeResultResponse {
                is_success: false,
                serial: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_get_crl(service: &SecretsService, mount: &str) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, "PKI get CRL request");

    let store = service.get_pki_store(mount).await?;
    match store.get_crl().await {
        Ok(crl_state) => {
            // Convert CRL state to PEM format
            let crl_pem = format!(
                "# CRL with {} entries, last updated: {}, next update: {}",
                crl_state.entries.len(),
                crl_state.last_update_unix_ms,
                crl_state.next_update_unix_ms
            );
            Ok(ClientRpcResponse::SecretsPkiCrlResult(SecretsPkiCrlResultResponse {
                is_success: true,
                crl: Some(crl_pem),
                error: None,
            }))
        }
        Err(e) => {
            warn!(error = %e, "PKI get CRL failed");
            Ok(ClientRpcResponse::SecretsPkiCrlResult(SecretsPkiCrlResultResponse {
                is_success: false,
                crl: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_list_certs(service: &SecretsService, mount: &str) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, "PKI list certs request");

    let store = service.get_pki_store(mount).await?;
    match store.list_certificates().await {
        Ok(serials) => Ok(ClientRpcResponse::SecretsPkiListResult(SecretsPkiListResultResponse {
            is_success: true,
            items: serials,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "PKI list certs failed");
            Ok(ClientRpcResponse::SecretsPkiListResult(SecretsPkiListResultResponse {
                is_success: false,
                items: vec![],
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_get_role(service: &SecretsService, mount: &str, name: String) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, name = %name, "PKI get role request");

    let store = service.get_pki_store(mount).await?;
    match store.read_role(&name).await {
        Ok(Some(role)) => Ok(ClientRpcResponse::SecretsPkiRoleResult(SecretsPkiRoleResultResponse {
            is_success: true,
            role: Some(SecretsPkiRoleConfig {
                name: role.name,
                allowed_domains: role.allowed_domains,
                max_ttl_days: (role.max_ttl_secs / (24 * 3600)) as u32,
                allow_bare_domains: role.allow_bare_domains,
                allow_wildcards: role.allow_wildcard_certificates,
            }),
            error: None,
        })),
        Ok(None) => Ok(ClientRpcResponse::SecretsPkiRoleResult(SecretsPkiRoleResultResponse {
            is_success: false,
            role: None,
            error: Some("Role not found".to_string()),
        })),
        Err(e) => {
            warn!(error = %e, "PKI get role failed");
            Ok(ClientRpcResponse::SecretsPkiRoleResult(SecretsPkiRoleResultResponse {
                is_success: false,
                role: None,
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}

async fn handle_pki_list_roles(service: &SecretsService, mount: &str) -> anyhow::Result<ClientRpcResponse> {
    debug!(mount = %mount, "PKI list roles request");

    let store = service.get_pki_store(mount).await?;
    match store.list_roles().await {
        Ok(roles) => Ok(ClientRpcResponse::SecretsPkiListResult(SecretsPkiListResultResponse {
            is_success: true,
            items: roles,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "PKI list roles failed");
            Ok(ClientRpcResponse::SecretsPkiListResult(SecretsPkiListResultResponse {
                is_success: false,
                items: vec![],
                error: Some(sanitize_secrets_error(&e)),
            }))
        }
    }
}
