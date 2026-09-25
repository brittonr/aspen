
const STATUS_RECORD: &str = "native-host-status-v2";
const CLAIM_LEVEL: &str = "local-live-materialized-values-pilot";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeServiceError {
    Host(String),
    Journal(NativeJournalError),
    Executor(NativeExecutorError),
    StatePoisoned,
    Admission(Vec<NativeHostIssue>),
    MissingCheckpoint,
}

impl From<crate::error::MoltenError> for NativeServiceError {
    fn from(error: crate::error::MoltenError) -> Self {
        Self::Host(error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeServiceIngressResult {
    pub admission: NativeIngressAdmission,
    pub dispatch: HostDispatchResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalNativeServiceStatus {
    pub status_ref: String,
    pub claim_level: String,
    pub operator: CanonicalOperatorStatus,
    pub recovery: Vec<NativeRecoveryInventory>,
    pub non_claims: Vec<NativeHostNonClaim>,
}

pub trait NativeServiceIngressPort {
    type Error;

    fn submit(
        &mut self,
        ingress: &NativeIngressEnvelope,
        logical_tick: u64,
    ) -> std::result::Result<NativeServiceIngressResult, Self::Error>;
}

pub struct NativeServiceClient<'a, T: NativeServiceIngressPort> {
    port: &'a mut T,
}

impl<'a, T: NativeServiceIngressPort> NativeServiceClient<'a, T> {
    pub fn new(port: &'a mut T) -> Self {
        Self { port }
    }

    pub fn submit(
        &mut self,
        ingress: &NativeIngressEnvelope,
        logical_tick: u64,
    ) -> std::result::Result<NativeServiceIngressResult, T::Error> {
        self.port.submit(ingress, logical_tick)
    }
}

pub struct NativeSystemExtensionService<P, J>
where
    P: crate::fabric_execution::ExecutionFabricPort,
    J: NativeHostJournal,
{
    profile: AdmittedNativeHostProfile,
    executable: AdmittedNativeExecutable,
    journal: std::sync::Arc<std::sync::Mutex<J>>,
    instance: std::sync::Arc<std::sync::Mutex<NativeInstanceRecord>>,
    host: SystemExtensionHost<NativeProcessSystemExtensionExecutor<P, J>>,
}

/// The admitted native host profile, executable evidence, and manifest a native service is built
/// from.
pub struct AdmissionSet {
    pub profile: AdmittedNativeHostProfile,
    pub executable: AdmittedNativeExecutable,
    pub admitted: CanonicalAdmittedSystemExtensionManifest,
}
