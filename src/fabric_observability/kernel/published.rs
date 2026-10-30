use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbpfEndpointFailure {
    Read(EbpfReadFailure),
    Admission(EbpfIssue),
    Timeout,
}

pub struct EbpfEndpointSelection<'a> {
    pub epoch: u64,
    pub deadline: u64,
    pub content_ref: Option<&'a str>,
}

pub struct EbpfEndpointReadback {
    pub window: EbpfWindow,
    /// Canonical packed facts sorted by source-local sequence, not delivery.
    pub endpoint_ref: String,
    /// External capture-byte reference, deliberately excluded from projection identity.
    pub wire_ref: String,
}

/// A single generation/epoch window capability. Never discovers sibling files.
pub struct EbpfEndpointReader<'a> {
    root: &'a crate::node_state::Scope,
    leaf: crate::node_state::RelativePath,
    adapter: &'a EbpfProfile,
    selection: EbpfEndpointSelection<'a>,
    closed: bool,
}

impl<'a> EbpfEndpointReader<'a> {
    // r[impl linux_ebpf_observability.boundary]
    pub fn open(
        root: &'a crate::node_state::Scope,
        leaf: crate::node_state::RelativePath,
        profile: &ObservationProfile,
        adapter: &'a EbpfProfile,
        selection: EbpfEndpointSelection<'a>,
    ) -> Result<Self, EbpfEndpointFailure> {
        validate_ebpf_profile(profile, adapter).map_err(EbpfEndpointFailure::Admission)?;
        if selection.epoch == 0 || selection.deadline == 0 {
            return Err(EbpfEndpointFailure::Admission(EbpfIssue::Binding));
        }
        if let Some(reference) = selection.content_ref {
            crate::preserves_rail::ContentRef::parse(reference)
                .map_err(|_| EbpfEndpointFailure::Admission(EbpfIssue::Binding))?;
        }
        Ok(Self {
            root,
            leaf,
            adapter,
            selection,
            closed: false,
        })
    }

    pub fn poll(&mut self, now: u64, cancelled: bool) -> Result<EbpfEndpointReadback, EbpfEndpointFailure> {
        if self.closed {
            return Err(EbpfEndpointFailure::Read(EbpfReadFailure::Closed));
        }
        if cancelled {
            self.close();
            return Err(EbpfEndpointFailure::Read(EbpfReadFailure::Cancelled));
        }
        if now > self.selection.deadline {
            self.close();
            return Err(EbpfEndpointFailure::Timeout);
        }
        let file = match self
            .root
            .observe_file(&self.leaf)
            .map_err(|_| EbpfEndpointFailure::Read(EbpfReadFailure::Permission))?
        {
            crate::node_state::FileObservation::Missing => {
                return Err(EbpfEndpointFailure::Read(EbpfReadFailure::Missing));
            }
            crate::node_state::FileObservation::NonRegular(_) => {
                self.close();
                return Err(EbpfEndpointFailure::Read(EbpfReadFailure::Malformed));
            }
            crate::node_state::FileObservation::Regular(file) => file,
        };
        if file.size() > self.adapter.max_bytes as u64 {
            self.close();
            return Err(EbpfEndpointFailure::Read(EbpfReadFailure::OverBound));
        }
        let bytes = file
            .read_bounded(self.adapter.max_bytes as u64)
            .map_err(|_| EbpfEndpointFailure::Read(EbpfReadFailure::Permission))?;
        // An immutable published epoch is consumed once. Drift/malformed reads
        // require a new independently admitted capability rather than retry merge.
        self.close();
        let wire_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
        if self.selection.content_ref.is_some_and(|expected| expected != wire_ref) {
            return Err(EbpfEndpointFailure::Admission(EbpfIssue::Binding));
        }
        let window = decode_ebpf_window(&bytes, self.adapter.max_bytes as u64)?;
        admit_ebpf_window(self.adapter, &window, self.selection.epoch, now).map_err(EbpfEndpointFailure::Admission)?;
        let endpoint_ref =
            canonical_ebpf_window_ref(&window).map_err(|_| EbpfEndpointFailure::Admission(EbpfIssue::Canonical))?;
        Ok(EbpfEndpointReadback {
            window,
            endpoint_ref,
            wire_ref,
        })
    }

    /// No map/link handles are retained and no BPF state is detached or removed.
    pub fn close(&mut self) {
        self.closed = true;
    }
}

pub fn decode_ebpf_window(bytes: &[u8], max_bytes: u64) -> Result<EbpfWindow, EbpfEndpointFailure> {
    if max_bytes == 0 || max_bytes > 4 * 1024 * 1024 || bytes.len() as u64 > max_bytes {
        return Err(EbpfEndpointFailure::Read(EbpfReadFailure::OverBound));
    }
    let mut window: EbpfWindow =
        serde_json::from_slice(bytes).map_err(|_| EbpfEndpointFailure::Read(EbpfReadFailure::Malformed))?;
    window.encoded_bytes = bytes.len();
    Ok(window)
}

pub fn decode_ebpf_profile(bytes: &[u8]) -> Result<EbpfProfileExport, EbpfIssue> {
    if bytes.len() > 64 * 1024 {
        return Err(EbpfIssue::Bounds);
    }
    serde_json::from_slice(bytes).map_err(|_| EbpfIssue::Mapping)
}
