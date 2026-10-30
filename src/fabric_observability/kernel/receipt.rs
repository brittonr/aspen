//! Capability-rooted consumer of the actual OnixOS status export.
//! A status receipt is not a record stream and cannot satisfy window readiness.
use super::*;

#[cfg(test)]
#[path = "tests/receipt.rs"]
mod tests;

/// Has no loader, mutable handle, ambient path, credential, or host mutation API.
pub struct OnixBpfReader<'a> {
    root: &'a crate::node_state::Scope,
    path: crate::node_state::RelativePath,
    expected: OnixBpfStatusExpectation,
    closed: bool,
}

impl<'a> OnixBpfReader<'a> {
    pub fn open(
        root: &'a crate::node_state::Scope,
        path: crate::node_state::RelativePath,
        expected: OnixBpfStatusExpectation,
    ) -> Self {
        Self {
            root,
            path,
            expected,
            closed: false,
        }
    }

    // r[impl linux_ebpf_observability.boundary]
    pub fn poll(&mut self, cancelled: bool) -> Result<OnixBpfStatus, EbpfReadFailure> {
        if self.closed {
            return Err(EbpfReadFailure::Closed);
        }
        if cancelled {
            self.close();
            return Err(EbpfReadFailure::Cancelled);
        }
        let file = match self.root.observe_file(&self.path).map_err(|_| EbpfReadFailure::Permission)? {
            crate::node_state::FileObservation::Missing => return Err(EbpfReadFailure::Missing),
            crate::node_state::FileObservation::NonRegular(_) => return Err(EbpfReadFailure::Malformed),
            crate::node_state::FileObservation::Regular(file) => file,
        };
        if file.size() > ONIX_BPF_STATUS_MAX_BYTES {
            return Err(EbpfReadFailure::OverBound);
        }
        let bytes = file.read_bounded(ONIX_BPF_STATUS_MAX_BYTES).map_err(|_| EbpfReadFailure::Permission)?;
        let status = decode_onix_bpf_status(&bytes)?;
        if let Err(issue) = admit_onix_bpf_status(&status, &self.expected) {
            self.close();
            return Err(issue);
        }
        Ok(status)
    }

    /// Closing the reader never detaches programs or removes OnixOS-owned pins.
    pub fn close(&mut self) {
        self.closed = true;
    }
}
#[derive(serde::Deserialize)]
struct OnixBpfReport {
    runtime: OnixBpfStatus,
}

pub fn decode_onix_bpf_status(bytes: &[u8]) -> Result<OnixBpfStatus, EbpfReadFailure> {
    if bytes.len() as u64 > ONIX_BPF_STATUS_MAX_BYTES {
        return Err(EbpfReadFailure::OverBound);
    }
    // Ignore non-runtime claims without allocating them. Typed derives reject
    // duplicate runtime/status members and unknown behavior-affecting fields.
    let report: OnixBpfReport = serde_json::from_slice(bytes).map_err(|_| EbpfReadFailure::Malformed)?;
    validate_onix_bpf_status(&report.runtime)?;
    Ok(report.runtime)
}
