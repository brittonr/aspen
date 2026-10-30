pub fn command(operation: &str, root: &crate::node_state::DirectoryRoot) -> crate::error::Result<serde_json::Value> {
    let authorization = super::access::Authorization::load(root)?;
    authorization.require(operation)?;
    let source = crate::node_daemon::admit_startup_source_gate_with_root(root)?;
    let (configuration, group) = super::decoder::Configuration::load(&authorization, &source)?;
    match operation {
        "init" => super::control::initialize(&authorization, &source, &group),
        "serve" => {
            let path = crate::node_state::RelativePath::parse("coordination-bootstrap.preserves")?;
            if !root.try_exists(&path)? {
                return Err(crate::error::Failure::invalid_harness("coordination init must complete before serve"));
            }
            super::control::check_bootstrap(&authorization)?;
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))?;
            runtime.block_on(async { configuration.assemble(authorization, source, group).await?.serve().await })?;
            Ok(
                serde_json::json!({"schema":"molten.coordination-stopped.v1", "stopped":true,"production_admitted":false}),
            )
        }
        "status" => status(&authorization, &configuration),
        "stop" => super::control::stop(&authorization),
        _ => Err(crate::error::Failure::invalid_harness("unsupported coordination lifecycle command")),
    }
}

fn status(
    authorization: &super::access::Authorization,
    configuration: &super::decoder::Configuration,
) -> crate::error::Result<serde_json::Value> {
    authorization.require("probe")?;
    let current = super::super::readiness::current::read(&authorization.root, &configuration.health_path)?;
    let challenge = super::super::authority::Challenge {
        nonce: iroh::SecretKey::generate().to_bytes(),
        node_id: configuration.node_id.clone(),
        endpoint: configuration.admission.endpoint.to_string(),
        source_binding_ref: current.source_binding_ref,
        startup_ref: current.startup_ref,
        health_ref: current.health_ref,
    };
    let request = super::super::protocol::Request {
        schema: super::super::protocol::SCHEMA.into(),
        phase: super::super::protocol::Phase::Probe,
        run_id: configuration.admission.run_id.clone(),
        endpoint: configuration.admission.endpoint.to_string(),
        operation: None,
        deadline_ms: configuration.admission.profile.recovery.timeout_ms.min(120_000),
        profile: Some(configuration.admission.profile.clone()),
        probe: Some(challenge.clone()),
        authorization: None,
    };
    let response = super::super::client::execute(&request);
    if response.outcome != super::super::protocol::Outcome::Ok {
        return Err(crate::error::Failure::invalid_harness("live coordination status probe unavailable"));
    }
    let signed = serde_json::from_value(response.value)
        .map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))?;
    let state = super::super::authority::verify_probe(&authorization.grant, &signed, &challenge)?;
    authorization.grant.verify_current(&authorization.root)?;
    if !state.ready {
        return Err(crate::error::Failure::invalid_harness("coordination host is not quorum-ready"));
    }
    serde_json::to_value(signed).map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))
}
