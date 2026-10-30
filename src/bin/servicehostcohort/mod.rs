mod credentials;
mod operations;
mod operator;
mod service;
mod source;

#[derive(clap::Parser)]
#[command(about = "Author an owned experimental native recovery cohort from an independently signed actual Nix output")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
    Create(Arguments),
    Authorize(credentials::Arguments),
}

#[derive(clap::Args)]
pub struct Arguments {
    #[arg(long)]
    pub source_trust: std::path::PathBuf,
    #[arg(long)]
    pub signed_output: std::path::PathBuf,
    #[arg(long)]
    pub issuer_key: std::path::PathBuf,
    #[arg(long)]
    pub holder_key: std::path::PathBuf,
    #[arg(long)]
    pub policy: std::path::PathBuf,
    #[arg(long)]
    pub output_parent: std::path::PathBuf,
    #[arg(long)]
    pub name: String,
}

pub fn run(cli: Cli) -> molten::error::Result<()> {
    match cli.command {
        Command::Create(args) => create(args),
        Command::Authorize(args) => credentials::authorize(args),
    }
}

fn create(args: Arguments) -> molten::error::Result<()> {
    let source = source::prepare(&args)?;
    let mut operator = operator::Authority::open(&args.issuer_key, &args.holder_key, &args.policy, &source.root)?;
    let policy_ref =
        source::record(&source.root, "node-service-host-policy-v1", vec![molten::preserves_rail::string(
            &operator.policy,
        )])?;
    let elapsed_ref = source::record(&source.root, "experimental-admitted-logical-time-source-v1", vec![
        molten::preserves_rail::string(&source.validation.selected_output_validation_ref),
        molten::preserves_rail::string(&policy_ref),
        molten::preserves_rail::string(operator.issuer_did()?),
        molten::preserves_rail::string("explicit-signed-operation-ticks-not-wall-clock"),
    ])?;
    let mut definitions = molten::harness::composition::group_definitions()?;
    definitions.group.elapsed_source_ref = elapsed_ref.clone();
    let sibling_group = molten::system_extension::RecoveryGroupDeclaration {
        group_id: "molten.composition.sibling-group".to_string(),
        members: vec![molten::system_extension::RecoveryGroupMember {
            member_id: definitions.sibling_facade.service_id.clone(),
            restart_class: molten::system_extension::RestartClass::Permanent,
            after: Vec::new(),
        }],
        window_ticks: 100,
        backoff_ticks: 5,
        max_restarts_per_member: 4,
        max_restarts_per_group: 3,
        elapsed_source_ref: elapsed_ref,
    };
    let context = service::Context::new(&source, &operator, &policy_ref)?;
    let mut services = Vec::with_capacity(3);
    for member in &definitions.group.members {
        services.push(service::build(&context, &member.member_id, &definitions.group, false)?);
    }
    services.push(service::build(&context, &definitions.sibling_facade.service_id, &sibling_group, false)?);
    let mut next = service::build(&context, &definitions.group.members[0].member_id, &definitions.group, true)?;
    next.context.instance_id = services[0].context.instance_id.clone();
    let bundle = molten::node_service_host::codec::Bundle {
        replacements: vec![molten::node_service_host::codec::ReplacementInput {
            previous_manifest_ref: services[0].executable.manifest_ref.clone(),
            service: next,
        }],
        services,
    };
    let bytes = molten::node_service_host::codec::encode(&bundle)?;
    operations::publish(&source.root, &bundle)?;
    let bundle_ref = molten::preserves_rail::content_ref_from_bytes(&bytes);
    source.root.write_new(&molten::node_state::RelativePath::parse("admission.preserves")?, &bytes)?;
    operator.publish(&source.root, &bundle_ref, &source.validation)?;
    let receipt = molten::preserves_rail::record("experimental-service-host-cohort-authored-v1", vec![
        molten::preserves_rail::string(&bundle_ref),
        molten::preserves_rail::string(&source.validation.selected_output_validation_ref),
        molten::preserves_rail::string(&source.validation.expected_startup_validation_ref),
        molten::preserves_rail::string(&source.callback_ref),
        molten::preserves_rail::string("not-a-runtime-test-or-production-admission"),
    ]);
    source.root.write_new(
        &molten::node_state::RelativePath::parse("authoring-receipt.preserves")?,
        &molten::preserves_rail::canonical_bytes(&receipt)?,
    )?;
    println!("{}", molten::preserves_rail::to_text(&receipt)?);
    Ok(())
}

fn invalid(message: impl Into<String>) -> molten::error::Failure {
    molten::error::Failure::invalid_harness(message)
}
