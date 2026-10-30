use std::cell;
use std::rc;

use super::super::continuation;
use super::super::evidence;
use super::super::fixture;
use super::super::fixtureauthority;
use super::super::profile;

pub(super) struct Ready {
    pub bytes: Vec<u8>,
    pub input: Vec<u8>,
    pub source: rc::Rc<fixtureauthority::Authority>,
    pub binding: continuation::Binding,
    pub verified: evidence::Verified,
    pub contexts: Vec<continuation::ResumeContext>,
}

pub(super) fn prepare(request: &super::Request<'_>) -> Result<Ready, String> {
    validate_scenario(request.name, &request.scenario)?;
    request.profile.validate()?;
    let verified = evidence::verify(request.trust, &request.sources)?;
    let bytes = super::read(request.sources.root, request.sources.module, 65536)?;
    let input = super::read(request.sources.root, request.sources.input, 64512)?;
    let input_value = crate::codec::parse_canonical_bytes(&input).map_err(|e| e.to_string())?;
    let binding = continuation::Binding {
        actor: "fixture".into(),
        generation: 1,
        artifact: crate::codec::content_ref_from_bytes(&bytes),
        profile: request.profile.content_ref()?,
        input: crate::codec::canonical_hash(&input_value).map_err(|e| e.to_string())?,
        authority: crate::codec::content_ref_from_bytes(b"unissued-fixture-authority"),
        resource: resource(&request.profile)?,
        effects: effects(request.name, &request.scenario)?,
    };
    let source = rc::Rc::new(fixtureauthority::Authority::new(binding)?);
    let binding = continuation::CurrentAdmission::current(&*source)?;
    let count = request
        .profile
        .limits
        .total_fuel
        .checked_div(request.profile.limits.segment_fuel)
        .and_then(|value| value.checked_add(72))
        .ok_or("fixture segment bound")?;
    if count > 1_100_000 {
        return Err("fixture harness context bound".into());
    }
    let resolver: rc::Rc<dyn continuation::CurrentAdmission> = if request.scenario == fixture::Scenario::StaleGeneration
    {
        rc::Rc::new(super::Stale {
            source: source.clone(),
            advanced: cell::Cell::new(false),
        })
    } else {
        source.clone()
    };
    let contexts = (0..count)
        .map(|_| continuation::ResumeContext {
            current: resolver.clone(),
            logical_turn: 1,
        })
        .collect();
    Ok(Ready {
        bytes,
        input,
        source,
        binding,
        verified,
        contexts,
    })
}

fn validate_scenario(name: &str, scenario: &fixture::Scenario) -> Result<(), String> {
    if !fixture::NAMES.contains(&name) {
        return Err("unknown materialized fixture".into());
    }
    if matches!(
        scenario,
        fixture::Scenario::HostPause | fixture::Scenario::HostTrap { .. } | fixture::Scenario::HostUncertain { .. }
    ) && name != "molten-abi-hostcall"
    {
        return Err("host scenario requires declared-hostcall fixture".into());
    }
    Ok(())
}

fn resource(profile: &profile::Profile) -> Result<String, String> {
    let l = &profile.limits;
    let bounds = [
        l.allocation_bytes,
        l.ir_bytes,
        l.stack_bytes,
        u64::from(l.memory_pages),
        u64::from(l.table_elements),
        l.total_fuel,
        l.hostcall_bytes,
        l.input_bytes,
        l.output_bytes,
        u64::from(l.continuation_count),
        l.continuation_bytes,
        l.retention_turns,
    ];
    crate::codec::canonical_hash(&crate::codec::record("spacewasm-fixture-resource-v1", vec![crate::codec::sequence(
        bounds.into_iter().map(crate::codec::u64_value).collect(),
    )]))
    .map_err(|e| e.to_string())
}

