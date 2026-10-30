//! Separate optional telemetry fixture. Exporter failures never change the RPC report.
pub struct Input<'a> {
    pub package: &'a cap_std::fs::Dir,
    pub staging: &'a cap_std::fs::Dir,
    pub rpc_output: &'a cap_std::fs::Dir,
    pub sink: &'a cap_std::fs::Dir,
    pub sink_ref: &'a str,
}
fn unexported(sink: &cap_std::fs::Dir) -> super::Result<()> {
    match sink.symlink_metadata("otel-diagnostic.preserves") {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err("negative telemetry case produced an export or sink is not fresh".into()),
    }
}
fn scope(bundle: &super::Bundle, receipt: String, sink: &str) -> super::Result<super::Scope> {
    Ok(super::Scope {
        component: super::super::reference(bundle.component()),
        bundle: bundle.reference().into(),
        wit: bundle.wit(),
        receipt,
        sink: sink.into(),
        policy: super::super::authority::policy_ref(super::fixtureauth::POLICY)?,
        generation: 1,
        deadline_tick: 100,
    })
}

pub async fn run_fixture(input: Input<'_>) -> super::Result<preserves::IOValue> {
    let original = super::super::operator::show(input.rpc_output)?;
    let bundle = super::Bundle::load_package(input.package, input.staging)?;
    let bytes = super::super::package::read(input.rpc_output, "client.preserves")?;
    let value = crate::codec::parse_canonical_bytes(&bytes).map_err(super::super::failure)?;
    let transcript = super::super::transcript::Transcript::from_value(&value)?;
    let scope = scope(&bundle, transcript.reference, input.sink_ref)?;
    unexported(input.sink)?;
    if super::fixtureauth::lease(&scope, 1, false).is_ok() {
        return Err("RPC-only UCAN authorized telemetry export".into());
    }
    let mut cases = vec!["independent-telemetry-authority-deny"];
    for (mode, reason, label) in [
        (1, "OTel string field is not approved", "secret-attribute-export-deny"),
        (2, "span lifecycle/queue budget denied", "bounded-queue-export-deny"),
    ] {
        let lease = super::fixtureauth::lease(&scope, mode as u8 + 2, true)?;
        let result = super::export(super::ExportInput {
            bundle: &bundle,
            lease,
            sink: input.sink,
            sink_ref: input.sink_ref,
            mode,
            tick: 10,
            generation: 1,
        })
        .await;
        match result {
            Err(error) if error.contains(reason) => {}
            _ => return Err(format!("telemetry case did not fail at {label}")),
        }
        unexported(input.sink)?;
        cases.push(label);
    }
    let lease = super::fixtureauth::lease(&scope, 8, true)?;
    let diagnostic = super::export(super::ExportInput {
        bundle: &bundle,
        lease,
        sink: input.sink,
        sink_ref: input.sink_ref,
        mode: 0,
        tick: 10,
        generation: 1,
    })
    .await?;
    if show(input.sink)? != diagnostic || super::super::operator::show(input.rpc_output)? != original {
        return Err("telemetry readback differs or changed application acceptance".into());
    }
    let result = crate::codec::record("component-rpc-otel-fixture-diagnostic-v1", vec![
        crate::codec::string("diagnostic-only-not-application-acceptance"),
        crate::codec::string(&scope.receipt),
        crate::codec::string(super::super::hash(&diagnostic)?),
        crate::codec::record("observed-negative-cases", cases.into_iter().map(crate::codec::string).collect()),
    ]);
    super::super::operator::persist(input.sink, "otel-fixture.preserves", &result)?;
    Ok(result)
}

pub fn show(sink: &cap_std::fs::Dir) -> super::Result<preserves::IOValue> {
    let bytes = super::super::package::read(sink, "otel-diagnostic.preserves")?;
    let value = crate::codec::parse_canonical_bytes(&bytes).map_err(super::super::failure)?;
    let fields = crate::codec::simple_record_fields(&value, "component-rpc-otel-diagnostic-export-v1", 5)
        .map_err(super::super::failure)?;
    let scope_value = crate::codec::value_to_iovalue(&fields[1]);
    let scope = crate::codec::simple_record_fields(&scope_value, "component-rpc-otel-scope-v1", 12)
        .map_err(super::super::failure)?;
    let receipt = crate::codec::required_content_ref_string(&scope[5], "linked canonical receipt")
        .map_err(super::super::failure)?;
    let count = crate::codec::required_string_field(&fields[3], "span count").map_err(super::super::failure)?;
    let count: u64 = count.parse().map_err(super::super::failure)?;
    if count == 0
        || count > super::MAX_SPANS as u64
        || crate::codec::required_string_field(&fields[0], "role").map_err(super::super::failure)?
            != "diagnostic-only-not-canonical-evidence"
    {
        return Err("telemetry readback role or bound differs".into());
    }
    let spans_value = crate::codec::value_to_iovalue(&fields[4]);
    let spans = crate::codec::simple_record_fields(&spans_value, "spans", count).map_err(super::super::failure)?;
    let expected = crate::codec::record("component-rpc-otel-span-diagnostic-v1", vec![
        crate::codec::string("diagnostic-only"),
        crate::codec::string(receipt),
        crate::codec::string("component-rpc-call"),
        crate::codec::string("ended"),
        crate::codec::string("not-application-acceptance-not-canonical-evidence"),
    ]);
    if spans.fields_iter().any(|span| crate::codec::value_to_iovalue(span) != expected) {
        return Err("telemetry readback contains unapproved fields/payloads".into());
    }
    Ok(value)
}
