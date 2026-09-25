
pub const REQUIRED_RUNTIME_ADAPTERS: &[&str] = &[
    "ledger",
    "registry",
    "chunks",
    "storage",
    "cache",
    "remote-dataspace",
    "services",
    "jobs",
    "coordination",
    "plugin-host",
    "catalog-mcp",
    "control",
];

const MAX_NODE_ADAPTERS: usize = 16;
const MAX_NODE_SOURCE_GATE_RECEIPTS: usize = 16;
const MAX_NODE_DIAGNOSTICS: usize = 64;

const _: () = assert!(MAX_NODE_ADAPTERS >= REQUIRED_RUNTIME_ADAPTERS.len());
const _: () = assert!(MAX_NODE_SOURCE_GATE_RECEIPTS > 0);
