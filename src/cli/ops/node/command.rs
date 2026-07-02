#[path = "command/authority.rs"]
pub(crate) mod authority;
#[path = "command/base.rs"]
pub(crate) mod base;
#[path = "command/control.rs"]
pub(crate) mod control;
#[path = "command/health.rs"]
pub(crate) mod health;
#[path = "command/iroh.rs"]
pub(crate) mod transport;
pub(crate) mod iroh {
    pub(crate) use super::transport::*;
}
#[path = "command/live.rs"]
pub(crate) mod live;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Top {
    Init(base::Init),
    Run(base::Run),
    RunLoop(base::RunLoop),
    Serve(base::Serve),
    Status(base::Status),
    Stop(base::Stop),
    Show(base::Show),
    ControlRequest(authority::Request),
    ProvenanceFixture(authority::Provenance),
    AuthorityGrantFixture(authority::GrantFixture),
    AuthorityGrantImport(authority::GrantImport),
    SupervisorPolicyFixture(authority::PolicyFixture),
    LiveTicketExport(authority::TicketExport),
    LiveTicketImport(authority::TicketImport),
    LivePeerAdmit(authority::PeerAdmit),
    ControlSubmit(control::Submit),
    ControlDispatch(control::Dispatch),
    ControlIngressBuild(control::IngressBuild),
    ControlIngressLiveBuild(control::IngressLiveBuild),
    ControlIngressLiveLoopback(control::IngressLiveLoopback),
    ControlIngressLiveSend(control::IngressLiveSend),
    LiveWorkflowBundle(live::Bundle),
    LiveWorkflowBundleExport(live::Export),
    LiveWorkflowBundleVerify(live::Verify),
    LiveWorkflowBundleGate(live::Gate),
    LiveWorkflowBundleApply(live::Apply),
    LiveWorkflowBundleReconcile(live::Reconcile),
    LiveWorkflowBundleAckExport(live::AckExport),
    LiveWorkflowBundleAckImport(live::AckImport),
    LiveWorkflowBundleProtocolGate(live::ProtocolGate),
    LiveWorkflowBundleImport(live::Import),
    ControlIngressPublish(control::IngressPublish),
    ControlIngressDeliver(control::IngressDeliver),
    ControlDeny(control::Deny),
    IrohRouterFixture(iroh::RouterFixture),
    IrohFrameFixture(iroh::FrameFixture),
    NetworkDiagnosticsFixture(iroh::DiagnosticsFixture),
    MetricsSnapshotFixture(iroh::MetricsFixture),
    PortMappingFixture(iroh::PortMappingFixture),
    ExternalDiagnosticsBridgeFixture(iroh::ExternalBridgeFixture),
    Shutdown(health::Shutdown),
    Health(health::Restart),
}
