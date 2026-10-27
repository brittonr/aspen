use basalt::ucan::HolderInvocationReplayAdmission;
use basalt::ucan::TokenSigner;

use crate::system_extension::NativeCallbackValuePort;

pub(super) const POLICY: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/wasm-component-runtime/generated/host-policy.json",));

pub(super) struct Fixture {
    pub(super) workspace: crate::test_support::Workspace,
    pub keys: Keys,
    pub token: basalt::ucan::CompactToken,
    pub scope: super::super::Scope,
    pub contract: Contract,
    pub output: preserves::IOValue,
    pub payload: preserves::IOValue,
    pub replay: NonceAdmission,
    pub policy_json: String,
}

pub(super) struct Keys {
    pub holder: basalt::ucan::Ed25519InMemorySigner,
    pub issuer: String,
    pub audience: basalt::ucan::AudienceDid,
    pub resolver: basalt::ucan::KeyResolutionContext,
}

pub(super) struct Contract {
    pub manifest: preserves::IOValue,
    pub profile: preserves::IOValue,
    pub grant: crate::wasm_component::ImportGrant,
    pub limits: super::super::Limits,
    pub capabilities: Vec<String>,
}

pub(super) struct NoCaveats;
impl basalt::ucan::CaveatPolicySet for NoCaveats {
    fn policy_for(&self, _caveat: &basalt::ucan::CaveatIdentifier) -> Option<&dyn basalt::ucan::CaveatPolicy> {
        None
    }
}

pub(super) struct NonceAdmission(std::cell::Cell<bool>);
impl HolderInvocationReplayAdmission for NonceAdmission {
    fn admit_holder_invocation(
        &self,
        _candidate: &basalt::ucan::HolderInvocationReplayCandidate<'_>,
    ) -> std::result::Result<(), basalt::ucan::HolderInvocationReplayError> {
        if self.0.replace(true) {
            Err(basalt::ucan::HolderInvocationReplayError::Duplicate)
        } else {
            Ok(())
        }
    }
}

impl Fixture {
    pub fn new() -> Self {
        Self::with_policy(POLICY.to_string())
    }

    pub fn with_policy(policy_json: String) -> Self {
        let output = crate::preserves_rail::record("blob-value", vec![crate::preserves_rail::string("measured bytes")]);
        let object_ref = crate::preserves_rail::canonical_hash(&output).expect("object ref");
        let payload = crate::preserves_rail::record("blob-get", vec![crate::preserves_rail::string(&object_ref)]);
        let (keys, token) = keys_and_token(&object_ref);
        let (contract, scope) = contract(&keys, &token, &policy_json);
        Self {
            workspace: crate::test_support::Workspace::new("component-requests").expect("workspace"),
            keys,
            token,
            scope,
            contract,
            output,
            payload,
            policy_json,
            replay: NonceAdmission(std::cell::Cell::new(false)),
        }
    }

    pub fn values(&self) -> crate::system_extension::CapabilityCallbackValuePort {
        let root = self.workspace.ledger().expect("ledger capability");
        let namespace = crate::node_state::Scope::from_dir(
            crate::node_state::NamespaceKind::Ledger,
            root.dir().try_clone().expect("clone capability"),
        )
        .expect("namespace");
        let mut values = crate::system_extension::CapabilityCallbackValuePort::new(namespace).expect("value port");
        let bytes = crate::preserves_rail::canonical_bytes(&self.output).expect("blob bytes");
        values
            .publish(
                &crate::system_extension::NativeCallbackValue {
                    value_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
                    bytes,
                },
                crate::wasm_component::HOST_PAYLOAD_BYTES,
            )
            .expect("publish actual blob");
        values
    }

    pub fn request(&self, sequence: u64) -> Vec<u8> {
        let request = crate::effects::effect_request_value(&crate::effects::EffectRequestInput {
            artifact_ref: reference("artifact"),
            effect_id: "blob.get".to_string(),
            operation: "get".to_string(),
            handler_profile: "production".to_string(),
            input_ref: crate::preserves_rail::canonical_hash(&self.payload).expect("payload ref"),
            capability_refs: self.contract.capabilities.clone(),
            evidence_refs: Vec::new(),
        })
        .expect("effect request");
        crate::wasm_component::encode_host_request(crate::wasm_component::HostRequestInput {
            generation: 3,
            incarnation: 5,
            sequence,
            grant: &self.contract.grant,
            effect: &request,
            payload: &self.payload,
        })
        .expect("carrier")
    }

    pub fn sign(&self, bytes: &[u8]) -> basalt::ucan::SignerSignature {
        let request = crate::wasm_component::decode_host_request(bytes).expect("request");
        let operation = super::super::authority::holder_binding(&request, &self.scope).expect("binding");
        basalt::ucan::sign_holder_invocation(
            &basalt::ucan::HolderInvocationSigningRequest::new(
                self.token.as_str(),
                &self.keys.audience,
                &super::super::authority::blob_resource(&request).expect("resource"),
                super::super::authority::BLOB_ABILITY,
                operation,
                basalt::ucan::HolderInvocationNonce::from_bytes([7; 32]),
            ),
            &self.keys.holder,
            basalt::ucan::VerificationLimits::default(),
            basalt::ucan::HolderInvocationLimits::default(),
        )
        .expect("holder signature")
    }

