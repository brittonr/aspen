use ed25519_dalek::SigningKey;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_snapshot::*;

use super::super::*;
use crate::node_state::DirectoryRoot;
use crate::node_state::RelativePath;
use crate::runtime::RuntimeState;
use crate::runtime::RuntimeStep;
use crate::runtime::RuntimeValue;

fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

struct Fixture {
    _workspace: crate::test_support::ProcessWorkspace,
    root: DirectoryRoot,
    signer: SigningKey,
    descriptor: SnapshotDescriptor,
    destination_ref: String,
    current: LocalCurrentAdmission,
    image: LogicalRuntimeImage,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let workspace = crate::test_support::process_workspace(label).expect("workspace");
        let root = DirectoryRoot::open(&workspace).expect("root");
        root.create_layout().expect("layout");
        for name in ["config.preserves", "identity.preserves"] {
            let bytes = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(
                "explicit-experimental-snapshot-node-v1",
                vec![crate::preserves_rail::string(name)],
            ))
            .expect("node binding");
            root.write(&RelativePath::parse(name).expect("path"), &bytes).expect("node identity");
        }
        let signer = SigningKey::from_bytes(&[7; 32]); // explicitly experimental fixture authority
        let adapter = LocalLogicalSnapshot::from_root(root.clone(), signer.verifying_key()).expect("adapter");
        let mut runtime = RuntimeState::new(7);
        let value = RuntimeValue::string("ready").expect("value");
        let mut history = Vec::with_capacity(4096);
        for step in [
            RuntimeStep::Assert {
                actor: "producer".into(),
                value: value.clone(),
            },
            RuntimeStep::Observe {
                actor: "consumer".into(),
                pattern: value.clone(),
            },
            RuntimeStep::Send {
                from: "producer".into(),
                to: "consumer".into(),
                body: value.clone(),
            },
            RuntimeStep::Clock {
                actor: "producer".into(),
            },
            RuntimeStep::Random {
                actor: "producer".into(),
                upper: 100,
            },
        ] {
            let events = runtime.apply_step(&step);
            assert!(events.len() <= 4095 - history.len(), "fixture history leaves room for unresolved effect");
            history.extend(events.iter().map(|event| event.to_value()));
        }
        let (events, _) = runtime.apply_facet_create("producer", "facet:root", None, None).expect("facet");
        history.extend(events.iter().map(|event| event.to_value()));
        let (events, _) = runtime.apply_facet_assert("producer", "facet:root", &value).expect("facet assertion");
        history.extend(events.iter().map(|event| event.to_value()));
        let unresolved = runtime
            .begin_effect_for_step(&RuntimeStep::Clock {
                actor: "producer".into(),
            })
            .expect("checked effect sequence")
            .expect("unresolved request");
        history.push(unresolved.to_value());
        let image = LogicalRuntimeImage {
            snapshot: runtime.snapshot(),
            history,
            unresolved_effects: vec![unresolved],
            policy_ref: reference("policy"),
        };
        let descriptor = adapter
            .prepare_bundle(&image, WorldCommitRef::new(reference("commit")).expect("commit"))
            .expect("bundle");
        let current = LocalCurrentAdmission {
            policy_ref: image.policy_ref.clone(),
            capability_ref: reference("capability"),
            revocation_ref: reference("revocation-1"),
            resource_ref: reference("fixed-bounded-runtime-resources"),
            adapter_ref: reference("local-runtime-spine-redb"),
            generation: 1,
            allowed: true,
            expires_unix_seconds: u64::MAX,
        };
        let fixture = Self {
            _workspace: workspace,
            root,
            signer,
            descriptor,
            destination_ref: adapter.destination_ref().to_string(),
            current,
            image,
        };
        fixture.admit(&fixture.current, &fixture.signer);
        fixture
    }

    fn admit(&self, current: &LocalCurrentAdmission, signer: &SigningKey) {
        let descriptor_ref = canonical_snapshot_descriptor(&self.descriptor).expect("descriptor").artifact_ref;
        let grant = sign_local_admission(LocalAdmissionInput {
            signer,
            descriptor_ref: &descriptor_ref,
            profile_ref: self.descriptor.profile_ref.as_str(),
            cohort_ref: self.descriptor.cohort.cohort_ref.as_str(),
            destination_ref: &self.destination_ref,
            current: current.clone(),
        })
        .expect("signed current grant");
        self.write("storage/world-snapshot/current.json", &serde_json::to_vec(current).expect("current"));
        self.write("storage/world-snapshot/admission.json", &grant);
    }

    fn write(&self, path: &str, bytes: &[u8]) {
        self.root.write(&RelativePath::parse(path).expect("path"), bytes).expect("write");
    }
    fn adapter(&self) -> LocalLogicalSnapshot {
        LocalLogicalSnapshot::from_root(self.root.clone(), self.signer.verifying_key()).expect("adapter")
    }
    fn restore(&self) -> crate::error::Result<LocalLogicalRestoreOutcome> {
        restore_local_logical_snapshot(self.adapter(), &self.descriptor, &self.descriptor.cohort)
    }
}