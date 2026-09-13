use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DeliveryPortCall {
    Load,
    CompareAndCommit,
    TimerIntents,
    PublishStatus,
}

pub(super) type DeliveryCallTrace = Arc<Mutex<Vec<DeliveryPortCall>>>;

pub(super) fn record_port_call(trace: &Option<DeliveryCallTrace>, call: DeliveryPortCall) {
    if let Some(trace) = trace {
        trace.lock().expect("test call trace").push(call);
    }
}