    pub fn services(&self) -> basalt::ucan_enforcement::UcanEnforcementBoundaries<'_> {
        basalt::ucan_enforcement::UcanEnforcementBoundaries {
            verification_time: basalt::ucan::VerificationTime::from_unix_seconds(150),
            key_resolver: &self.keys.resolver,
            proof_store: &EMPTY_PROOFS,
            revocations: &basalt::ucan::NoRevocations,
            caveat_policies: &NoCaveats,
            replay: &self.replay,
            verification_limits: basalt::ucan::VerificationLimits::default().with_clock_skew_tolerance_seconds(0),
            holder_limits: basalt::ucan::HolderInvocationLimits::default(),
        }
    }

    pub fn current<'a>(
        &'a self,
        services: &'a basalt::ucan_enforcement::UcanEnforcementBoundaries<'a>,
        signature: &'a basalt::ucan::SignerSignature,
    ) -> super::super::CurrentInput<'a> {
        super::super::CurrentInput {
            generation: 3,
            incarnation: 5,
            effect_manifest: &self.contract.manifest,
            handler_profile: &self.contract.profile,
            grant: &self.contract.grant,
            capability_refs: &self.contract.capabilities,
            limits: self.contract.limits,
            authority: super::super::authority::Input {
                policy_json: &self.policy_json,
                trusted_issuer: &self.keys.issuer,
                expected_holder: self.keys.audience.as_str(),
                credential: super::super::authority::Credential {
                    compact_token: self.token.as_str(),
                    nonce: basalt::ucan::HolderInvocationNonce::from_bytes([7; 32]),
                    holder_signature: signature,
                },
                services,
            },
        }
    }

    pub fn session(&self) -> super::super::Session {
        super::super::Session::new(self.scope.clone(), 3, 5).expect("session")
    }
}

static EMPTY_PROOFS: basalt::ucan::ProofCollection = basalt::ucan::ProofCollection::empty();

fn keys_and_token(object_ref: &str) -> (Keys, basalt::ucan::CompactToken) {
    let issuer = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([11; 32]);
    let holder = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([13; 32]);
    let audience = basalt::ucan::AudienceDid::from(holder.issuer().expect("holder DID"));
    let capabilities = basalt::ucan::CapabilitySet::new(vec![
        basalt::ucan::CapabilityDocument::new(format!("molten://blob/{object_ref}"), "blob/get".to_string())
            .expect("capability"),
    ])
    .expect("capability set");
    let token = basalt::ucan::issue_token_with_signer(
        &issuer,
        &audience,
        &capabilities,
        &basalt::ucan::ProofReferences::empty(),
        basalt::ucan::TokenTimeBounds::new(100, 200).expect("time bounds"),
    )
    .expect("signed token");
    let resolver = basalt::ucan::KeyResolutionContext::new(vec![
        issuer.verification_key().expect("issuer key"),
        holder.verification_key().expect("holder key"),
    ]);
    (
        Keys {
            issuer: issuer.issuer().expect("issuer DID").as_str().to_string(),
            holder,
            audience,
            resolver,
        },
        token,
    )
}

fn contract(keys: &Keys, token: &basalt::ucan::CompactToken, policy_json: &str) -> (Contract, super::super::Scope) {
    let limits = super::super::Limits {
        max_calls: 2,
        max_output_bytes: 4_096,
    };
    let policy = super::super::authority::policy_ref(policy_json).expect("policy ref");
    let authority = super::super::authority::credential_ref(token.as_str(), &keys.issuer, keys.audience.as_str())
        .expect("authority ref");
    let resources = super::super::limits_ref(limits).expect("limits ref");
    let profile = crate::effects::handler_profile_value(&crate::effects::HandlerProfileInput {
        profile: "production".to_string(),
        handler_binding_refs: vec![reference("handler")],
        policy_ref: policy.clone(),
        capability_context_ref: authority.clone(),
        resource_refs: vec![resources.clone()],
        evidence_refs: Vec::new(),
    })
    .expect("profile");
    let scope = super::super::Scope {
        run_ref: reference("run"),
        handler_profile_ref: crate::preserves_rail::canonical_hash(&profile).expect("profile ref"),
        turn_ref: reference("turn"),
        boundary_ref: reference("boundary"),
    };
    let capabilities = vec![reference("capability")];
    let manifest = crate::effects::effect_manifest_value(&crate::effects::EffectManifestInput {
        artifact_kind: "wasm".to_string(),
        artifact_ref: reference("artifact"),
        executor_kind: "wasm".to_string(),
        declared_effects: vec![crate::effects::DeclaredEffect {
            effect_id: "blob.get".to_string(),
            operation: "get".to_string(),
            input_schema_ref: reference("input-schema"),
            output_schema_ref: reference("output-schema"),
            resource_class: "hostcall".to_string(),
            capability_refs: capabilities.clone(),
            evidence_refs: Vec::new(),
        }],
        policy_refs: vec![policy.clone()],
        evidence_refs: Vec::new(),
    })
    .expect("manifest");
    let grant = crate::wasm_component::ImportGrant {
        import: "effect-request".to_string(),
        capability: "hostcall:blob-get".to_string(),
        policy_ref: policy,
        authority_ref: authority,
        resource_ref: resources,
        recorded_effect_ref: super::super::recording_ref(&scope, 3, 5).expect("recording ref"),
    };
    (
        Contract {
            manifest,
            profile,
            grant,
            limits,
            capabilities,
        },
        scope,
    )
}

pub(super) fn reference(label: &str) -> String {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("test-ref", vec![
        crate::preserves_rail::string(label),
    ]))
    .expect("ref")
}
