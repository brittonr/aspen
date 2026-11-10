//! Per-node operator authority, startup selection and exact host credentials.

pub const STARTUP: &str = "service-host-startup";

/// The admitted scope policy the operator authority publishes.
pub const SCOPE_POLICY: &str = r#"{"backends":{},"contracts":{"collaboration-scope":{"abilities":["resource/read","resource/use","resource/share","resource/bind","resource/move","scope/create","scope/membership"],"description":"Native collaboration operator","id":"collaboration-scope","resource_prefixes":["molten://collaboration/"]}},"schema_version":"ucan-nickel-contracts.policy.v1"}"#;

/// Prepare one node root for `node serve`: operator trusts, startup selection and the exact
/// `load`/`install-effects` credentials the serving process will demand.
pub fn prepare(
    signed_main: &std::path::Path,
    node: &crate::producer::Node,
    issuer: &crate::authority::Issuer,
    index: u64,
    tick: u64,
) -> crate::producer::Result<()> {
    let window = crate::authority::Window::around_now()?;
    issuer.provision_host(&node.root, window)?;
    issuer.provision_scope(&node.root, &crate::authority::scope::TrustInput {
        policies: &[SCOPE_POLICY],
        revoked_tokens: &[],
        logical_position: 0,
        unix_seconds: now()?,
    })?;
    write_startup(node, tick)?;
    let activation_host = crate::driver::process::Host {
        signed_main,
        state_root: &node.path,
        issuer,
        index,
        control: None,
    };
    let operation =
        super::operations::wire::operation("install", vec![molten::preserves_rail::string(crate::producer::GROUP_ID)]);
    super::flow::authorize(&activation_host, node, &operation, 0)?;
    let (success, output) = super::process::invoke(&activation_host, "install", 0, None)?;
    if !success {
        return Err(format!("pre-serve group activation denied: {output}").into());
    }
    // The current-process validation reference is a fact of the executing binary, so the exact
    // load request is read from a real preview rather than reconstructed from authoring evidence.
    let load = preview_load(signed_main, node, tick)?;
    let validation_ref = load.state_ref.clone();
    sign_host(node, issuer, &load, window, nonce(index, 0)?)?;
    let install =
        crate::authority::host::install_effects_request(&node.bundle_ref, &node.effects, &validation_ref, tick)?;
    sign_host(node, issuer, &install, window, nonce(index, 1)?)?;
    // `serve` admits its first live status operation at the declared startup tick, so the operator
    // must authorize that exact request before the process starts.
    let status = preserves::IOValue::record(preserves::IOValue::symbol("node-service-host-status-v1"), Vec::new());
    let host = crate::driver::process::Host {
        signed_main,
        state_root: &node.path,
        issuer,
        index,
        control: None,
    };
    crate::driver::flow::authorize(&host, node, &status, tick)?;
    // Selection is published only after explicit group activation. Startup must reject a selected
    // runtime whose member is not Running; it must not silently defer required admission.
    node.root.write_new(
        &molten::node_state::RelativePath::parse(crate::producer::EFFECTS)?,
        &molten::node_service_host::effects::config::encode(&node.effects)?,
    )?;
    Ok(())
}

/// Authorize a new serving process without changing the existing runtime or operator trusts.
pub(super) fn authorize_replacement(
    signed_main: &std::path::Path,
    node: &crate::producer::Node,
    issuer: &crate::authority::Issuer,
    index: u64,
    tick: u64,
) -> crate::producer::Result<molten::node_service_hosting::OperationRequest> {
    let startup = molten::preserves_rail::record("node-service-host-startup-v1", vec![
        molten::preserves_rail::string(crate::producer::ADMISSION),
        molten::preserves_rail::u64_value(tick),
    ]);
    node.root.write(
        &molten::node_state::RelativePath::parse(STARTUP)?,
        &molten::preserves_rail::canonical_bytes(&startup)?,
    )?;
    let host = super::process::Host {
        signed_main,
        state_root: &node.path,
        issuer,
        index,
        control: None,
    };
    let load = preview_load(signed_main, node, tick)?;
    let install =
        crate::authority::host::install_effects_request(&node.bundle_ref, &node.effects, &load.state_ref, tick)?;
    super::process::sign(&host, node, &install, tick, 3)?;
    let status = super::operations::wire::operation("status", Vec::new());
    super::flow::authorize(&host, node, &status, tick)?;
    Ok(install)
}

