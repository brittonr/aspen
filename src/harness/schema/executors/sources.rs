pub(super) fn steel_review_receipt_value(
    config: &super::model::SteelExecutorConfig,
) -> crate::error::Result<preserves::IOValue> {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    Ok(crate::preserves_rail::record("steel-review-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_STEEL_REVIEW_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("source-ref", vec![crate::preserves_rail::string(steel_source_ref(config)?)]),
        crate::preserves_rail::record("callable", vec![crate::preserves_rail::string(&config.callable)]),
        crate::preserves_rail::record("allowed-hostcalls", vec![crate::preserves_rail::sequence(
            allowed_hostcalls
                .iter()
                .map(|hostcall: &String| crate::preserves_rail::string(hostcall.as_str()))
                .collect::<Vec<_>>(),
        )]),
        super::calls::hostcall_checks_value(&[
            "source-ref-binding",
            "reviewed-callable",
            "allowed-hostcall-contract",
            "no-ambient-steel-io",
        ]),
    ]))
}

pub(super) fn adapter_preflight_receipt_value(
    config: &super::model::AdapterExecutorConfig,
) -> crate::error::Result<preserves::IOValue> {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    Ok(crate::preserves_rail::record("adapter-preflight-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_ADAPTER_PREFLIGHT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("manifest-ref", vec![crate::preserves_rail::string(adapter_manifest_ref(
            config,
        )?)]),
        crate::preserves_rail::record("abi-ref", vec![crate::preserves_rail::string(
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(&config.abi))?,
        )]),
        crate::preserves_rail::record("allowed-hostcalls", vec![crate::preserves_rail::sequence(
            allowed_hostcalls
                .iter()
                .map(|hostcall: &String| crate::preserves_rail::string(hostcall.as_str()))
                .collect::<Vec<_>>(),
        )]),
        crate::preserves_rail::record("transcript", vec![crate::preserves_rail::string(&config.transcript)]),
        super::calls::hostcall_checks_value(&[
            "manifest-ref-binding",
            "permission-binding",
            "deterministic-transcript",
            "no-ambient-adapter-io",
        ]),
    ]))
}

pub(super) fn remote_proxy_preflight_receipt_value(
    config: &super::model::RemoteProxyExecutorConfig,
) -> crate::error::Result<preserves::IOValue> {
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    Ok(crate::preserves_rail::record("remote-proxy-preflight-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_REMOTE_PROXY_PREFLIGHT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("peer-ref", vec![crate::preserves_rail::string(
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(&config.peer))?,
        )]),
        crate::preserves_rail::record("endpoint-ref", vec![crate::preserves_rail::string(remote_proxy_endpoint_ref(
            config,
        )?)]),
        crate::preserves_rail::record("contract-ref", vec![crate::preserves_rail::string(
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(&config.contract))?,
        )]),
        crate::preserves_rail::record("allowed-hostcalls", vec![crate::preserves_rail::sequence(
            allowed_hostcalls
                .iter()
                .map(|hostcall: &String| crate::preserves_rail::string(hostcall.as_str()))
                .collect::<Vec<_>>(),
        )]),
        crate::preserves_rail::record("transcript", vec![crate::preserves_rail::string(&config.transcript)]),
        super::calls::hostcall_checks_value(&[
            "peer-identity-binding",
            "endpoint-contract-binding",
            "verified-transcript",
            "transport-not-authority",
        ]),
    ]))
}

pub(super) fn validate_steel_executor_config(
    actor_id: &str,
    config: &super::model::SteelExecutorConfig,
) -> crate::error::Result<()> {
    if config.source.trim().is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Steel executor source for actor {actor_id} is empty"
        )));
    }
    if config.callable.trim().is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Steel executor callable for actor {actor_id} is empty"
        )));
    }
    for token in FORBIDDEN_STEEL_SOURCE_TOKENS {
        if config.source.contains(token) {
            return Err(crate::error::Failure::invalid_harness(format!(
                "Steel executor source for actor {actor_id} references forbidden ambient IO token {token}; reviewed Steel preflight remains fail-closed"
            )));
        }
    }
    Ok(())
}

const FORBIDDEN_STEEL_SOURCE_TOKENS: &[&str] = &[
    "open-input-file",
    "open-output-file",
    "call-with-input-file",
    "call-with-output-file",
    "delete-file",
    "read-file",
    "write-file",
    "system",
    "process",
    "current-seconds",
    "current-inexact-milliseconds",
    "random",
    "tcp",
    "udp",
    "http",
    "ffi",
];

pub fn steel_source_ref(config: &super::model::SteelExecutorConfig) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(&config.source))
}

pub(super) fn validate_adapter_executor_config(
    actor_id: &str,
    config: &super::model::AdapterExecutorConfig,
) -> crate::error::Result<()> {
    if config.manifest.trim().is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "adapter executor manifest for actor {actor_id} is empty"
        )));
    }
    if config.abi.trim().is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "adapter executor ABI for actor {actor_id} is empty"
        )));
    }
    for token in FORBIDDEN_ADAPTER_MANIFEST_TOKENS {
        if config.manifest.contains(token) || config.abi.contains(token) {
            return Err(crate::error::Failure::invalid_harness(format!(
                "adapter executor manifest for actor {actor_id} references forbidden ambient or stale token {token}"
            )));
        }
    }
    if config.transcript != "deterministic-local" && config.transcript != "verified" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "adapter executor transcript profile for actor {actor_id} must be deterministic-local or verified"
        )));
    }
    Ok(())
}

const FORBIDDEN_ADAPTER_MANIFEST_TOKENS: &[&str] =
    &["ambient-network", "ambient-fs", "process", "socket", "stale-signature"];

pub(super) fn validate_remote_proxy_executor_config(
    actor_id: &str,
    config: &super::model::RemoteProxyExecutorConfig,
) -> crate::error::Result<()> {
    if config.peer.trim().is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!("remote-proxy peer for actor {actor_id} is empty")));
    }
    if config.peer == "unknown" || config.peer.contains("revoked") {
        return Err(crate::error::Failure::invalid_harness(format!(
            "remote-proxy peer for actor {actor_id} cannot satisfy trusted deterministic gate evidence"
        )));
    }
    if config.endpoint.trim().is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "remote-proxy endpoint for actor {actor_id} is empty"
        )));
    }
    if !config.endpoint.starts_with("iroh:") {
        return Err(crate::error::Failure::invalid_harness(format!(
            "remote-proxy endpoint for actor {actor_id} must use an explicit iroh: transport profile"
        )));
    }
    if config.contract.trim().is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "remote-proxy contract for actor {actor_id} is empty"
        )));
    }
    if config.contract.contains("stale-signature") {
        return Err(crate::error::Failure::invalid_harness(format!(
            "remote-proxy contract for actor {actor_id} references stale signature evidence"
        )));
    }
    if config.transcript != "verified" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "remote-proxy transcript profile for actor {actor_id} must be verified before deterministic gates"
        )));
    }
    Ok(())
}

pub(super) fn adapter_manifest_ref(config: &super::model::AdapterExecutorConfig) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("adapter-manifest-v1", vec![
        crate::preserves_rail::string(&config.manifest),
        crate::preserves_rail::string(&config.abi),
    ]))
}

pub(super) fn remote_proxy_endpoint_ref(
    config: &super::model::RemoteProxyExecutorConfig,
) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("remote-proxy-endpoint-v1", vec![
        crate::preserves_rail::string(&config.peer),
        crate::preserves_rail::string(&config.endpoint),
        crate::preserves_rail::string(&config.contract),
    ]))
}
