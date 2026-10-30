//! wRPC unary transport over two independently admitted fabric frame sessions.
//! Echo acknowledgements are delivery evidence only; the reverse frame is the RPC result.
use std::sync;

use crate::codec;
use crate::fabric_transport;

pub struct Session {
    pub request: fabric_transport::ClientInput,
    pub result: fabric_transport::Listener,
    pub result_session: String,
    pub lease: super::authority::Lease,
    pub tick: u64,
    pub generation: u64,
    pub journal: super::journal::Journal,
}
#[derive(Clone)]
pub struct Client {
    session: sync::Arc<tokio::sync::Mutex<Option<Session>>>,
    observed: sync::Arc<tokio::sync::Mutex<Option<super::transcript::Transcript>>>,
}
impl Client {
    pub fn new(session: Session) -> super::Result<Self> {
        check_session(&session)?;
        Ok(Self {
            session: sync::Arc::new(tokio::sync::Mutex::new(Some(session))),
            observed: sync::Arc::new(tokio::sync::Mutex::new(None)),
        })
    }
    pub async fn transcript(&self) -> super::Result<super::transcript::Transcript> {
        self.observed.lock().await.clone().ok_or_else(|| "RPC did not complete".into())
    }
    pub(crate) async fn preflight_negatives(&self) -> super::Result<Vec<&'static str>> {
        let mut cases = Vec::new();
        for (interface, function, label) in [
            ("other:world/handler", super::FUNCTION, "wrong-world-before-effect"),
            (super::INTERFACE, "forbidden", "unauthorized-function-before-effect"),
        ] {
            let result = wrpc_transport::Invoke::invoke(
                self,
                (),
                interface,
                function,
                bytes::Bytes::new(),
                Vec::<Vec<Option<usize>>>::new(),
            )
            .await;
            if result.is_ok() || self.session.lock().await.is_none() {
                return Err(format!("pre-effect denial failed: {label}"));
            }
            cases.push(label);
        }
        let oversized = bytes::Bytes::from(vec![0; super::MAX_BYTES + 1]);
        let result = wrpc_transport::Invoke::invoke(
            self,
            (),
            super::INTERFACE,
            super::FUNCTION,
            oversized,
            Vec::<Vec<Option<usize>>>::new(),
        )
        .await;
        if result.is_ok() || self.session.lock().await.is_none() {
            return Err("oversized call consumed live session".into());
        }
        cases.push("oversized-invoke-before-effect");
        Ok(cases)
    }
}

fn check_session(session: &Session) -> super::Result<()> {
    session.lease.check(session.tick, session.generation)?;
    let scope = session.lease.scope();
    if session.request.session_ref != scope.session
        || session.request.request_ref != super::bridge::invocation(scope)?
        || session.request.endpoint.descriptor.expected_peer_context_ref != scope.peer
        || session.request.protocol.generation != scope.generation
        || session.result_session != super::profile::response_session(scope)?
        || endpoint_peer(&session.request.endpoint)? != scope.peer
    {
        return Err("transport peer/session/request differs from authorized scope".into());
    }
    codec::validate_content_ref(&session.result_session).map_err(super::failure)?;
    Ok(())
}

// r[impl molten.component_rpc.transport]
impl wrpc_transport::Invoke for Client {
    type Context = ();
    type Outgoing = super::channels::Writer;
    type Incoming = super::channels::Reader;
    async fn invoke<P>(
        &self,
        _: (),
        instance: &str,
        function: &str,
        params: bytes::Bytes,
        paths: impl AsRef<[P]> + Send,
    ) -> anyhow::Result<(Self::Outgoing, Self::Incoming)>
    where
        P: AsRef<[Option<usize>]> + Send + Sync,
    {
        super::bridge::admit_shape(instance, function, paths.as_ref().len()).map_err(anyhow::Error::msg)?;
        if !params.is_empty() {
            anyhow::bail!("hello arguments exceed the admitted zero-byte unary budget");
        }
        let session = self.session.lock().await.take().ok_or_else(|| anyhow::anyhow!("duplicate invocation"))?;
        check_session(&session).map_err(anyhow::Error::msg)?;
        let intent = session.journal.begin(&session.lease).map_err(anyhow::Error::msg)?;
        let Session {
            request,
            mut result,
            result_session,
            lease,
            journal,
            ..
        } = session;
        let attempted = invoke_once(request, &mut result, &lease, &result_session).await;
        journal
            .finish(&intent, attempted.as_ref().ok().map(|(transcript, _)| transcript))
            .map_err(anyhow::Error::msg)?;
        let cleanup = result.drain_and_close(fabric_transport::ListenerDrainReason::OperatorRequest).await;
        cleanup?;
        let (transcript, wire) =
            attempted.map_err(|error| anyhow::anyhow!("uncertain RPC attempt; reconcile before retry: {error}"))?;
        *self.observed.lock().await = Some(transcript);
        Ok((super::channels::Writer::default(), super::channels::Reader(std::io::Cursor::new(wire))))
    }
}

