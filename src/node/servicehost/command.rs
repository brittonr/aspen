//! Explicit public CLI boundary for admitted durable native service hosting.

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    Install(Arguments),
    Status(Arguments),
    Supervise(Arguments),
    Submit(Arguments),
    Readmit(Arguments),
    Deliver(Arguments),
    Upgrade(Arguments),
    Reconcile(Arguments),
}

#[derive(Debug, clap::Args)]
pub struct Arguments {
    #[arg(long)]
    pub state_root: std::path::PathBuf,
    /// Canonical packed bundle, relative to the selected owned state root.
    #[arg(long)]
    pub admission: String,
    #[arg(long)]
    pub logical_tick: u64,
    /// Canonical packed bounded operation, relative to the selected owned state root.
    #[arg(long)]
    pub input: String,
    /// Emit an exact operation signing request without performing the operation.
    #[arg(long)]
    pub preview: bool,
    /// Emit a source-validated load signing request before any hosting effects.
    #[arg(long, conflicts_with = "preview")]
    pub preview_load: bool,
}

pub fn run(command: Subcommand) -> crate::error::Result<()> {
    let (name, args) = match command {
        Subcommand::Install(args) => ("install", args),
        Subcommand::Status(args) => ("status", args),
        Subcommand::Supervise(args) => ("supervise", args),
        Subcommand::Submit(args) => ("submit", args),
        Subcommand::Readmit(args) => ("readmit", args),
        Subcommand::Deliver(args) => ("deliver", args),
        Subcommand::Upgrade(args) => ("upgrade", args),
        Subcommand::Reconcile(args) => ("reconcile", args),
    };
    let receipt = invoke(name, args)?;
    println!("{}", crate::preserves_rail::to_text(&receipt)?);
    Ok(())
}

pub fn invoke(name: &str, args: Arguments) -> crate::error::Result<preserves::IOValue> {
    let root = crate::node_state::DirectoryRoot::open_existing(&args.state_root)?;
    let gate = crate::node_daemon::admit_startup_source_gate_with_root(&root)?;
    let admission = root.read(&crate::node_state::RelativePath::parse(&args.admission)?, 16_777_216)?;
    let bundle_ref = crate::preserves_rail::content_ref_from_bytes(&admission);
    let bundle = super::codec::decode(&admission, &bundle_ref)?;
    let load = load_request(&bundle_ref, gate.validation_ref(), args.logical_tick)?;
    if args.preview_load {
        return Ok(request_value(&load));
    }
    let bytes =
        root.read(&crate::node_state::RelativePath::parse(&args.input)?, super::operation::MAX_OPERATION_BYTES as u64)?;
    let operation = super::operation::decode(&bytes, name, args.logical_tick)?;
    let clock = crate::world_witness::live_clock()?;
    let mut authority = super::authority::Current::open(&root, &bundle_ref, clock)?;
    crate::node_service_hosting::AuthorityPort::admit(&mut authority, &load)?;
    let mut materialized = super::runtime::materialize(&root, bundle, &gate)?;
    if name == "deliver" && !args.preview {
        super::effects::install(super::effects::InstallInput {
            root: &root,
            bundle_ref: &bundle_ref,
            gate: &gate,
            logical_tick: args.logical_tick,
            runtime: &mut materialized.runtime,
            observers: None,
        })?;
    }
    let mut host = crate::node_service_hosting::Host::open(&root, authority, materialized.runtime)?;
    let preview = host.preview(hosting_input(&operation, &materialized.groups)?, args.logical_tick)?;
    if args.preview {
        return Ok(request_value(&preview));
    }
    let outcome = execute(&mut host, operation, &materialized.groups, args.logical_tick)?;
    // Keep measured executable and workspace handles alive until after the last callback.
    drop(host);
    drop(materialized.files);
    drop(materialized.directories);
    Ok(crate::preserves_rail::record("node-service-host-receipt-v1", vec![
        crate::preserves_rail::string(name),
        crate::preserves_rail::string(&bundle_ref),
        crate::preserves_rail::u64_value(args.logical_tick),
        crate::preserves_rail::string(super::authority::request_ref(&bundle_ref, &preview)?),
        outcome,
    ]))
}

type Hosting = crate::node_service_hosting::Host<
    super::authority::Current<crate::fabric_time::LiveClockAdapter>,
    super::runtime::Native,
>;
type Groups = std::collections::BTreeMap<String, crate::system_extension::RecoveryGroupDeclaration>;

