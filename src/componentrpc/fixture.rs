//! Actual materialized client -> wRPC codec -> admitted Iroh frames -> materialized server.
//! Fixed keys/time are test inputs only; this API cannot be used as production admission.
use crate::fabric_transport;

pub struct Outcome {
    pub client_result: u32,
    pub client: super::transcript::Transcript,
    pub server: super::transcript::Transcript,
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
    let client_lease = super::fixtureauth::lease(&scope, 1)?;
    let server_lease = super::fixtureauth::lease(&scope, 2)?;
    let authority_cases = super::fixtureauth::negatives(&scope)?;
    let client_journal = super::journal::Journal::new(output, "client")?;
    let server_journal = super::journal::Journal::new(output, "server")?;
    let mut server = super::fixturefabric::listener(&server_lease, 21).await?;
    let result = super::fixturefabric::listener(&client_lease, 22).await?;
    let invocation = super::bridge::invocation(&scope)?;
    let response_session = super::profile::response_session(&scope)?;
    let request = super::fixturefabric::client(
        &client_lease,
        server.handoff().clone(),
        22,
        scope.session.clone(),
        invocation.clone(),
    )?;
    let response = super::fixturefabric::client(
        &server_lease,
        result.handoff().clone(),
        21,
        response_session.clone(),
        invocation,
    )?;
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
    let cleanup = server.drain_and_close(fabric_transport::ListenerDrainReason::OperatorRequest).await;
    cleanup.map_err(super::failure)?;
    let client_result = client_result?;
    let server = server_result?;
    let transcript = client.transcript().await?;
    let replay_result = super::transcript::replay(&scope, &transcript)?;
    if replay_result != "hello from Rust" || server.result_wire != transcript.result_wire {
        return Err("remote client/server/replay disagree".into());
    }
    let mut negative_cases = super::negative::verify(&transcript)?;
    negative_cases.extend(authority_cases);
    negative_cases.extend(invoke_cases);
    Ok(Outcome {
        client_result,
        client: transcript,
        server,
        replay_result,
        negative_cases,
    })
}