fn preview_load(
    signed_main: &std::path::Path,
    node: &crate::producer::Node,
    tick: u64,
) -> crate::producer::Result<molten::node_service_hosting::OperationRequest> {
    let operation = preserves::IOValue::record(preserves::IOValue::symbol("node-service-host-status-v1"), Vec::new());
    node.root.write(
        &molten::node_state::RelativePath::parse("operation.preserves")?,
        &molten::preserves_rail::canonical_bytes(&operation)?,
    )?;
    let output = std::process::Command::new(signed_main)
        .args(["node", "service-host", "status", "--state-root"])
        .arg(&node.path)
        .args(["--admission", crate::producer::ADMISSION, "--logical-tick"])
        .arg(tick.to_string())
        .args(["--input", "operation.preserves", "--preview-load"])
        .output()?;
    let text = format!("{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        return Err(format!("load preview denied: {text}").into());
    }
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with("<node-service-host-operation-request-v1"))
        .ok_or("missing load preview request")?;
    let parsed = molten::preserves_rail::parse_text(line)?;
    let fields = molten::preserves_rail::simple_record_fields(&parsed, "node-service-host-operation-request-v1", 5)?;
    Ok(molten::node_service_hosting::OperationRequest {
        operation: molten::preserves_rail::required_string_field(&fields[0], "load operation")?,
        subject: molten::preserves_rail::required_string_field(&fields[1], "load subject")?,
        logical_tick: fields[2].as_u64().ok_or("load tick")??,
        state_ref: molten::preserves_rail::required_string_field(&fields[3], "load validation")?,
        input_ref: molten::preserves_rail::required_string_field(&fields[4], "load input")?,
    })
}

fn write_startup(node: &crate::producer::Node, tick: u64) -> crate::producer::Result<()> {
    let value = molten::preserves_rail::record("node-service-host-startup-v1", vec![
        molten::preserves_rail::string(crate::producer::ADMISSION),
        molten::preserves_rail::u64_value(tick),
    ]);
    node.root.write_new(
        &molten::node_state::RelativePath::parse(STARTUP)?,
        &molten::preserves_rail::canonical_bytes(&value)?,
    )?;
    Ok(())
}

fn sign_host(
    node: &crate::producer::Node,
    issuer: &crate::authority::Issuer,
    request: &molten::node_service_hosting::OperationRequest,
    window: crate::authority::Window,
    nonce: [u8; 32],
) -> crate::producer::Result<()> {
    let credential = issuer.host_credential(&node.bundle_ref, request, window, nonce)?;
    issuer.write_host_credential(&node.root, &node.bundle_ref, request, &credential)?;
    Ok(())
}

pub fn now() -> crate::producer::Result<u64> {
    // The operator interval is read through the admitted wall clock, not an ambient one.
    molten::world_witness::WallClock::unix_seconds(&mut molten::world_witness::live_clock()?)
        .map_err(|error| format!("{error:?}").into())
}

/// A distinct holder nonce per node and role; the operator root never reuses one.
pub fn nonce(index: u64, role: u8) -> crate::producer::Result<[u8; 32]> {
    let mut nonce = [0u8; 32];
    nonce[0] = role;
    nonce[1] = u8::try_from(index).map_err(|_| "cohort position exceeds one byte")?;
    nonce[31] = 0x7f;
    Ok(nonce)
}
