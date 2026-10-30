//! Public black-box client protocol. Transport failure never proves non-application.

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub phase: Phase,
    pub run_id: String,
    pub endpoint: String,
    pub operation: Option<serde_json::Value>,
    pub deadline_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<super::profile::Register>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization: Option<super::authority::SignedInvocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probe: Option<super::authority::Challenge>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    Setup,
    Invoke,
    Recover,
    FinalRead,
    Teardown,
    Probe,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub schema: String,
    pub run_id: String,
    pub endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
    pub outcome: Outcome,
    pub value: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<String>,
    #[serde(default)]
    pub pre_effect_denial: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Ok,
    Fail,
    Info,
}

pub const SCHEMA: &str = "onixos.live-client.v1";
pub const MAX_FRAME_BYTES: u64 = 1_048_576;

// r[impl molten.consensus.live_operation_history]
pub fn validate_response(request: &Request, response: Response) -> Result<Response, String> {
    if response.schema != SCHEMA || response.run_id != request.run_id || response.endpoint != request.endpoint {
        return Err("response-identity-mismatch".into());
    }
    if let Some(operation) = &request.operation {
        if response.operation_id.as_deref() != operation.get("id").and_then(|v| v.as_str()) {
            return Err("response-operation-identity-mismatch".into());
        }
        if let Some(attempt) = operation.get("attempt_id").and_then(|v| v.as_str())
            && response.attempt_id.as_deref() != Some(attempt)
        {
            return Err("response-attempt-identity-mismatch".into());
        }
    }
    if response.outcome == Outcome::Fail && !response.pre_effect_denial {
        return Err("false-definite-failure".into());
    }
    if response.outcome != Outcome::Fail && response.pre_effect_denial {
        return Err("contradictory-pre-effect-denial".into());
    }
    if response.outcome == Outcome::Ok && request.phase != Phase::Invoke && request.phase != Phase::FinalRead {
        return Ok(response);
    }
    if response.outcome == Outcome::Ok
        && request.operation.as_ref().and_then(|v| v.get("function")).and_then(|v| v.as_str()) == Some("read")
    {
        validate_value(&response.value)?;
        if let Some(profile) = &request.profile {
            validate_corpus(&response.value, profile, true)?;
        }
    }
    Ok(response)
}

pub fn validate_request(request: &Request) -> Result<std::net::SocketAddr, String> {
    if request.schema != SCHEMA || request.run_id.is_empty() || request.run_id.len() > 256 {
        return Err("invalid-request-identity".into());
    }
    if !(1..=120_000).contains(&request.deadline_ms) {
        return Err("invalid-deadline".into());
    }
    if let Some(profile) = &request.profile {
        super::profile::admit(profile).map_err(|issue| format!("profile-denied:{issue:?}"))?;
        if !profile.endpoints.contains(&request.endpoint) {
            return Err("unadmitted-endpoint".into());
        }
    }
    if request.phase == Phase::Invoke || request.phase == Phase::FinalRead {
        let operation = request.operation.as_ref().ok_or("missing-operation")?;
        for field in ["id", "process", "object_key"] {
            if operation.get(field).and_then(|v| v.as_str()).is_none_or(|v| v.is_empty() || v.len() > 256) {
                return Err(format!("invalid-operation-{field}"));
            }
        }
        match operation.get("function").and_then(|v| v.as_str()) {
            Some("read") => {}
            Some("write") if request.phase == Phase::Invoke => {
                validate_value(operation.get("value").ok_or("missing-value")?)?
            }
            _ => return Err("unsupported-operation".into()),
        }
        if let Some(profile) = &request.profile {
            if operation.get("object_key").and_then(|v| v.as_str()) != Some(profile.keys[0].as_str()) {
                return Err("unadmitted-object-key".into());
            }
            if !profile
                .clients
                .iter()
                .any(|client| operation.get("process").and_then(|v| v.as_str()) == Some(client.as_str()))
            {
                return Err("unadmitted-client".into());
            }
            if operation.get("function").and_then(|v| v.as_str()) == Some("write") {
                validate_corpus(operation.get("value").ok_or("missing-value")?, profile, false)?;
            }
        }
    }
    request.endpoint.parse().map_err(|_| "invalid-endpoint".into())
}

fn validate_value(value: &serde_json::Value) -> Result<(), String> {
    let object = value.as_object().ok_or("unsupported-value")?;
    match object.get("type").and_then(|v| v.as_str()) {
        Some("null") if object.len() == 1 => Ok(()),
        Some("integer") if object.len() == 2 && object.get("value").is_some_and(|v| v.as_i64().is_some()) => Ok(()),
        Some(_) | None => Err("unsupported-value".into()),
    }
}

fn validate_corpus(
    value: &serde_json::Value,
    profile: &super::profile::Register,
    allow_empty: bool,
) -> Result<(), String> {
    if allow_empty && value.get("type").and_then(|v| v.as_str()) == Some("null") {
        return Ok(());
    }
    if value.get("value").and_then(|v| v.as_i64()).is_some_and(|integer| profile.values.contains(&integer)) {
        return Ok(());
    }
    Err("unsupported-profile-value".into())
}

pub fn uncertain(request: &Request, diagnostic: String) -> Response {
    Response {
        schema: SCHEMA.into(),
        run_id: request.run_id.clone(),
        endpoint: request.endpoint.clone(),
        outcome: Outcome::Info,
        value: serde_json::Value::Null,
        diagnostic: Some(diagnostic),
        pre_effect_denial: false,
        operation_id: request.operation.as_ref().and_then(|v| v.get("id")).and_then(|v| v.as_str()).map(String::from),
        attempt_id: request
            .operation
            .as_ref()
            .and_then(|v| v.get("attempt_id"))
            .and_then(|v| v.as_str())
            .map(String::from),
    }
}

pub fn response(request: &Request, outcome: Outcome, value: serde_json::Value, diagnostic: Option<String>) -> Response {
    let mut response = uncertain(request, String::new());
    response.outcome = outcome;
    response.value = value;
    response.diagnostic = diagnostic;
    response.pre_effect_denial = outcome == Outcome::Fail;
    response
}
