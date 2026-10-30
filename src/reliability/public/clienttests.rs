fn request(endpoint: String, function: &str) -> super::protocol::Request {
    super::protocol::Request {
        schema: super::protocol::SCHEMA.into(),
        phase: super::protocol::Phase::Invoke,
        run_id: "campaign-a".into(),
        endpoint,
        deadline_ms: 500,
        profile: None,
        authorization: None,
        probe: None,
        operation: Some(serde_json::json!({"id":"logical-a","process":"client-a","function":function,
            "value":{"type":"integer","value":7},"object_key":"service-a"})),
    }
}

fn server(response: Option<serde_json::Value>) -> (String, std::thread::JoinHandle<()>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let endpoint = listener.local_addr().expect("address").to_string();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream.set_read_timeout(Some(std::time::Duration::from_secs(2))).expect("timeout");
        let mut line = String::new();
        std::io::BufRead::read_line(&mut std::io::BufReader::new(&mut stream), &mut line).expect("request");
        let request: super::protocol::Request = serde_json::from_str(&line).expect("wire request");
        if let Some(mut response) = response {
            response["endpoint"] = request.endpoint.into();
            let mut bytes = serde_json::to_vec(&response).expect("response");
            bytes.push(b'\n');
            std::io::Write::write_all(&mut stream, &bytes).expect("write response");
        }
    });
    (endpoint, handle)
}

fn response(value: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"schema":"onixos.live-client.v1","run_id":"campaign-a","endpoint":"replaced",
        "outcome":"ok","value":value,"pre_effect_denial":false,"operation_id":"logical-a"})
}

// r[verify molten.testing.live_reliability_validation]
#[test]
fn public_read_preserves_tagged_integer() {
    let (endpoint, handle) = server(Some(response(serde_json::json!({"type":"integer","value":7}))));
    let result = super::client::execute(&request(endpoint, "read"));
    handle.join().expect("server");
    assert_eq!(result.outcome, super::protocol::Outcome::Ok);
    assert_eq!(result.value, serde_json::json!({"type":"integer","value":7}));
}

// r[verify molten.consensus.live_operation_history]
#[test]
fn lost_ack_is_uncertain_not_definite_failure() {
    let (endpoint, handle) = server(None);
    let result = super::client::execute(&request(endpoint, "write"));
    handle.join().expect("server");
    assert_eq!(result.outcome, super::protocol::Outcome::Info);
    assert!(!result.pre_effect_denial);
}

// r[verify molten.testing.live_reliability_validation]
#[test]
fn changed_identity_malformed_value_and_false_failure_are_uncertain() {
    let mut changed = response(serde_json::json!({"type":"integer","value":7}));
    changed["run_id"] = "different-campaign".into();
    let mut changed_operation = response(serde_json::json!({"type":"integer","value":7}));
    changed_operation["operation_id"] = "different-logical-operation".into();
    let mut denied = response(serde_json::Value::Null);
    denied["outcome"] = "fail".into();
    for public_response in [
        changed,
        changed_operation,
        denied,
        response(serde_json::json!({"type":"string","value":"bad"})),
    ] {
        let (endpoint, handle) = server(Some(public_response));
        let result = super::client::execute(&request(endpoint, "read"));
        handle.join().expect("server");
        assert_eq!(result.outcome, super::protocol::Outcome::Info);
    }
}

// r[verify molten.consensus.live_operation_history]
#[test]
fn only_explicit_pre_effect_denial_maps_to_fail() {
    let mut denied = response(serde_json::Value::Null);
    denied["outcome"] = "fail".into();
    denied["pre_effect_denial"] = true.into();
    let (endpoint, handle) = server(Some(denied));
    let result = super::client::execute(&request(endpoint, "write"));
    handle.join().expect("server");
    assert_eq!(result.outcome, super::protocol::Outcome::Fail);
}
