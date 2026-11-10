//! Closed local service-host protocol. Transport framing never grants authority.

pub(in super::super) const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

pub(super) struct Request {
    pub(super) mode: &'static str,
    pub(super) command: &'static str,
    pub(super) tick: u64,
    pub(super) operation: Action,
}

pub(super) enum Action {
    Host(molten::node_service_host::operation::Invocation),
    ScopeObserve {
        service: String,
        read: molten::collaboration_scope::RequestRef,
    },
}

impl Request {
    pub(super) fn decode(frame: &[u8]) -> Result<Self, &'static str> {
        if frame.len() > MAX_FRAME_BYTES {
            return Err("control frame exceeds byte bound");
        }
        let text = std::str::from_utf8(frame).map_err(|_| "control frame requires UTF-8")?;
        let value = molten::preserves_rail::parse_text(text).map_err(|_| "invalid control frame")?;
        let fields = molten::preserves_rail::simple_record_fields(&value, "node-service-host-control-v1", 4)
            .map_err(|_| "control frame requires exactly four fields")?;
        let mode = match fields[0].as_string().as_deref() {
            Some("preview") => "preview",
            Some("execute") => "execute",
            _ => return Err("unknown control mode"),
        };
        let command = match fields[1].as_string().as_deref() {
            Some("install") => "install",
            Some("status") => "status",
            Some("supervise") => "supervise",
            Some("submit") => "submit",
            Some("readmit") => "readmit",
            Some("upgrade") => "upgrade",
            Some("reconcile") => "reconcile",
            Some("deliver") => "deliver",
            Some("scope-observe") => "scope-observe",
            _ => return Err("unknown service host command"),
        };
        let tick = fields[2]
            .as_u64()
            .ok_or("control tick requires u64")?
            .map_err(|_| "control tick outside u64 range")?;
        let value = molten::preserves_rail::value_to_iovalue(&fields[3]);
        let operation = molten::preserves_rail::canonical_bytes(&value).map_err(|_| "invalid control operation")?;
        // Both branches preserve the packed operation bound, independently of framing.
        if operation.len() > molten::node_service_host::operation::MAX_OPERATION_BYTES {
            return Err("invalid or oversized service host operation");
        }
        let operation = if command == "scope-observe" {
            let fields = molten::preserves_rail::simple_record_fields(&value, "node-service-host-scope-observe-v1", 2)
                .map_err(|_| "invalid scope observation operation")?;
            let service = fields[0].as_string().ok_or("invalid scope observation service")?;
            if service.is_empty() || service.len() > 1024 {
                return Err("invalid scope observation service");
            }
            let read = fields[1].as_string().ok_or("invalid scope observation read")?;
            molten::preserves_rail::ContentRef::parse(&read).map_err(|_| "invalid scope observation read")?;
            let read =
                molten::collaboration_scope::RequestRef::new(read).map_err(|_| "invalid scope observation read")?;
            Action::ScopeObserve {
                service: service.into_owned(),
                read,
            }
        } else {
            Action::Host(
                molten::node_service_host::operation::decode(&operation, command, tick)
                    .map_err(|_| "invalid or oversized service host operation")?,
            )
        };
        Ok(Self {
            mode,
            command,
            tick,
            operation,
        })
    }
}

pub(super) fn failure_detail(error: &molten::error::Failure) -> &'static str {
    // Only exact existing public diagnostics known not to contain operator material are disclosed.
    match error {
        molten::error::Failure::InvalidHarness(message) => match message.as_str() {
            "service host current authority denied" => {
                "invalid harness artifact: service host current authority denied"
            }
            "service host concrete effect authority denied" => {
                "invalid harness artifact: service host concrete effect authority denied"
            }
            "native lifecycle intent remains uncertain; callback is not repeated" => {
                "invalid harness artifact: native lifecycle intent remains uncertain; callback is not repeated"
            }
            "scope effect runtime is stale, uncertain, or bound to another native service" => {
                "invalid harness artifact: scope effect runtime is stale, uncertain, or bound to another native service"
            }
            "hosting logical time regressed" => "invalid harness artifact: hosting logical time regressed",
            _ => scope_failure_detail(message)
                .unwrap_or("operation failed; outcome may be uncertain; inspect durable state before retry"),
        },
        _ => "operation failed; outcome may be uncertain; inspect durable state before retry",
    }
}

fn scope_failure_detail(message: &str) -> Option<&'static str> {
    let code = message.strip_prefix(
        "native hosting lifecycle failed or remains uncertain: Host(\"invalid harness artifact: malformed observation: ",
    ).and_then(|message| message.strip_suffix("\")")).unwrap_or(message);
    [
        "collaboration-scope/bound",
        "collaboration-scope/domain",
        "collaboration-scope/missing",
        "collaboration-scope/stale",
        "collaboration-scope/expired",
        "collaboration-scope/membership",
        "collaboration-scope/authority",
        "collaboration-scope/conflict",
        "collaboration-scope/owner",
        "collaboration-scope/replay",
        "collaboration-scope/uncertain",
        "collaboration-scope/partition",
    ]
    .into_iter()
    .find(|known| *known == code)
}

