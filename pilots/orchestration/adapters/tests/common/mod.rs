// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(dead_code)]
use std::collections::BTreeMap;
use std::sync::Arc;

use molten_batch_application::*;
use molten_batch_core::*;
use molten_orchestrator_adapters::*;
pub fn id(b: u8) -> Id {
    Id([b; 32])
}
pub fn resources() -> Resources {
    Resources {
        cpu_millis: 1000,
        memory_bytes: 16 * 1024 * 1024,
        slots: 1,
    }
}
pub fn task(component: Id) -> TaskSpec {
    TaskSpec {
        id: "task".into(),
        component,
        inputs: vec![],
        dependencies: vec![],
        resources: resources(),
        required_labels: BTreeMap::new(),
        fuel: 1_000_000,
        timeout_ms: 2000,
        max_output_bytes: 1024,
        max_attempts: 2,
        retry_delay_ticks: 1,
    }
}
pub fn request(component: Id) -> ExecutionRequest {
    let spec = JobSpec {
        name: "test".into(),
        tasks: vec![task(component)],
    };
    let identity = CanonicalIdentity;
    let spec_ref = identity.spec(&spec).unwrap();
    let run = identity.run(id(1), "test").unwrap();
    let p = Placement {
        run,
        task: "task".into(),
        node: id(4),
        number: 1,
        inputs: vec![],
    };
    ExecutionRequest {
        fence: Fence {
            cluster: id(1),
            generation: 1,
            run,
            task: p.task.clone(),
            assignment: identity.assignment(id(1), 1, &p, spec_ref).unwrap(),
            spec_ref,
            node: id(4),
            epoch: 1,
        },
        policy_ref: id(2),
        component,
        inputs: vec![],
        resources: resources(),
        fuel: 1_000_000,
        timeout_ms: 2000,
        max_output_bytes: 1024,
    }
}
pub struct Setup {
    pub temporary: tempfile::TempDir,
    pub disk: Arc<Disk>,
    pub store: Arc<RedbControllerStore>,
    pub content: Arc<RedbContent>,
}
pub fn setup() -> Setup {
    let temporary = tempfile::tempdir().unwrap();
    let root = CapabilityRoot::open(temporary.path()).unwrap();
    let disk = Disk::open(&root).unwrap();
    let store = RedbControllerStore::open(disk.clone(), State::empty(id(1), id(2), 1, 3).unwrap()).unwrap();
    let content = RedbContent::new(disk.clone());
    Setup {
        temporary,
        disk,
        store,
        content,
    }
}
pub fn fixture() -> Content {
    content(ContentKind::Component, include_bytes!("../../../examples/constant.wat").to_vec()).unwrap()
}
