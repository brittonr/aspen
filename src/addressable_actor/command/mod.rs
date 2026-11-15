//! Aspen's explicit public-node counter consumer. Every protected operation uses independent
//! signing.
mod actions;
pub mod configuration;
pub mod placement;
mod signing;
#[cfg(test)]
mod tests;

use crate::addressable_actor::native::SigningPort;

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    Initialize(Arguments),
    Fence(Arguments),
    Apply(Arguments),
    PlacementInit(Arguments),
    PlacementStep(Arguments),
    Delivery(Arguments),
    Native(Arguments),
    Result(Arguments),
}

#[derive(Debug, clap::Args)]
pub struct Arguments {
    #[arg(long)]
    pub state_root: std::path::PathBuf,
    #[arg(long, default_value = "actor-configuration.json")]
    pub configuration: String,
    #[arg(long, default_value = "delivery-policy.json")]
    pub delivery_policy: String,
    #[arg(long, default_value = "delivery-manifest.json")]
    pub delivery_manifest: String,
    #[arg(long, default_value = "time-profile.json")]
    pub time_profile: String,
    #[arg(long, default_value = "membership-profile.json")]
    pub membership_profile: String,
    #[arg(long, default_value = "assignment-proposal.json")]
    pub counter_placement: String,
    #[arg(long)]
    pub input: Option<String>,
    #[arg(long)]
    pub logical_tick: u64,
    #[arg(long, default_value_t = 1)]
    pub engine_epoch: u64,
    /// Emit each exact current signing request and wait for its independently published credential.
    #[arg(long)]
    pub interactive_signing: bool,
    /// Describe only the actual source-validated load request; perform no hosting operation.
    #[arg(long, conflicts_with = "interactive_signing")]
    pub preview: bool,
}

impl Subcommand {
    fn selection(self) -> (&'static str, Arguments) {
        match self {
            Self::Initialize(args) => ("initialize", args),
            Self::Fence(args) => ("fence", args),
            Self::Apply(args) => ("apply", args),
            Self::PlacementInit(args) => ("placement-init", args),
            Self::PlacementStep(args) => ("placement-step", args),
            Self::Delivery(args) => ("delivery", args),
            Self::Native(args) => ("native", args),
            Self::Result(args) => ("result", args),
        }
    }
}

pub fn run(command: Subcommand) -> crate::error::Result<()> {
    let (action, args) = command.selection();
    let root = crate::node_state::DirectoryRoot::open_existing(&args.state_root)?;
    let gate = crate::node_daemon::admit_startup_source_gate_with_root(&root)?;
    let bytes = root.read(&crate::node_state::RelativePath::parse("admission.preserves")?, 16_777_216)?;
    let bundle_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
    if args.preview {
        let request =
            crate::node_service_host::command::load_request(&bundle_ref, gate.validation_ref(), args.logical_tick)?;
        println!(
            "SIGN {}",
            crate::preserves_rail::to_text(&crate::node_service_host::command::request_value(&request))?
        );
        return Ok(());
    }
    if !args.interactive_signing {
        return Err(invalid("public actor operations require --interactive-signing"));
    }
    let bundle = crate::node_service_host::codec::decode(&bytes, &bundle_ref)?;
    let configuration: super::native::Configuration = read_json(&root, &args.configuration)?;
    let policy: molten_core::coordination_delivery::DeliveryPolicy = read_json(&root, &args.delivery_policy)?;
    let manifest: molten_core::coordination_delivery::DeliveryManifest = read_json(&root, &args.delivery_manifest)?;
    let time: configuration::TimeSelection = read_json(&root, &args.time_profile)?;
    let time = time.admit()?;
    if time.profile.kind != crate::fabric_time::TimeProfileKind::Live
        || configuration.profile.time_profile_ref != time.profile.profile_ref
        || configuration.profile.delivery_profile_ref
            != molten_core::coordination_delivery::identify_delivery_manifest(&manifest)
        || configuration.delivery_member != manifest.service_id
    {
        return Err(invalid("public actor profile selections do not match exact live delivery/time configuration"));
    }
    let membership: configuration::MembershipSelection = read_json(&root, &args.membership_profile)?;
    let membership = membership.admit()?;
    let storage = root.namespace(crate::node_state::NamespaceKind::Storage)?;
    let assignment_root = crate::local_store::DurableStoreRoot::from_ledger(
        &root.ledger_store()?,
        &crate::local_store::Locator::parse("addressable-actor-assignments")?,
    )?;
    let placement = crate::fabric_membership::persistence::Redb::open(&assignment_root).map_err(problem)?;
    let local = crate::coordination_delivery::Local::open(&storage, args.engine_epoch).map_err(problem)?;
    let clock = crate::fabric_time::LiveClockAdapter::new(&time.profile, 1)?;
    let bridge = super::delivery::Bridge::new(local, manifest.clone(), policy, time.profile, clock)?;
    let mut signing = signing::Dialogue::new(bundle_ref.clone());
    let owner = crate::node_service_host::installed::Owner::open_interactive(
        &root,
        &gate,
        "admission.preserves",
        args.logical_tick,
        &mut signing,
    )?;
    let session = Session {
        root,
        storage,
        configuration,
        manifest,
        membership,
        bundle,
        bundle_ref,
        owner,
        placement,
        bridge,
        signing,
        engine_epoch: args.engine_epoch,
        tick: args.logical_tick,
    };
    execute_session(session, &args, action)
}

