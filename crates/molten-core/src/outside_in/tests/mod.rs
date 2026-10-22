use super::*;

mod judge;
mod validate;

const FIRST_REQUEST: &str = "blake3:1111111111111111111111111111111111111111111111111111111111111111";
const SECOND_REQUEST: &str = "blake3:2222222222222222222222222222222222222222222222222222222222222222";
const FIRST_ENVELOPE: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SECOND_ENVELOPE: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const ENDPOINT: &str = "iroh:0101010101010101010101010101010101010101010101010101010101010101";
const OTHER_ENDPOINT: &str = "iroh:0202020202020202020202020202020202020202020202020202020202020202";
const WALL_CLOCK_MS: u64 = 60_000;
const HOLD_MS: u64 = 500;
const STABILIZATION_MS: u64 = 1_000;

fn budgets() -> ScenarioBudgets {
    ScenarioBudgets {
        max_processes: 3,
        wall_clock_ms: WALL_CLOCK_MS,
        max_retained_output_bytes: 64 * 1024,
        max_client_operations: 4,
    }
}

fn declaration(profile: FaultProfile, expected: &[&str]) -> ScenarioDeclaration {
    let is_held = matches!(profile, FaultProfile::PauseResume | FaultProfile::PartitionHeal);
    ScenarioDeclaration {
        scenario_id: format!("outside-in-{}", profile.as_str()),
        path: ScenarioPath::NodeControl,
        profile,
        boundary: profile.boundary(),
        budgets: budgets(),
        fault_hold_ms: if is_held { HOLD_MS } else { 0 },
        stabilization_ms: STABILIZATION_MS,
        expected_processed: expected.iter().map(ToString::to_string).collect(),
    }
}

fn attempt(request: &str, fate: ResponseFate, outcome: ClientOutcome) -> ClientAttempt {
    ClientAttempt {
        request_ref: request.to_string(),
        fate,
        outcome,
    }
}

fn acknowledged(request: &str) -> ClientAttempt {
    attempt(request, ResponseFate::Observed, ClientOutcome::Acknowledged)
}

fn delivered(envelope: &str, request: &str) -> ObservedDelivery {
    ObservedDelivery {
        envelope_ref: envelope.to_string(),
        request_ref: request.to_string(),
        is_accepted: true,
    }
}

fn processed(request: &str) -> ObservedProcessed {
    ObservedProcessed {
        request_ref: request.to_string(),
        is_passed: true,
    }
}

fn history(deliveries: Vec<ObservedDelivery>, processed: Vec<ObservedProcessed>) -> ReceiverHistory {
    ReceiverHistory {
        deliveries,
        pending_requests: Vec::new(),
        processed,
    }
}

fn final_read(receiver: ReceiverHistory) -> HistoryCheckpoint {
    HistoryCheckpoint {
        kind: CheckpointKind::Final,
        receiver: Some(receiver),
    }
}

/// One request sent once, delivered once, and processed once.
fn agreeing_history(request: &str, envelope: &str) -> ObservedHistory {
    ObservedHistory {
        attempts: vec![acknowledged(request)],
        checkpoints: vec![final_read(history(vec![delivered(envelope, request)], vec![
            processed(request),
        ]))],
        listener_endpoints: vec![ENDPOINT.to_string()],
    }
}

fn judge(declaration: &ScenarioDeclaration, observed: ObservedHistory) -> ScenarioResult {
    judge_scenario(declaration, &ScenarioExecution::Completed(observed), TeardownStatus::Complete)
}
