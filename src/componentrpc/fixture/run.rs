//! Actual materialized client -> wRPC codec -> admitted Iroh frames -> materialized server.
//! Fixed keys/time are test inputs only; this API cannot be used as production admission.

pub struct Outcome {
    pub client_result: u32,
    pub client: super::transcript::Call,
    pub server: super::transcript::Call,
    pub replay_result: String,
    pub negative_cases: Vec<&'static str>,
}
fn scope(bundle: &super::materialization::Bundle) -> super::Result<super::bridge::Scope> {
    Ok(super::bridge::Scope {
        caller: super::reference(bundle.client()),
        callee: super::reference(bundle.server()),
        wit: bundle.wit().into(),
        bundle: bundle.reference().into(),
        policy: super::authority::policy_ref(super::fixtureauth::POLICY)?,
        resources: super::profile::resources()?,
        peer: super::fixturefabric::peer(21),
        caller_peer: super::fixturefabric::peer(22),
        session: super::reference(b"component-rpc-fixture-session-v1"),
        delivery: super::profile::delivery()?,
        dedup: super::reference(b"component-rpc-fixture-single-invocation-v1"),
        generation: 1,
        deadline_tick: 100,
    })
}

// r[impl molten.component_rpc.validation]
pub async fn run(bundle: &super::materialization::Bundle, output: &cap_std::fs::Dir) -> super::Result<Outcome> {
    let scope = scope(bundle)?;
    bundle.check(&scope)?;
    let component_case = unmeasured_callee(bundle, &scope)?;
    let client_lease = super::fixtureauth::lease(&scope, 1)?;
    let server_lease = super::fixtureauth::lease(&scope, 2)?;
    let authority_cases = super::fixtureauth::negatives(&scope)?;
    let client_journal = super::journal::Recorder::new(output, "client")?;
    let server_journal = super::journal::Recorder::new(output, "server")?;
    let mut server = super::fixturefabric::listener(&server_lease, 21, 22).await?;
    let result = super::fixturefabric::listener(&client_lease, 22, 21).await?;
    let drift_case = drifted_transport(&client_lease, server.handoff().clone(), &scope).await?;
    let (request, response, response_session) =
        sessions(&scope, &client_lease, &server_lease, server.handoff().clone(), result.handoff().clone())?;
    let client = super::transport::Client::new(super::transport::Session {
        request,
        result,
        result_session: response_session,
        lease: client_lease,
        tick: 10,
        generation: 1,
        journal: client_journal,
    })?;
    let invoke_cases = client.preflight_negatives().await?;
    let (client_result, server_result) = tokio::join!(
        super::runtime::run_client(bundle, client.clone(), &scope),
        super::transport::serve_one(super::transport::Server {
            bundle,
            listener: &mut server,
            response,
            lease: server_lease,
            tick: 10,
            generation: 1,
            journal: &server_journal
        })
    );
    let cleanup = server.drain_and_close(crate::fabric_transport::ListenerDrainReason::OperatorRequest).await;
    cleanup.map_err(super::failure)?;
    let client_result = client_result?;
    let server = server_result?;
    let transcript = client.transcript().await?;
    let retry_case = client.retry_negative().await?;
    let replay_result = super::transcript::replay(&scope, &transcript)?;
    if replay_result != "hello from Rust" || server.result_wire != transcript.result_wire {
        return Err("remote client/server/replay disagree".into());
    }
    let mut negative_cases = super::negative::verify(&transcript)?;
    negative_cases.extend(authority_cases);
    negative_cases.extend(invoke_cases);
    negative_cases.extend([component_case, drift_case, retry_case]);
    Ok(Outcome {
        client_result,
        client: transcript,
        server,
        replay_result,
        negative_cases,
    })
}

/// Only the exact measured server is admissible; another callee is denied before any effect.
fn unmeasured_callee(
    bundle: &super::materialization::Bundle,
    scope: &super::bridge::Scope,
) -> super::Result<&'static str> {
    let mut unmeasured = scope.clone();
    unmeasured.callee = super::reference(b"unmeasured-server-component");
    if bundle.check(&unmeasured).is_ok() {
        return Err("unmeasured callee component was admitted".into());
    }
    Ok("unmeasured-component-before-effect")
}

/// The admitted request session to the server and the reverse result session to the client.
fn sessions(
    scope: &super::bridge::Scope,
    client_lease: &super::authority::Lease,
    server_lease: &super::authority::Lease,
    server: crate::fabric_transport::Endpoint,
    result: crate::fabric_transport::Endpoint,
) -> super::Result<(crate::fabric_transport::ClientInput, crate::fabric_transport::ClientInput, String)> {
    let invocation = super::bridge::invocation(scope)?;
    let response_session = super::profile::response_session(scope)?;
    let request = super::fixturefabric::client(client_lease, server, 22, scope.session.clone(), invocation.clone())?;
    let response = super::fixturefabric::client(server_lease, result, 21, response_session.clone(), invocation)?;
    Ok((request, response, response_session))
}

/// A caller expecting another transport profile is denied at endpoint import, before any dial.
async fn drifted_transport(
    lease: &super::authority::Lease,
    endpoint: crate::fabric_transport::Endpoint,
    scope: &super::bridge::Scope,
) -> super::Result<&'static str> {
    let invocation = super::bridge::invocation(scope)?;
    let mut drifted = super::fixturefabric::client(lease, endpoint, 22, scope.session.clone(), invocation)?;
    drifted.expected.profile_ref = super::reference(b"undeclared-transport-profile");
    let request = crate::codec::canonical_bytes(&super::bridge::request(scope, &[])?).map_err(super::failure)?;
    let timeout = std::time::Duration::from_secs(super::profile::TIMEOUT_SECONDS);
    match crate::fabric_transport::exchange_frame(drifted, &request, timeout).await {
        Ok(_) => Err("drifted transport profile reached the listener".into()),
        Err(error) => {
            let error = error.to_string();
            if !error.contains("endpoint import denied") || !error.contains("ProfileIdentityMismatch") {
                return Err(format!("drifted transport profile was not denied at import admission: {error}"));
            }
            Ok("transport-profile-drift-before-session")
        }
    }
}
