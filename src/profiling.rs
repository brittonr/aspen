pub const DEVELOPMENT_PROFILER_APP: &str = "molten-dev";
pub const PROFILER_TRACE_EXTENSION: &str = "fxt";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfilerArtifactRole {
    DevelopmentObservation,
    CairnReceipt,
    DeterminismEvidence,
    ReleaseReadiness,
    ValenceEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfilerArtifactError {
    NotProfilerTrace,
    EvidenceRoleDenied(ProfilerArtifactRole),
}

pub fn admit_profiler_artifact(
    path: &std::path::Path,
    role: ProfilerArtifactRole,
) -> Result<(), ProfilerArtifactError> {
    let is_trace = path.extension().and_then(|extension| extension.to_str()) == Some(PROFILER_TRACE_EXTENSION);
    if !is_trace {
        return Err(ProfilerArtifactError::NotProfilerTrace);
    }
    if role != ProfilerArtifactRole::DevelopmentObservation {
        return Err(ProfilerArtifactError::EvidenceRoleDenied(role));
    }
    Ok(())
}

#[cfg(all(feature = "profiler", target_arch = "x86_64", target_os = "linux"))]
pub fn enable_development_profiler() {
    flux_profiler::enable_profiler(DEVELOPMENT_PROFILER_APP);
}

#[cfg(not(all(feature = "profiler", target_arch = "x86_64", target_os = "linux")))]
pub fn enable_development_profiler() {}

#[cfg(feature = "profiler-alloc")]
#[global_allocator]
static PROFILING_ALLOCATOR: flux_profiler::allocator::CountingAllocator<std::alloc::System> =
    flux_profiler::allocator::CountingAllocator(std::alloc::System);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_trace_is_an_observation() {
        assert_eq!(
            admit_profiler_artifact(
                std::path::Path::new("bounded-development-capture.fxt"),
                ProfilerArtifactRole::DevelopmentObservation,
            ),
            Ok(())
        );
    }

    #[test]
    fn profiler_trace_is_denied_as_release_evidence() {
        let roles = [
            ProfilerArtifactRole::CairnReceipt,
            ProfilerArtifactRole::DeterminismEvidence,
            ProfilerArtifactRole::ReleaseReadiness,
            ProfilerArtifactRole::ValenceEvidence,
        ];
        for role in roles {
            assert_eq!(
                admit_profiler_artifact(std::path::Path::new("capture.fxt"), role),
                Err(ProfilerArtifactError::EvidenceRoleDenied(role))
            );
        }
    }

    #[test]
    fn unrelated_artifact_is_not_misclassified_as_a_trace() {
        assert_eq!(
            admit_profiler_artifact(std::path::Path::new("receipt.json"), ProfilerArtifactRole::DevelopmentObservation,),
            Err(ProfilerArtifactError::NotProfilerTrace)
        );
    }
}
