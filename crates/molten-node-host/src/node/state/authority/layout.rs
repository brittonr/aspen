//! The fixed namespaces a node-state root is divided into.

const NODE_STATE_NAMESPACE_COUNT: usize = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NamespaceKind {
    Identity,
    Secrets,
    Ledger,
    ControlInbox,
    ControlOutbox,
    ControlIngress,
    ControlIdempotency,
    ControlService,
    Services,
    Receipts,
    Registry,
    Chunks,
    Storage,
    Ingress,
}

impl NamespaceKind {
    pub const ALL: [Self; NODE_STATE_NAMESPACE_COUNT] = [
        Self::Identity,
        Self::Secrets,
        Self::Ledger,
        Self::ControlInbox,
        Self::ControlOutbox,
        Self::ControlIngress,
        Self::ControlIdempotency,
        Self::ControlService,
        Self::Services,
        Self::Receipts,
        Self::Registry,
        Self::Chunks,
        Self::Storage,
        Self::Ingress,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Identity | Self::Secrets => "identity",
            Self::Ledger => "ledger",
            Self::ControlInbox => "control/inbox",
            Self::ControlOutbox => "control/outbox",
            Self::ControlIngress => "control/iroh-ingress",
            Self::ControlIdempotency => "control/idempotency",
            Self::ControlService => "control/service",
            Self::Services => "services",
            Self::Receipts => "receipts",
            Self::Registry => "registry",
            Self::Chunks => "chunks",
            Self::Storage => "storage",
            Self::Ingress => "control/iroh-ingress",
        }
    }
}
