// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::BTreeMap;

use molten_orchestrator_codec::*;
#[test]
fn projection_matches_an_independently_written_preserves_value() {
    let input = BTreeMap::from([("name", "example")]);
    let encoded = encode("fixture.v1", &input).unwrap();
    let expected = preserves::read_iovalue_text(
        "<molten.orchestration.document.v1 \"fixture.v1\" <o.map [<o.field \"name\" <o.string \"example\">>]>>",
        false,
    )
    .unwrap();
    assert_eq!(encoded.canonical, preserves::write_iovalue_packed(&expected, false).unwrap());
    assert_eq!(encoded.id, digest(&encoded.canonical));
}
#[test]
fn schema_domain_separation_changes_identity() {
    assert_ne!(identity("one.v1", &123_u64).unwrap(), identity("two.v1", &123_u64).unwrap());
}
#[test]
fn roundtrip_rejects_modified_canonical_and_projection() {
    let input = BTreeMap::from([("name".to_string(), "x".to_string())]);
    let encoded = encode("test.v1", &input).unwrap();
    let out: BTreeMap<String, String> = decode("test.v1", &encoded.canonical, &encoded.projection).unwrap();
    assert_eq!(out, input);
    let mut corrupt = encoded.canonical.clone();
    corrupt.push(0);
    assert!(decode::<BTreeMap<String, String>>("test.v1", &corrupt, &encoded.projection).is_err());
    assert!(
        decode::<BTreeMap<String, String>>("test.v1", &encoded.canonical, b"{\"name\":\"x\",\"name\":\"x\"}").is_err()
    );
}
#[test]
fn wire_rejects_wrong_schema_trailing_data_and_duplicate_keys() {
    let bytes = frame("request.v1", &42_u64).unwrap();
    assert_eq!(unframe::<u64>("request.v1", &bytes).unwrap(), 42);
    assert!(unframe::<u64>("other.v1", &bytes).is_err());
    let mut extra = bytes.clone();
    extra.push(b' ');
    assert!(unframe::<u64>("request.v1", &extra).is_err());
    let text = String::from_utf8(bytes).unwrap().replace("\"body\":42", "\"body\":42,\"body\":42");
    assert!(unframe::<u64>("request.v1", text.as_bytes()).is_err());
}
#[test]
fn floats_and_excessive_nesting_are_not_admitted() {
    assert!(encode("float.v1", &1.5_f64).is_err());
    let nested = format!("{}0{}", "[".repeat(200), "]".repeat(200));
    assert!(unframe::<serde_json::Value>("nested.v1", nested.as_bytes()).is_err());
}

#[test]
fn workload_pair_and_content_kinds_are_canonically_distinct() {
    use molten_batch_core::ContentKind;
    use molten_batch_core::ContentManifest;
    use molten_batch_core::Id;
    use molten_batch_core::Workload;
    let a = Id([7; 32]);
    let b = Id([8; 32]);
    let native = Workload::PureNet {
        manifest: a,
        artifact: b,
    };
    let encoded = encode("workload-test.v2", &native).unwrap();
    assert_eq!(decode::<Workload>("workload-test.v2", &encoded.canonical, &encoded.projection).unwrap(), native);
    assert_ne!(
        encoded.id,
        identity("workload-test.v2", &Workload::PureNet {
            manifest: b,
            artifact: a
        })
        .unwrap()
    );
    assert_ne!(encoded.id, identity("workload-test.v2", &Workload::Component { artifact: b }).unwrap());
    assert!(decode::<Workload>("workload-test.v1", &encoded.canonical, &encoded.projection).is_err());
    let ids: std::collections::BTreeSet<_> = [
        ContentKind::Component,
        ContentKind::Data,
        ContentKind::PureNetManifest,
        ContentKind::PureNetProgram,
    ]
    .into_iter()
    .map(|kind| {
        identity("content-manifest.v2", &ContentManifest {
            kind,
            raw_digest: a,
            length: 1,
        })
        .unwrap()
    })
    .collect();
    assert_eq!(ids.len(), 4);
}

#[test]
fn legacy_and_incomplete_task_carriers_are_rejected_without_adoption() {
    use molten_batch_core::TaskSpec;
    let artifact = "07".repeat(32);
    let base = serde_json::json!({"id":"task","workload":{"Component":{"artifact":artifact}},
        "inputs":[],"dependencies":[],"resources":{"cpu_millis":1,"memory_bytes":65536,"slots":1},
        "required_labels":{},"fuel":1,"timeout_ms":1,"max_output_bytes":1,"max_attempts":1,"retry_delay_ticks":0});
    assert!(serde_json::from_value::<TaskSpec>(base.clone()).is_ok());
    let mut legacy = base.clone();
    legacy.as_object_mut().unwrap().remove("workload");
    legacy["component"] = serde_json::json!(artifact);
    assert!(serde_json::from_value::<TaskSpec>(legacy).is_err());
    for workload in [
        serde_json::json!({"PureNet":{"artifact":artifact}}),
        serde_json::json!({"PureNet":{"manifest":artifact,"artifact":artifact,"fallback":true}}),
        serde_json::json!({"Native":{"artifact":artifact}}),
    ] {
        let mut invalid = base.clone();
        invalid["workload"] = workload;
        assert!(serde_json::from_value::<TaskSpec>(invalid).is_err());
    }
}

#[test]
fn stopped_failure_report_participates_in_terminal_identity() {
    use molten_batch_core::ExecutionOutcome;
    use molten_batch_core::ExecutionResult;
    use molten_batch_core::FailureCode;
    use molten_batch_core::Id;
    let result = ExecutionResult {
        request_ref: Id([1; 32]),
        outcome: ExecutionOutcome::Failed {
            code: FailureCode::ReductionLimit,
        },
        stopped: true,
        report: Some(Id([2; 32])),
    };
    let encoded = encode("execution-result.v2", &result).unwrap();
    assert_eq!(
        decode::<ExecutionResult>("execution-result.v2", &encoded.canonical, &encoded.projection).unwrap(),
        result
    );
    assert_ne!(encoded.id, identity("execution-result.v2", &ExecutionResult { report: None, ..result }).unwrap());
    assert_ne!(
        encoded.id,
        identity("execution-result.v2", &ExecutionResult {
            report: Some(Id([3; 32])),
            ..result
        })
        .unwrap()
    );
}