pub(super) fn hosting_input<'a>(
    operation: &'a super::operation::Invocation,
    groups: &'a Groups,
) -> crate::error::Result<crate::node_service_hosting::preview::Input<'a>> {
    Ok(match operation {
        super::operation::Invocation::Install(group) => crate::node_service_hosting::preview::Input::Install(
            groups
                .get(group)
                .ok_or_else(|| super::invalid("install selects no admitted recovery declaration"))?,
        ),
        super::operation::Invocation::Status => crate::node_service_hosting::preview::Input::Status,
        super::operation::Invocation::Supervise { group, exit } => {
            crate::node_service_hosting::preview::Input::Supervise { group_id: group, exit }
        }
        super::operation::Invocation::Submit { member, work, payload } => {
            crate::node_service_hosting::preview::Input::Submit {
                member,
                presentation: work,
                payload,
            }
        }
        super::operation::Invocation::Readmit { member, work_ref } => {
            crate::node_service_hosting::preview::Input::Readmit { member, work_ref }
        }
        super::operation::Invocation::Migrate { member, input } => {
            crate::node_service_hosting::preview::Input::ReadmitMigrated { member, input }
        }
        super::operation::Invocation::Deliver(input) => {
            crate::node_service_hosting::preview::Input::Deliver(input.as_ref())
        }
        super::operation::Invocation::Upgrade {
            member,
            replacement_ref,
        } => crate::node_service_hosting::preview::Input::Upgrade {
            member,
            admission_ref: replacement_ref,
        },
        super::operation::Invocation::Reconcile(group_id) => {
            crate::node_service_hosting::preview::Input::Reconcile { group_id }
        }
    })
}

pub(super) fn execute(
    host: &mut Hosting,
    operation: super::operation::Invocation,
    groups: &Groups,
    tick: u64,
) -> crate::error::Result<preserves::IOValue> {
    match operation {
        super::operation::Invocation::Status => return host.status(tick),
        super::operation::Invocation::Install(group) => host.install(
            groups.get(&group).ok_or_else(|| super::invalid("unknown admitted recovery group"))?.clone(),
            tick,
        )?,
        super::operation::Invocation::Supervise { group, exit } => {
            host.supervise(&group, &exit, tick)?;
        }
        super::operation::Invocation::Submit { member, work, payload } => host.submit(&member, work, payload, tick)?,
        super::operation::Invocation::Readmit { member, work_ref } => {
            return host.readmit(&member, &work_ref, tick).map(claim_value);
        }
        super::operation::Invocation::Migrate { member, input } => {
            return host.readmit_migrated(&member, &input, tick).map(claim_value);
        }
        super::operation::Invocation::Deliver(input) => {
            let result = host.deliver(&input)?;
            return Ok(crate::preserves_rail::record("node-service-host-delivered-v1", vec![
                crate::preserves_rail::string(result.admission.acknowledgement_ref),
                effect_outcome(&result.effects),
            ]));
        }
        super::operation::Invocation::Upgrade {
            member,
            replacement_ref,
        } => {
            host.upgrade(&member, &replacement_ref, tick)?;
        }
        super::operation::Invocation::Reconcile(group) => {
            host.reconcile(&group, tick)?;
        }
    }
    Ok(crate::preserves_rail::record("node-service-host-committed-v1", Vec::new()))
}

/// The effect disposition an operator needs to tell an unrouted effect from a completed one.
///
/// The delivery acknowledgement alone cannot distinguish "the callback ran and its effect
/// completed" from "the callback ran and the host had no router", and durable work completion
/// depends on exactly that distinction.
fn effect_outcome(disposition: &crate::node_service_hosting::NativeEffectDisposition) -> preserves::IOValue {
    match disposition {
        crate::node_service_hosting::NativeEffectDisposition::NotRequested => crate::preserves_rail::record(
            "node-service-host-effect-outcome-v1",
            vec![crate::preserves_rail::string("not-requested")],
        ),
        crate::node_service_hosting::NativeEffectDisposition::Unrouted => crate::preserves_rail::record(
            "node-service-host-effect-outcome-v1",
            vec![crate::preserves_rail::string("unrouted")],
        ),
        crate::node_service_hosting::NativeEffectDisposition::Completed(completions) => {
            crate::preserves_rail::record("node-service-host-effect-outcome-v1", vec![
                crate::preserves_rail::string("completed"),
                crate::preserves_rail::sequence(completions.iter().map(crate::preserves_rail::string).collect()),
            ])
        }
    }
}

fn claim_value(claim: crate::system_extension::DeliveryClaim) -> preserves::IOValue {
    crate::preserves_rail::record("node-service-host-claim-v1", vec![
        crate::preserves_rail::string(claim.claim_ref),
        crate::preserves_rail::string(claim.class.as_str()),
        crate::preserves_rail::u64_value(claim.generation),
        crate::preserves_rail::u64_value(claim.incarnation),
    ])
}

pub fn request_value(request: &crate::node_service_hosting::OperationRequest) -> preserves::IOValue {
    crate::preserves_rail::record("node-service-host-operation-request-v1", vec![
        crate::preserves_rail::string(&request.operation),
        crate::preserves_rail::string(&request.subject),
        crate::preserves_rail::u64_value(request.logical_tick),
        crate::preserves_rail::string(&request.state_ref),
        crate::preserves_rail::string(&request.input_ref),
    ])
}

/// Pure request projection for an independent load-credential producer.
pub fn load_request(
    bundle_ref: &str,
    source_validation_ref: &str,
    logical_tick: u64,
) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
    crate::preserves_rail::ContentRef::parse(bundle_ref)?;
    crate::preserves_rail::ContentRef::parse(source_validation_ref)?;
    Ok(crate::node_service_hosting::OperationRequest {
        operation: "load".to_string(),
        subject: bundle_ref.to_string(),
        logical_tick,
        state_ref: source_validation_ref.to_string(),
        input_ref: bundle_ref.to_string(),
    })
}