pub(in super::super) fn error(command: &str, tick: u64, detail: &str) -> preserves::IOValue {
    molten::preserves_rail::record("node-service-host-control-error-v1", vec![
        molten::preserves_rail::string(command),
        molten::preserves_rail::u64_value(tick),
        molten::preserves_rail::string(detail),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(mode: &str, command: &str, tick: &str, operation: &str) -> Vec<u8> {
        format!("<node-service-host-control-v1 \"{mode}\" \"{command}\" {tick} {operation}>").into_bytes()
    }

    #[test]
    fn admits_exact_preview_and_execute_with_unsigned_tick_boundary() -> Result<(), Box<dyn std::error::Error>> {
        for mode in ["preview", "execute"] {
            let parsed =
                Request::decode(&request(mode, "status", "18446744073709551615", "<node-service-host-status-v1>"))?;
            assert_eq!(parsed.tick, u64::MAX);
            assert!(matches!(parsed.operation, Action::Host(molten::node_service_host::operation::Invocation::Status)));
        }
        Ok(())
    }

    #[test]
    fn denies_open_modes_commands_tick_types_and_mismatched_operation() {
        for frame in [
            request("retry", "status", "0", "<node-service-host-status-v1>"),
            request("execute", "unknown", "0", "<node-service-host-status-v1>"),
            request("execute", "status", "-1", "<node-service-host-status-v1>"),
            request("execute", "status", "18446744073709551616", "<node-service-host-status-v1>"),
            request("execute", "status", "\"0\"", "<node-service-host-status-v1>"),
            request("execute", "status", "0", "<node-service-host-reconcile-v1 \"member\">"),
            b"<node-service-host-control-v1 \"preview\" \"status\" 0>".to_vec(),
            b"<node-service-host-control-v1 \"preview\" \"status\" 0 <node-service-host-status-v1> 1>".to_vec(),
            vec![0xff],
        ] {
            assert!(Request::decode(&frame).is_err());
        }
    }

    #[test]
    fn arbitrary_backend_details_are_not_disclosed() {
        let first = molten::error::Failure::invalid_harness("service host current authority denied: private-token");
        let second = molten::error::Failure::invalid_harness("private-payload");
        assert_eq!(failure_detail(&first), failure_detail(&second));
        assert!(!failure_detail(&first).contains("private-token"));

        assert!(!failure_detail(&second).contains("private-payload"));
    }
    #[test]
    fn scope_observation_is_typed_and_rejects_authority_or_schema_substitution()
    -> Result<(), Box<dyn std::error::Error>> {
        let read = molten::preserves_rail::content_ref_from_bytes(b"scope-diagnostic-read");
        let operation = format!("<node-service-host-scope-observe-v1 \"service\" \"{read}\">");
        for mode in ["preview", "execute"] {
            let parsed = Request::decode(&request(mode, "scope-observe", "7", &operation))?;
            match parsed.operation {
                Action::ScopeObserve { service, read: actual } => {
                    assert_eq!(service, "service");
                    assert_eq!(actual.as_str(), read);
                }
                Action::Host(_) => panic!("metadata request became a host action"),
            }
        }
        for operation in [
            "<node-service-host-status-v1>".to_string(),
            "<node-service-host-scope-observe-v1 \"service\" \"bad-ref\">".to_string(),
            format!("<node-service-host-scope-observe-v1 \"\" \"{read}\">"),
            format!("<node-service-host-scope-observe-v1 \"service\" \"{read}\" \"grant\">"),
            format!("<node-service-host-scope-observe-v1 \"service\" {read}>"),
        ] {
            assert!(Request::decode(&request("execute", "scope-observe", "7", &operation)).is_err());
        }
        assert!(Request::decode(&request("execute", "deliver", "7", &operation)).is_err());
        Ok(())
    }

    #[test]
    fn denies_frame_and_packed_operation_overflow_independently() {
        let oversized = vec![b' '; MAX_FRAME_BYTES + 1];
        assert!(Request::decode(&oversized).is_err());
        let operation = format!(
            "<node-service-host-install-v1 \"{}\">",
            "a".repeat(molten::node_service_host::operation::MAX_OPERATION_BYTES)
        );
        assert!(Request::decode(&request("execute", "install", "0", &operation)).is_err());
        let read = molten::preserves_rail::content_ref_from_bytes(b"read");
        let operation = format!(
            "<node-service-host-scope-observe-v1 \"{}\" \"{read}\">",
            "a".repeat(molten::node_service_host::operation::MAX_OPERATION_BYTES)
        );
        assert!(Request::decode(&request("execute", "scope-observe", "0", &operation)).is_err());
    }

    #[test]
    fn canonical_denial_codes_never_disclose_appended_material() {
        let wrap = |code| {
            molten::error::Failure::invalid_harness(format!(
                "native hosting lifecycle failed or remains uncertain: Host(\"invalid harness artifact: malformed observation: {code}\")"
            ))
        };
        assert_eq!(failure_detail(&wrap("collaboration-scope/stale")), "collaboration-scope/stale");
        let opaque = molten::error::Failure::invalid_harness("unknown backend failure");
        assert_eq!(failure_detail(&wrap("collaboration-scope/stale/private-token")), failure_detail(&opaque));
        let diagnostic = molten::error::Failure::invalid_harness("collaboration-scope/stale");
        assert_eq!(failure_detail(&diagnostic), "collaboration-scope/stale");
        let injected = molten::error::Failure::invalid_harness("collaboration-scope/stale/private-token");
        assert_eq!(failure_detail(&injected), failure_detail(&opaque));
    }
}