fn execute_session(mut session: Session, args: &Arguments, action: &str) -> crate::error::Result<()> {
    let input = args.input.as_ref().map(|path| read_json::<serde_json::Value>(&session.root, path)).transpose()?;
    let output = match action {
        "placement-init" => placement::initialize(&mut session, &args.counter_placement)?,
        "placement-step" => {
            placement::step(&mut session, input.as_ref().ok_or_else(|| invalid("placement-step requires --input"))?)?
        }
        _ => actions::execute(&mut session, action, input.as_ref())?,
    };
    println!("RESULT {}", serde_json::to_string(&output).map_err(problem)?);
    session.owner.close();
    Ok(())
}

pub(super) struct Session {
    root: crate::node_state::DirectoryRoot,
    storage: crate::node_state::Scope,
    configuration: super::native::Configuration,
    manifest: molten_core::coordination_delivery::DeliveryManifest,
    membership: crate::fabric_membership::Profile,
    bundle: crate::node_service_host::codec::Bundle,
    bundle_ref: String,
    owner: crate::node_service_host::installed::Owner,
    placement: crate::fabric_membership::persistence::Redb,
    bridge: super::delivery::Bridge<crate::fabric_time::LiveClockAdapter>,
    signing: signing::Dialogue,
    engine_epoch: u64,
    tick: u64,
}

impl Session {
    fn member(&self) -> String {
        molten_core::addressable_actor::identify_actor_key(&self.configuration.actor_key)
    }
    fn observe(&mut self) -> crate::error::Result<crate::system_extension::NativeInstanceRecord> {
        let member = self.member();
        self.signing.prepare(
            &self
                .owner
                .actor_preview(crate::node_service_hosting::native::actor::Input::Observe(&member), self.tick)?,
        )?;
        self.owner.actor_instance(&member, self.tick)
    }
    fn consumer(&mut self) -> crate::error::Result<super::native::Consumer<'_, crate::fabric_time::LiveClockAdapter>> {
        super::native::Consumer::open(super::native::Open {
            storage: &self.storage,
            engine_epoch: self.engine_epoch,
            configuration: &self.configuration,
            owner: &mut self.owner,
            placement: &mut self.placement,
            delivery: &mut self.bridge,
            signing: &mut self.signing,
        })
    }
    fn retain(&self, value: &preserves::IOValue) -> crate::error::Result<String> {
        let bytes = crate::preserves_rail::canonical_bytes(value)?;
        let reference = crate::preserves_rail::content_ref_from_bytes(&bytes);
        let path = crate::node_state::RelativePath::parse(&format!("evidence/{}.preserves", &reference[7..]))?;
        match self.root.entry_kind(&path)? {
            Some(_) if self.root.read(&path, 4_194_304)? != bytes => {
                return Err(invalid("actor evidence identity collision"));
            }
            Some(_) => {}
            None => self.root.write_new(&path, &bytes)?,
        }
        Ok(reference)
    }
}

pub(super) fn read_json<T: serde::de::DeserializeOwned>(
    root: &crate::node_state::DirectoryRoot,
    path: &str,
) -> crate::error::Result<T> {
    let bytes = root.read(&crate::node_state::RelativePath::parse(path)?, 1_048_576)?;
    serde_json::from_slice(&bytes).map_err(problem)
}
pub(super) fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
pub(super) fn problem(error: impl std::fmt::Debug) -> crate::error::Failure {
    invalid(format!("public actor command: {error:?}"))
}
