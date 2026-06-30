#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeHostcall {
    Send,
    Subscribe,
    BlobGet,
    BlobPut,
}

impl RuntimeHostcall {
    pub fn capability_name(self) -> &'static str {
        match self {
            Self::Send => "hostcall:send",
            Self::Subscribe => "hostcall:subscribe",
            Self::BlobGet => "hostcall:blob-get",
            Self::BlobPut => "hostcall:blob-put",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HostcallAdmission {
    pub hostcall: RuntimeHostcall,
    pub envelope_ref: String,
    pub capability: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WasiCapabilityProfile {
    pub filesystem: bool,
    pub clock: bool,
    pub environment: bool,
    pub sockets: bool,
}

impl WasiCapabilityProfile {
    pub fn deny_all() -> Self {
        Self {
            filesystem: false,
            clock: false,
            environment: false,
            sockets: false,
        }
    }

    pub fn admits_no_ambient_access(&self) -> bool {
        !self.filesystem && !self.clock && !self.environment && !self.sockets
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ComponentInspection {
    pub imports: Vec<String>,
    pub admitted: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SteelOrchestrationRecord {
    pub script_ref: String,
    pub operation: String,
    pub envelope_ref: String,
}

pub fn admit_hostcall(
    envelope: &super::Envelope,
    hostcall: RuntimeHostcall,
) -> std::result::Result<HostcallAdmission, super::RuntimeBoundaryError> {
    let required = hostcall.capability_name();
    let has_capability = envelope.capabilities.iter().any(|capability| capability.as_str() == required);
    if !has_capability {
        return Err(super::RuntimeBoundaryError::denied_operation(
            "wasmtime-hostcall",
            format!("missing capability {required}"),
        ));
    }
    Ok(HostcallAdmission {
        hostcall,
        envelope_ref: envelope
            .canonical_hash()
            .map_err(|error| super::RuntimeBoundaryError::invalid_input("wasmtime-hostcall", error.to_string()))?,
        capability: required.to_string(),
    })
}

pub fn inspect_component_imports(imports: &[String], allowed: &[RuntimeHostcall]) -> ComponentInspection {
    let allowed_names: Vec<&str> = allowed.iter().map(|hostcall| hostcall.capability_name()).collect();
    let mut diagnostics = Vec::with_capacity(imports.len());
    for import in imports {
        if !allowed_names.contains(&import.as_str()) {
            diagnostics.push(format!("unsupported component import {import}"));
        }
    }
    ComponentInspection {
        imports: imports.to_vec(),
        admitted: diagnostics.is_empty(),
        diagnostics,
    }
}

pub fn steel_orchestration_record(
    script_ref: impl Into<String>,
    operation: impl Into<String>,
    envelope: &super::Envelope,
) -> std::result::Result<SteelOrchestrationRecord, super::RuntimeBoundaryError> {
    Ok(SteelOrchestrationRecord {
        script_ref: script_ref.into(),
        operation: operation.into(),
        envelope_ref: envelope
            .canonical_hash()
            .map_err(|error| super::RuntimeBoundaryError::invalid_input("steel-orchestration", error.to_string()))?,
    })
}

pub fn hostcall_capability(
    value: RuntimeHostcall,
) -> std::result::Result<super::Capability, super::RuntimeBoundaryError> {
    super::Capability::parse(value.capability_name())
        .map_err(|error| super::RuntimeBoundaryError::invalid_input("hostcall-capability", error.to_string()))
}

#[cfg(test)]
mod tests {
    fn envelope_with_caps(caps: Vec<crate::runtime::Capability>) -> crate::runtime::Envelope {
        crate::runtime::Envelope::new(crate::runtime::EnvelopeInput {
            sender: crate::runtime::ActorId::parse("actor:wasm").expect("sender"),
            subject: crate::runtime::RuntimeValue::string("service.ready").expect("subject"),
            body: crate::runtime::RuntimeValue::string("ready").expect("body"),
            blob_refs: vec![
                crate::runtime::ContentRef::parse(crate::preserves_rail::content_ref_from_bytes(b"blob"))
                    .expect("blob"),
            ],
            capabilities: caps,
            evidence_refs: vec![
                crate::runtime::EvidenceRef::parse(crate::preserves_rail::content_ref_from_bytes(b"evidence"))
                    .expect("evidence"),
            ],
        })
        .expect("envelope")
    }

    #[test]
    fn wasmtime_hostcall_requires_matching_capability() {
        let envelope =
            envelope_with_caps(vec![super::hostcall_capability(super::RuntimeHostcall::Send).expect("hostcall cap")]);
        let admission = super::admit_hostcall(&envelope, super::RuntimeHostcall::Send).expect("send admitted");
        assert_eq!(admission.capability, "hostcall:send");

        let error = super::admit_hostcall(&envelope, super::RuntimeHostcall::BlobGet).expect_err("blob get denied");
        assert_eq!(error.category(), crate::runtime::RuntimeErrorCategory::DeniedOperation);
    }

    #[test]
    fn wasi_profile_denies_ambient_access_by_default() {
        let profile = super::WasiCapabilityProfile::deny_all();
        assert!(profile.admits_no_ambient_access());
    }

    #[test]
    fn component_inspection_rejects_unadmitted_import() {
        let imports = vec!["hostcall:send".to_string(), "hostcall:socket".to_string()];
        let inspection = super::inspect_component_imports(&imports, &[super::RuntimeHostcall::Send]);
        assert!(!inspection.admitted);
        assert!(inspection.diagnostics[0].contains("hostcall:socket"));
    }

    #[test]
    fn steel_orchestration_binds_script_operation_and_envelope() {
        let envelope = envelope_with_caps(vec![
            super::hostcall_capability(super::RuntimeHostcall::Subscribe).expect("hostcall cap"),
        ]);
        let record = super::steel_orchestration_record(
            crate::preserves_rail::content_ref_from_bytes(b"script"),
            "spawn-inspect",
            &envelope,
        )
        .expect("steel record");
        assert_eq!(record.operation, "spawn-inspect");
        assert_eq!(record.envelope_ref, envelope.canonical_hash().expect("envelope ref"));
    }
}
