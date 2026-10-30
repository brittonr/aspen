//! Optional bounded diagnostic projection, never canonical evidence or an authority source.
use std::collections;

#[derive(Clone, Copy)]
pub enum Status {
    Returned,
    Denied,
    Uncertain,
}
#[derive(Default)]
pub struct Diagnostics {
    queue: collections::VecDeque<preserves::IOValue>,
}
impl Diagnostics {
    // r[impl molten.component_rpc.telemetry]
    /// Closed fields prevent credentials, payloads, arbitrary labels and high cardinality IDs.
    /// Export admission is independent and must be performed by the owning telemetry port.
    pub fn observe(&mut self, receipt: &str, status: Status) -> super::Result<()> {
        crate::codec::validate_content_ref(receipt).map_err(super::failure)?;
        if self.queue.len() >= 32 {
            return Err("diagnostic queue is full; canonical receipts unaffected".into());
        }
        let status = match status {
            Status::Returned => "returned",
            Status::Denied => "denied",
            Status::Uncertain => "uncertain",
        };
        self.queue.push_back(crate::codec::record("component-rpc-diagnostic-v1", vec![
            crate::codec::string("diagnostic-only"),
            crate::codec::string(receipt),
            crate::codec::string(super::PROFILE),
            crate::codec::string(status),
        ]));
        Ok(())
    }
    pub fn drain(&mut self) -> impl Iterator<Item = preserves::IOValue> + '_ {
        self.queue.drain(..)
    }
}
