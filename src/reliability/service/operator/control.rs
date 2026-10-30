const ACTIVE: &str = "coordination-active.json";
const STOP: &str = "coordination-stop.json";
const BOOTSTRAP: &str = "coordination-bootstrap.preserves";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Active {
    schema: String,
    instance_ref: String,
    grant_ref: String,
    node_id: String,
    endpoint: String,
    state: String,
}

pub(super) fn instance_ref(grant: &super::super::authority::VerifiedGrant) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("coordination-process-instance-v1", vec![
        crate::preserves_rail::string(grant.grant_ref()),
        preserves::IOValue::bytes(iroh::SecretKey::generate().to_bytes()),
    ]))
}

pub(super) fn initialize(
    authority: &super::access::Authorization,
    source: &crate::node_runtime::ValidatedNodeSourceGate,
    group: &crate::fabric_consistency::Group,
) -> crate::error::Result<serde_json::Value> {
    authority.require("init")?;
    let grant = authority.grant.grant();
    if source.binding_ref() != grant.source_binding_ref
        || group.binding_ref != grant.group_ref
        || group.membership_ref != grant.membership_ref
        || group.service_generation != grant.generation
    {
        return Err(crate::error::Failure::invalid_harness("bootstrap differs from independently signed domain grant"));
    }
    let value = bootstrap_value(authority.grant.grant_ref(), grant);
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    let path = path(BOOTSTRAP)?;
    if authority.root.try_exists(&path)? {
        if authority.root.read(&path, 65_536)? != bytes {
            return Err(crate::error::Failure::invalid_harness(
                "existing coordination bootstrap differs from current authority",
            ));
        }
    } else {
        authority.root.write_new(&path, &bytes)?;
    }
    Ok(serde_json::json!({"schema":"molten.coordination-initialized.v1",
        "bootstrap_ref":crate::preserves_rail::canonical_hash(&value)?, "production_admitted":false}))
}

pub(super) fn bootstrap_ref(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<String> {
    let value = crate::preserves_rail::parse_canonical_bytes(&root.read(&path(BOOTSTRAP)?, 65_536)?)?;
    crate::preserves_rail::canonical_hash(&value)
}

pub(super) fn check_bootstrap(authority: &super::access::Authorization) -> crate::error::Result<()> {
    authority.grant.verify_current(&authority.root)?;
    let actual = bootstrap_ref(&authority.root)?;
    let expected = bootstrap_reference(authority.grant.grant_ref(), authority.grant.grant())?;
    if actual != expected {
        return Err(crate::error::Failure::invalid_harness("rooted bootstrap differs from signed current grant"));
    }
    Ok(())
}

pub fn bootstrap_value(grant_ref: &str, grant: &super::super::authority::Grant) -> preserves::IOValue {
    crate::preserves_rail::record("live-coordination-bootstrap-v1", vec![
        crate::preserves_rail::string(grant_ref),
        crate::preserves_rail::string(&grant.source_binding_ref),
        crate::preserves_rail::string(&grant.group_ref),
        crate::preserves_rail::string(&grant.membership_ref),
        crate::preserves_rail::u64_value(grant.generation),
        crate::preserves_rail::string(&grant.endpoint),
    ])
}

pub fn bootstrap_reference(grant_ref: &str, grant: &super::super::authority::Grant) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&bootstrap_value(grant_ref, grant))
}

pub(super) fn started(
    context: &super::session::Context,
    admission: &super::runtime::HostAdmission,
) -> crate::error::Result<()> {
    context.authorization.require("serve")?;
    publish(context, "running")?;
    let descriptor = serde_json::json!({"schema":"molten.coordination-endpoint.v1",
        "node_id":context.authorization.grant.node_id(), "endpoint":admission.endpoint.to_string(),
        "run_id":admission.run_id, "profile":admission.profile});
    context.authorization.root.write(
        &path("coordination-endpoint.json")?,
        &serde_json::to_vec(&descriptor).map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))?,
    )
}

pub(super) fn publish(context: &super::session::Context, state: &str) -> crate::error::Result<()> {
    let active = Active {
        schema: "molten.coordination-active.v1".into(),
        instance_ref: context.instance_ref.clone(),
        grant_ref: context.authorization.grant.grant_ref().into(),
        node_id: context.authorization.grant.node_id().into(),
        endpoint: context.authorization.grant.endpoint().into(),
        state: state.into(),
    };
    context.authorization.root.write(
        &path(ACTIVE)?,
        &serde_json::to_vec(&active).map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))?,
    )
}

pub(super) fn should_stop(context: &super::session::Context) -> crate::error::Result<bool> {
    let path = path(STOP)?;
    if !context.authorization.root.try_exists(&path)? {
        return Ok(false);
    }
    let request: Active = serde_json::from_slice(&context.authorization.root.read(&path, 65_536)?)
        .map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))?;
    if request.instance_ref != context.instance_ref {
        return Ok(false);
    }
    context.authorization.require("stop")?;
    validate(&context.authorization, &request)?;
    Ok(request.state == "stop-requested")
}

pub(super) fn stop(authority: &super::access::Authorization) -> crate::error::Result<serde_json::Value> {
    authority.require("stop")?;
    let mut active = active(authority)?;
    if active.state == "stopped" {
        return Ok(serde_json::json!({"stopped":true,"instance_ref":active.instance_ref}));
    }
    if active.state != "running" {
        return Err(crate::error::Failure::invalid_harness("coordination host not in running state"));
    }
    active.state = "stop-requested".into();
    authority.root.write(
        &path(STOP)?,
        &serde_json::to_vec(&active).map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))?,
    )?;
    let mut deadline = crate::fabric_time::SupervisionDeadline::after(std::time::Duration::from_secs(10))?;
    while !deadline.is_expired()? {
        let observed = self::active(authority)?;
        if observed.instance_ref != active.instance_ref {
            return Err(crate::error::Failure::invalid_harness("coordination process changed while stopping"));
        }
        if observed.state == "stopped" {
            return Ok(serde_json::json!({"stopped":true,"instance_ref":active.instance_ref}));
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Err(crate::error::Failure::invalid_harness("coordination host stop deadline exceeded"))
}

fn active(authority: &super::access::Authorization) -> crate::error::Result<Active> {
    let value: Active = serde_json::from_slice(&authority.root.read(&path(ACTIVE)?, 65_536)?)
        .map_err(|error| crate::error::Failure::invalid_harness(error.to_string()))?;
    validate(authority, &value)?;
    Ok(value)
}

fn validate(authority: &super::access::Authorization, value: &Active) -> crate::error::Result<()> {
    authority.grant.verify_current(&authority.root)?;
    if value.schema != "molten.coordination-active.v1"
        || value.grant_ref != authority.grant.grant_ref()
        || value.node_id != authority.grant.node_id()
        || value.endpoint != authority.grant.endpoint()
    {
        return Err(crate::error::Failure::invalid_harness(
            "coordination control state differs from current signed grant",
        ));
    }
    crate::preserves_rail::validate_content_ref(&value.instance_ref)
}

fn path(name: &str) -> crate::error::Result<crate::node_state::RelativePath> {
    crate::node_state::RelativePath::parse(name)
}