fn effects(name: &str, scenario: &fixture::Scenario) -> Result<String, String> {
    let scenario = match scenario {
        fixture::Scenario::Standard => crate::codec::record("standard", Vec::new()),
        fixture::Scenario::StaleGeneration => crate::codec::record("stale-generation", Vec::new()),
        fixture::Scenario::HostPause => crate::codec::record("host-pause", Vec::new()),
        fixture::Scenario::HostTrap { reason } => crate::codec::record("host-trap", vec![crate::codec::string(reason)]),
        fixture::Scenario::HostUncertain { reason } => {
            crate::codec::record("host-uncertain", vec![crate::codec::string(reason)])
        }
        fixture::Scenario::Cancel { after_segments, reason } => crate::codec::record("cancel", vec![
            crate::codec::u64_value(*after_segments as u64),
            crate::codec::string(reason),
        ]),
    };
    crate::codec::canonical_hash(&crate::codec::record("spacewasm-fixture-effect-plan-v1", vec![
        crate::codec::string(name),
        scenario,
    ]))
    .map_err(|e| e.to_string())
}

pub(super) fn expected_output(request: &super::Request<'_>, ready: &Ready) -> Result<fixture::ExpectedOutput, String> {
    if !matches!(request.scenario, fixture::Scenario::Standard | fixture::Scenario::HostPause) {
        return Ok(fixture::ExpectedOutput::Absent);
    }
    match request.name {
        "molten-abi-roundtrip" | "molten-abi-hostcall" => Ok(fixture::ExpectedOutput::Canonical(ready.input.clone())),
        "molten-abi-invalid-output" => {
            // The same report has passed pinned external verification and exact module/input binding.
            let report: serde_json::Value =
                serde_json::from_slice(&super::read(request.sources.root, request.sources.chaos_report, 1_048_576)?)
                    .map_err(|e| e.to_string())?;
            if report.pointer("/reference/outcome").and_then(serde_json::Value::as_str) != Some("invalid-output") {
                return Err("invalid-output fixture lacks verified rejected bytes".into());
            }
            let hex = report
                .pointer("/reference/output_hex")
                .and_then(serde_json::Value::as_str)
                .ok_or("verified rejected bytes missing")?;
            Ok(fixture::ExpectedOutput::Rejected(rejected_bytes(hex)?))
        }
        _ => Ok(fixture::ExpectedOutput::Absent),
    }
}

fn rejected_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() > 131072 || !hex.len().is_multiple_of(2) {
        return Err("invalid bounded rejected output hex".into());
    }
    let bytes = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16).ok_or("invalid rejected output hex")?;
            let low = (pair[1] as char).to_digit(16).ok_or("invalid rejected output hex")?;
            Ok((high * 16 + low) as u8)
        })
        .collect::<Result<Vec<_>, String>>()?;
    if crate::codec::parse_canonical_bytes(&bytes).is_ok() {
        return Err("rejected output is canonical".into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::super::super::fixture;
    use super::super::super::profile;

    #[test]
    fn replay_resource_ignores_slicing_but_not_total_budget() {
        let left = profile::Profile::default();
        let mut right = left.clone();
        right.limits.segment_fuel = 1;
        assert_eq!(super::resource(&left).unwrap(), super::resource(&right).unwrap());
        right.limits.total_fuel += 1;
        assert_ne!(super::resource(&left).unwrap(), super::resource(&right).unwrap());
    }

    #[test]
    fn cancellation_and_uncertainty_reasons_bind_effect_plan() {
        let left = fixture::Scenario::Cancel {
            after_segments: 1,
            reason: "stop".into(),
        };
        let right = fixture::Scenario::Cancel {
            after_segments: 2,
            reason: "stop".into(),
        };
        assert_ne!(
            super::effects("molten-abi-out-of-fuel", &left).unwrap(),
            super::effects("molten-abi-out-of-fuel", &right).unwrap()
        );
        let left = fixture::Scenario::HostUncertain {
            reason: "committed".into(),
        };
        let right = fixture::Scenario::HostUncertain { reason: "other".into() };
        assert_ne!(
            super::effects("molten-abi-hostcall", &left).unwrap(),
            super::effects("molten-abi-hostcall", &right).unwrap()
        );
    }

    #[test]
    fn rejected_bytes_deny_malformed_hex_and_canonical_values() {
        assert!(super::rejected_bytes("0").is_err());
        assert!(super::rejected_bytes("zz").is_err());
        let canonical = crate::codec::canonical_bytes(&crate::codec::string("accepted")).unwrap();
        let hex: String = canonical.iter().map(|byte| format!("{byte:02x}")).collect();
        assert!(super::rejected_bytes(&hex).is_err());
    }
}