async fn invoke_once(
    request: fabric_transport::ClientInput,
    result: &mut fabric_transport::Listener,
    lease: &super::authority::Lease,
    result_session: &str,
) -> anyhow::Result<(super::transcript::Transcript, Vec<u8>)> {
    let scope = lease.scope();
    let invocation = super::bridge::invocation(scope).map_err(anyhow::Error::msg)?;
    let value = super::bridge::request(scope, &[]).map_err(anyhow::Error::msg)?;
    let bytes = codec::canonical_bytes(&value)?;
    let sent = fabric_transport::exchange_frame(request, &bytes, timeout()).await?;
    let received = result
        .accept_one_derived_frame(result_session, timeout(), |bytes| {
            let value = codec::parse_canonical_bytes(bytes)?;
            super::bridge::decode_envelope(&value, &invocation).map_err(crate::failures::Failure::invalid_harness)?;
            codec::canonical_hash(&value)
        })
        .await?;
    let value = codec::parse_canonical_bytes(&received.payload)?;
    let wire = super::bridge::decode_envelope(&value, &invocation).map_err(anyhow::Error::msg)?;
    let transcript =
        super::transcript::Transcript::complete(lease, &sent, &received.evidence, &wire).map_err(anyhow::Error::msg)?;
    Ok((transcript, wire))
}

pub struct Server<'a> {
    pub bundle: &'a super::materialization::Bundle,
    pub listener: &'a mut fabric_transport::Listener,
    pub response: fabric_transport::ClientInput,
    pub lease: super::authority::Lease,
    pub tick: u64,
    pub generation: u64,
    pub journal: &'a super::journal::Journal,
}

pub async fn serve_one(input: Server<'_>) -> super::Result<super::transcript::Transcript> {
    input.lease.check(input.tick, input.generation)?;
    input.bundle.check(input.lease.scope())?;
    let scope = input.lease.scope();
    if input.response.session_ref != super::profile::response_session(scope)?
        || input.response.protocol.generation != scope.generation
        || endpoint_peer(&input.response.endpoint)? != scope.caller_peer
    {
        return Err("reverse transport context differs from authorized scope".into());
    }
    let journal = input.journal;
    let intent = journal.begin(&input.lease)?;
    let result = serve_inner(input).await;
    journal.finish(&intent, result.as_ref().ok())?;
    result.map_err(|error| format!("uncertain RPC attempt; reconcile before retry: {error}"))
}

async fn serve_inner(input: Server<'_>) -> super::Result<super::transcript::Transcript> {
    let scope = input.lease.scope();
    let invocation = super::bridge::invocation(scope)?;
    let expected = codec::canonical_bytes(&super::bridge::request(scope, &[])?).map_err(super::failure)?;
    let received = input
        .listener
        .accept_one_derived_frame(&scope.session, timeout(), |bytes| {
            if bytes != expected {
                return Err(crate::failures::Failure::invalid_harness("wrong RPC request mapping"));
            }
            Ok(invocation.clone())
        })
        .await
        .map_err(super::failure)?;
    if received.evidence.remote_transport_identity_ref != scope.caller_peer {
        return Err("inbound transport peer is not the authorized caller".into());
    }
    let wire = super::runtime::call_server(input.bundle, Deny, &input.lease).await?;
    let result = super::bridge::result(&invocation, &wire)?;
    let result_ref = super::hash(&result)?;
    let mut response = input.response;
    response.request_ref = result_ref;
    let bytes = codec::canonical_bytes(&result).map_err(super::failure)?;
    let sent = fabric_transport::exchange_frame(response, &bytes, timeout()).await.map_err(super::failure)?;
    super::transcript::Transcript::complete(&input.lease, &received.evidence, &sent, &wire)
}

pub struct Deny;
impl wrpc_transport::Invoke for Deny {
    type Context = ();
    type Outgoing = super::channels::Writer;
    type Incoming = super::channels::Reader;
    async fn invoke<P>(
        &self,
        _: (),
        _: &str,
        _: &str,
        _: bytes::Bytes,
        _: impl AsRef<[P]> + Send,
    ) -> anyhow::Result<(Self::Outgoing, Self::Incoming)>
    where
        P: AsRef<[Option<usize>]> + Send + Sync,
    {
        anyhow::bail!("server outbound RPC denied")
    }
}
fn timeout() -> std::time::Duration {
    std::time::Duration::from_secs(super::profile::TIMEOUT_SECONDS)
}

fn endpoint_peer(endpoint: &fabric_transport::Endpoint) -> super::Result<String> {
    let identity = endpoint
        .descriptor
        .public_endpoint_identity
        .strip_prefix("iroh:")
        .ok_or("endpoint is not an Iroh identity")?;
    Ok(super::reference(identity.as_bytes()))
}
