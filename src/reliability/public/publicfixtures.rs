//! Deliberately faulty TCP peers: observations are real public client results, never a consensus
//! oracle.

#[derive(Debug, Clone, Copy)]
enum Fault {
    Stale,
    LostAcknowledgedWrite,
    Divergent,
    DelayedDuplicate,
    Stalled,
}

struct Register {
    current: super::generator::Value,
    previous: super::generator::Value,
    first: Option<super::generator::Value>,
    stale_returned: bool,
}

fn peer(fault: Fault) -> (Vec<String>, std::thread::JoinHandle<()>) {
    let listeners: Vec<_> = (0..2).map(|_| std::net::TcpListener::bind("127.0.0.1:0").expect("bind")).collect();
    let endpoints = listeners.iter().map(|l| l.local_addr().expect("address").to_string()).collect();
    for listener in &listeners {
        listener.set_nonblocking(true).expect("nonblocking");
    }
    let handle = std::thread::spawn(move || {
        let mut register = Register {
            current: super::generator::Value::Null,
            previous: super::generator::Value::Null,
            first: None,
            stale_returned: false,
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut requests = 0;
        while requests < 4 {
            assert!(std::time::Instant::now() < deadline, "fixture request bound");
            for (endpoint, listener) in listeners.iter().enumerate() {
                match listener.accept() {
                    Ok((stream, _)) => {
                        serve(stream, endpoint, fault, &mut register);
                        requests += 1;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(error) => panic!("fixture accept: {error}"),
                }
            }
            if requests < 4 {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    });
    (endpoints, handle)
}

fn serve(mut stream: std::net::TcpStream, endpoint: usize, fault: Fault, register: &mut Register) {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(2))).expect("read bound");
    stream.set_write_timeout(Some(std::time::Duration::from_secs(2))).expect("write bound");
    let mut line = String::new();
    std::io::BufRead::read_line(&mut std::io::BufReader::new(&mut stream), &mut line).expect("wire request");
    let request: super::protocol::Request = serde_json::from_str(&line).expect("request schema");
    let operation = request.operation.as_ref().expect("operation");
    let is_write = operation["function"] == "write";
    if !is_write && matches!(fault, Fault::Stalled) {
        // Hold the response, not a timer race: the client's admitted deadline closes the connection.
        let mut byte = [0];
        match std::io::Read::read(&mut stream, &mut byte) {
            Ok(_) => {}
            Err(error) => assert!(
                matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut | std::io::ErrorKind::ConnectionReset
                ),
                "fixture hold: {error}"
            ),
        }
        return;
    }
    let value = if is_write {
        let value = serde_json::from_value(operation["value"].clone()).expect("write value");
        register.first.get_or_insert(value);
        if !matches!(fault, Fault::LostAcknowledgedWrite) {
            register.previous = register.current;
            register.current = value;
        }
        serde_json::Value::Null
    } else {
        let observed = match fault {
            Fault::Stale if !register.stale_returned => {
                register.stale_returned = true;
                register.previous
            }
            Fault::Divergent if endpoint == 1 => super::generator::Value::Null,
            Fault::DelayedDuplicate => {
                register.current = register.first.expect("first acknowledged write");
                register.current
            }
            _ => register.current,
        };
        serde_json::to_value(observed).expect("read value")
    };
    let response = super::protocol::Response {
        schema: super::protocol::SCHEMA.into(),
        run_id: request.run_id,
        endpoint: request.endpoint,
        operation_id: operation["id"].as_str().map(String::from),
        attempt_id: operation["attempt_id"].as_str().map(String::from),
        outcome: super::protocol::Outcome::Ok,
        value,
        diagnostic: None,
        pre_effect_denial: false,
    };
    let mut bytes = serde_json::to_vec(&response).expect("wire response");
    bytes.push(b'\n');
    std::io::Write::write_all(&mut stream, &bytes).expect("response");
}

fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn profile(endpoints: Vec<String>) -> super::profile::Register {
    serde_json::from_value(serde_json::json!({"schema":"molten.live-register-profile.v1","group":"group",
        "service":"registry","keys":["key"],"initial_value":null,"values":[1,2],"generator_ref":reference("generator"),
        "adapter_ref":reference("adapter"),"model_ref":reference("model"),"outcome_ref":reference("outcome"),
        "seed":0,"read_weight":1,"write_weight":9,"clients":["public-client"],"endpoints":endpoints,
        "concurrency":1,"operations":4,"control_operations":2,"consistency_model":"register",
        "recovery":{"timeout_ms":1000,"stable_membership_observations":1,"require_all_endpoint_reads":true},
        "evidence":{"max_attempts_per_operation":1,"max_events":16,"max_uncertain":4,"max_bytes":1048576,
        "checker_states":1000,"checker_branches":1000,"checker_memo_bytes":4096,"checker_reductions":10}}))
    .expect("profile")
}

fn plan(endpoints: Vec<String>) -> (super::profile::Admitted, super::generator::OperationPlan) {
    let mut profile = profile(endpoints);
    for seed in 0..128 {
        profile.seed = seed;
        let admitted = super::profile::admit(&profile).expect("admitted profile");
        let plan = super::generator::generate(&admitted).expect("admitted plan");
        if plan.operations[..2].iter().all(|op| op.function == super::generator::Function::Write)
            && plan.operations[0].value != plan.operations[1].value
        {
            return (admitted, plan);
        }
    }
    panic!("bounded fixture seed search lacks distinct initial writes");
}

struct Observation {
    first_write: super::generator::Value,
    latest_write: super::generator::Value,
    history: super::history::Document,
}

fn observe(fault: Fault) -> Observation {
    let (endpoints, thread) = peer(fault);
    let (admitted, plan) = plan(endpoints);
    let mut events = Vec::with_capacity(plan.operations.len() * 2);
    for operation in &plan.operations {
        let ordinal = 0;
        let mut input = serde_json::to_value(operation).expect("planned operation");
        input["attempt_id"] = serde_json::json!(super::history::attempt_id(operation, ordinal));
        let request = super::protocol::Request {
            schema: super::protocol::SCHEMA.into(),
            phase: if operation.function == super::generator::Function::Read {
                super::protocol::Phase::FinalRead
            } else {
                super::protocol::Phase::Invoke
            },
            run_id: "fault-fixture".into(),
            endpoint: operation.endpoint.clone(),
            operation: Some(input),
            deadline_ms: 200,
            profile: Some(admitted.profile().clone()),
            authorization: None,
            probe: None,
        };
        let response = super::client::execute(&request);
        let outcome = match response.outcome {
            super::protocol::Outcome::Ok => super::history::PublicOutcome::Acknowledged(
                if operation.function == super::generator::Function::Write {
                    None
                } else {
                    Some(serde_json::from_value(response.value).expect("public tagged value"))
                },
            ),
            super::protocol::Outcome::Info => super::history::PublicOutcome::LostResponse,
            super::protocol::Outcome::Fail => super::history::PublicOutcome::PreEffectDenied,
        };
        let observation = super::history::AttemptObservation {
            ordinal,
            event_index: events.len() as u64,
            started_ns: events.len() as u64,
            completed_ns: events.len() as u64 + 1,
            source_artifact: reference("public-fixture-source"),
            outcome,
        };
        events.extend(
            super::history::project_attempt(&admitted, operation, &observation).expect("public history projection"),
        );
    }
    thread.join().expect("fault peer");
    let history = history(&admitted, events);
    super::history::validate(&admitted, &plan, &history).expect("complete public operation history");
    Observation {
        first_write: plan.operations[0].value,
        latest_write: plan.operations[1].value,
        history,
    }
}

fn history(admitted: &super::profile::Admitted, events: Vec<super::history::Event>) -> super::history::Document {
    let p = admitted.profile();
    super::history::Document {
        schema_version: 2,
        workload_profile: admitted.profile_ref().into(),
        source_artifact: reference("public-fixture-source"),
        profile: super::history::Profile {
            profile_id: admitted.profile_ref().into(),
            model: "register".into(),
            independent_key_decomposition: false,
            bounds: super::history::CheckerBounds {
                max_operations: 4,
                max_depth: 4,
                max_states: p.evidence.checker_states,
                max_branches: p.evidence.checker_branches,
                max_memo_bytes: p.evidence.checker_memo_bytes,
                max_reductions: p.evidence.checker_reductions,
            },
        },
        completeness: super::history::Completeness {
            invocations: 4,
            completions: 4,
            pending: 0,
            applied_faults: 0,
            observed_faults: 0,
        },
        events,
    }
}

fn completions(observation: &Observation) -> Vec<(super::history::Outcome, Option<super::generator::Value>)> {
    observation
        .history
        .events
        .iter()
        .filter_map(|event| match event {
            super::history::Event::Complete { outcome, output, .. } => Some((*outcome, *output)),
            _ => None,
        })
        .collect()
}

// r[verify molten.testing.live_reliability_validation]
#[test]
fn stale_cache_read_survives_public_transport_and_history_projection() {
    let observation = observe(Fault::Stale);
    let completions = completions(&observation);
    assert_eq!(completions[..2], [(super::history::Outcome::Ok, None); 2]);
    assert_eq!(completions[2], (super::history::Outcome::Ok, Some(observation.first_write)));
    assert_eq!(completions[3], (super::history::Outcome::Ok, Some(observation.latest_write)));
    assert_ne!(observation.first_write, observation.latest_write);
}

// r[verify molten.testing.live_reliability_validation]
#[test]
fn lost_acknowledged_writes_remain_acknowledged_before_empty_public_reads() {
    let observation = observe(Fault::LostAcknowledgedWrite);
    let completions = completions(&observation);
    assert_eq!(completions[..2], [(super::history::Outcome::Ok, None); 2]);
    assert_eq!(completions[2..], [(super::history::Outcome::Ok, Some(super::generator::Value::Null)); 2]);
}

// r[verify molten.testing.live_reliability_validation]
#[test]
fn divergent_endpoint_reads_remain_distinct_in_public_history() {
    let observation = observe(Fault::Divergent);
    let completions = completions(&observation);
    assert_eq!(completions[2], (super::history::Outcome::Ok, Some(observation.latest_write)));
    assert_eq!(completions[3], (super::history::Outcome::Ok, Some(super::generator::Value::Null)));
}

// r[verify molten.testing.live_reliability_validation]
#[test]
fn delayed_duplicate_old_write_is_observable_after_newer_acknowledgement() {
    let observation = observe(Fault::DelayedDuplicate);
    let completions = completions(&observation);
    assert_eq!(completions[..2], [(super::history::Outcome::Ok, None); 2]);
    assert_ne!(observation.first_write, observation.latest_write);
    assert_eq!(completions[2..], [(super::history::Outcome::Ok, Some(observation.first_write)); 2]);
}

// r[verify molten.consensus.live_recovery]
#[test]
fn stalled_recovery_final_reads_are_uncertain_not_success_or_definite_failure() {
    let observation = observe(Fault::Stalled);
    let completions = completions(&observation);
    assert_eq!(completions[..2], [(super::history::Outcome::Ok, None); 2]);
    assert_eq!(completions[2..], [(super::history::Outcome::Info, None); 2]);
}
