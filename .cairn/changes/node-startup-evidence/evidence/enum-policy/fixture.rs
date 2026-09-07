#![feature(register_tool)]
#![register_tool(octet)]
// Reduced policy models, not copies of the complete filesystem adapter.
#[cfg_attr(real_marker, octet::sealed_enum)]
#[cfg_attr(doc_marker, doc = "This text mentions sealed_enum but declares no sealing attribute.")]
pub enum LocalStoreKind { Artifact, Ledger, #[cfg(future)] Future }
#[cfg_attr(real_marker, octet::sealed_enum)]
#[cfg_attr(doc_marker, doc = "This text mentions sealed_enum but declares no sealing attribute.")]
pub enum NodeStateNamespaceKind { Identity, Ledger, #[cfg(future)] Future }
#[cfg_attr(real_marker, octet::sealed_enum)]
#[cfg_attr(doc_marker, doc = "This text mentions sealed_enum but declares no sealing attribute.")]
pub enum NodeStateFileObservation { Missing, NonRegular, Regular(u32), #[cfg(future)] Future }
pub fn store_name(value: LocalStoreKind) -> &'static str {
    match value { LocalStoreKind::Artifact => "artifact", LocalStoreKind::Ledger => "ledger" }
}
pub fn namespace_name(value: NodeStateNamespaceKind) -> &'static str {
    match value { NodeStateNamespaceKind::Identity => "identity", NodeStateNamespaceKind::Ledger => "ledger" }
}
pub fn mode(value: NodeStateFileObservation) -> Result<Option<u32>, &'static str> {
    match value {
        NodeStateFileObservation::Missing => Ok(None),
        NodeStateFileObservation::NonRegular => Err("not regular"),
        NodeStateFileObservation::Regular(mode) => Ok(Some(mode)),
    }
}
pub fn read(value: NodeStateFileObservation) -> Result<u32, &'static str> {
    match value {
        NodeStateFileObservation::Missing => Err("missing"),
        NodeStateFileObservation::NonRegular => Err("not regular"),
        NodeStateFileObservation::Regular(value) => Ok(value),
    }
}
