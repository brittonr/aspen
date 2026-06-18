#[path = "node/command.rs"]
pub(crate) mod command;

#[path = "node/authority.rs"]
mod authority;
#[path = "node/control.rs"]
mod control;
#[path = "node/core.rs"]
mod core;
#[path = "node/health.rs"]
mod health;
#[path = "node/lifecycle.rs"]
mod lifecycle;
#[path = "node/workflow.rs"]
mod workflow;

pub(crate) type Command = command::Top;

pub(crate) fn run(command: Command) -> molten::error::Result<()> {
    match command {
        command::Top::Init(input) => lifecycle::init(input),
        command::Top::Run(input) => lifecycle::run(input),
        command::Top::RunLoop(input) => lifecycle::run_loop(input),
        command::Top::Serve(input) => lifecycle::serve(input),
        command::Top::Status(input) => lifecycle::status(input),
        command::Top::Stop(input) => lifecycle::stop(input),
        command::Top::Show(input) => lifecycle::show(input),
        command::Top::ControlRequest(input) => authority::control_request(input),
        command::Top::ProvenanceFixture(input) => authority::provenance_fixture(input),
        command::Top::AuthorityGrantFixture(input) => authority::grant_fixture(input),
        command::Top::AuthorityGrantImport(input) => authority::grant_import(input),
        command::Top::SupervisorPolicyFixture(input) => authority::policy_fixture(input),
        command::Top::LiveTicketExport(input) => authority::ticket_export(input),
        command::Top::LiveTicketImport(input) => authority::ticket_import(input),
        command::Top::LivePeerAdmit(input) => authority::peer_admit(input),
        command::Top::ControlSubmit(input) => control::submit(input),
        command::Top::ControlDispatch(input) => control::dispatch(input),
        command::Top::ControlIngressBuild(input) => control::ingress::build(input),
        command::Top::ControlIngressLiveBuild(input) => control::ingress::live_build(input),
        command::Top::ControlIngressLiveLoopback(input) => control::ingress::live_loopback(input),
        command::Top::ControlIngressLiveSend(input) => control::ingress::live_send(input),
        command::Top::LiveWorkflowBundle(input) => workflow::bundle::run(input),
        command::Top::LiveWorkflowBundleExport(input) => workflow::bundle::export(input),
        command::Top::LiveWorkflowBundleVerify(input) => workflow::bundle::verify(input),
        command::Top::LiveWorkflowBundleGate(input) => workflow::gate::run(input),
        command::Top::LiveWorkflowBundleApply(input) => workflow::apply::run(input),
        command::Top::LiveWorkflowBundleReconcile(input) => workflow::ack::reconcile(input),
        command::Top::LiveWorkflowBundleAckExport(input) => workflow::ack::export(input),
        command::Top::LiveWorkflowBundleAckImport(input) => workflow::ack::import(input),
        command::Top::LiveWorkflowBundleProtocolGate(input) => workflow::ack::protocol_gate(input),
        command::Top::LiveWorkflowBundleImport(input) => workflow::bundle::import(input),
        command::Top::ControlIngressPublish(input) => control::ingress::publish(input),
        command::Top::ControlIngressDeliver(input) => control::ingress::deliver(input),
        command::Top::ControlDeny(input) => control::deny(input),
        command::Top::Shutdown(input) => health::shutdown(input),
        command::Top::Health(input) => health::restart(input),
    }
}
