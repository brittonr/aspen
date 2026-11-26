// Test peer identities, listeners and clients shared by the cross-process suites.

/// Peer id of the generic test client, which dials with `CLIENT_SECRET_BYTE`.
pub(crate) const CLIENT_PEER_ID: &str = "client";
/// Peer ids every default test listener admits. Each dials with its own derived test key.
pub(crate) const TEST_PEER_IDS: [&str; 4] = [CLIENT_PEER_ID, "node-a", "node-b", "node-c"];

/// The deterministic secret a test peer dials with: the generic client keeps its fixed byte key,
/// every other peer id derives its own so distinct voters authenticate as distinct endpoints.
pub(crate) fn peer_secret(peer_id: &str) -> [u8; IROH_SECRET_KEY_BYTES] {
    if peer_id == CLIENT_PEER_ID {
        return [CLIENT_SECRET_BYTE; IROH_SECRET_KEY_BYTES];
    }
    *blake3::hash(format!("molten-cross-process-test-peer:{peer_id}").as_bytes()).as_bytes()
}

fn peer_capability(peer_id: &str) -> EndpointCapability {
    EndpointCapability::from_secret_bytes(peer_secret(peer_id), CLIENT_CAPABILITY_REF.to_string())
        .expect("test peer capability")
}

/// The admitted-peer entry for one test peer id under the shared test peer context.
pub(crate) fn admitted_test_peer(peer_id: &str) -> AdmittedPeer {
    AdmittedPeer {
        public_endpoint_identity: peer_capability(peer_id).public_endpoint_identity(),
        peer_context_ref: PEER_CONTEXT_REF.to_string(),
    }
}

pub(crate) fn admitted_test_peers(peer_ids: &[&str]) -> std::collections::BTreeMap<String, AdmittedPeer> {
    peer_ids.iter().map(|peer_id| ((*peer_id).to_string(), admitted_test_peer(peer_id))).collect()
}

pub(crate) async fn listener() -> Listener {
    listener_with_secret(LISTENER_SECRET_BYTE).await
}

pub(crate) async fn listener_with_secret(secret_byte: u8) -> Listener {
    listener_admitting(secret_byte, admitted_test_peers(&TEST_PEER_IDS)).await
}

/// A listener keyed by the test peer `peer_id`'s own key, so its descriptor identity is the identity
/// that peer dials with, under `peer_context_ref`. It admits nobody until `Listener::admit_peer`.
pub(crate) async fn listener_for_peer(peer_id: &str, peer_context_ref: String) -> Listener {
    Listener::bind(ListenerInput {
        profile: profile(),
        protocol: protocol(),
        capability: peer_capability(peer_id),
        bind_addr: bind_addr(),
        listener_identity_ref: LISTENER_REF.to_string(),
        expected_peer_context_ref: peer_context_ref,
        locator_cohort_ref: LOCATOR_COHORT_REF.to_string(),
        disclosure: disclosure(),
        validity: validity(),
        admission: EndpointAdmissionState::fully_active(),
        observed_tick: OBSERVED_TICK,
        admitted_peers: std::collections::BTreeMap::new(),
    })
    .await
    .expect("per-peer cross-process listener")
}

pub(crate) async fn listener_admitting(
    secret_byte: u8,
    admitted_peers: std::collections::BTreeMap<String, AdmittedPeer>,
) -> Listener {
    Listener::bind(ListenerInput {
        profile: profile(),
        protocol: protocol(),
        capability: capability(secret_byte, LISTENER_CAPABILITY_REF),
        bind_addr: bind_addr(),
        listener_identity_ref: LISTENER_REF.to_string(),
        expected_peer_context_ref: PEER_CONTEXT_REF.to_string(),
        locator_cohort_ref: LOCATOR_COHORT_REF.to_string(),
        disclosure: disclosure(),
        validity: validity(),
        admission: EndpointAdmissionState::fully_active(),
        observed_tick: OBSERVED_TICK,
        admitted_peers,
    })
    .await
    .expect("cross-process listener")
}

pub(crate) fn client_input(endpoint: Endpoint) -> ClientInput {
    client_input_for(endpoint, CLIENT_PEER_ID)
}

/// A client input that dials `endpoint` authenticated as the test peer `peer_id`.
pub(crate) fn client_input_for(endpoint: Endpoint, peer_id: &str) -> ClientInput {
    ClientInput {
        profile: profile(),
        protocol: protocol(),
        capability: peer_capability(peer_id),
        bind_addr: bind_addr(),
        expected: expected(&endpoint),
        endpoint,
        admission: EndpointAdmissionState::fully_active(),
        session_ref: SESSION_REF.to_string(),
        request_ref: REQUEST_REF.to_string(),
    }
}
