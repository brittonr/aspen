/// A selected live or replay host together with its own authority and recording.
/// Private fields prevent pairing one host with another host's completion handle.
pub struct Binding<H> {
    pub(super) host: H,
    pub(super) import_grants: [crate::wasm_component::ImportGrant; 1],
    pub(super) recording: crate::wasm_component::consumer::host::Recording,
    pub(super) is_replay: bool,
}

impl<C, V> Binding<crate::wasm_component::consumer::host::Live<C, V>>
where
    C: crate::wasm_component::consumer::host::CurrentContext,
    V: crate::system_extension::NativeCallbackValuePort,
{
    pub fn live(mut host: crate::wasm_component::consumer::host::Live<C, V>) -> Self {
        let (grant, recording) = host.callback_binding();
        Self {
            host,
            import_grants: [grant],
            recording,
            is_replay: false,
        }
    }
}

impl Binding<crate::wasm_component::consumer::host::Replay> {
    /// Replay authority is the explicitly selected recorded grant, never live state.
    pub fn replay(
        host: crate::wasm_component::consumer::host::Replay,
        grant: crate::wasm_component::ImportGrant,
    ) -> Self {
        let recording = host.callback_recording();
        Self {
            host,
            import_grants: [grant],
            recording,
            is_replay: true,
        }
    }
}

/// Select live Session or exact Replay at the product root, never in the guest.
///
/// A factory must derive CurrentInput from current host-owned admission facts;
/// guest bytes cannot populate authority, grants, or lifecycle fences. The shared
/// `Live` and `Replay` implement the returned host interface.
pub trait ComponentCallbackHostFactory {
    type Host: crate::wasm_component::EffectRequestHost + 'static;

    fn create(
        &mut self,
        context: &crate::system_extension::CallbackContext,
        invocation: &crate::system_extension::CallbackInvocation,
    ) -> crate::wasm_component::Result<Binding<Self::Host>>;
}

impl<F, H> ComponentCallbackHostFactory for F
where
    F: FnMut(
        &crate::system_extension::CallbackContext,
        &crate::system_extension::CallbackInvocation,
    ) -> crate::wasm_component::Result<Binding<H>>,
    H: crate::wasm_component::EffectRequestHost + 'static,
{
    type Host = H;

    fn create(
        &mut self,
        context: &crate::system_extension::CallbackContext,
        invocation: &crate::system_extension::CallbackInvocation,
    ) -> crate::wasm_component::Result<Binding<H>> {
        self(context, invocation)
    }
}

/// An attempted publication identifies the exact materialized outcome value,
/// retains even invalid receipts, and preserves structured acceptance uncertainty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationObservation {
    pub value_ref: String,
    pub byte_count: u64,
    pub receipt: Option<crate::system_extension::ValuePublicationReceipt>,
    pub failure: Option<crate::system_extension::ValuePortFailure>,
}

/// Full canonical runtime receipts survive callback decoding/admission failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedObservation {
    pub envelope: crate::system_extension::CanonicalCallbackEnvelope,
    pub execution: crate::wasm_component::ExecutionOutcome,
    pub host_records: crate::wasm_component::Result<crate::wasm_component::consumer::host::Records>,
    pub materialized_outcome: Option<crate::system_extension::MaterializedCallbackOutcome>,
    pub publication_refs: Vec<String>,
    pub publications: Vec<PublicationObservation>,
    pub is_publication_uncertain: bool,
    pub is_committed: bool,
}
