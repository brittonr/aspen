### **Project Aspen: A New Core**

The goal is to build `aspen`, a new core crate for the Blixard ecosystem. This crate will be the foundation for orchestration, providing the core logic and distributed primitives (`iroh`, `hiqlite`) needed for tasks like managing Flawless workers or a distributed OpenTofu backend.

---

### **Phase 1: Foundation & Scaffolding**

This phase focuses on setting up the new crate and its foundational structure, following Tiger Style's emphasis on logical organization and minimizing dependencies.

1.  **Reset to a Clean State:** The `blixard` repository was reset to a clean state.
2.  **Create the `aspen` Crate:** A new library crate named `aspen` was created within the `blixard` workspace.
3.  **Integrate into Workspace:** `aspen` was added as a dependency to the root `Cargo.toml`.
4.  **Define Core Dependencies:** `aspen/Cargo.toml` was configured with essential dependencies (`iroh`, `hiqlite`, `tokio`, `serde`, `async-trait`, `tracing`, `uuid`, `chrono`, `dashmap`, `thiserror`, `bytes`).
5.  **Establish Module Structure:** A basic module structure (`orchestration`, `distributed`, `traits`, `types`) was established within `aspen/src`, and declared in `aspen/src/lib.rs`.

---

### **Phase 2: Defining Core Abstractions**

With the foundation in place, this phase will define the core nouns and verbs of our system, adhering to Tiger Style's principles of clear naming and simple interfaces.

1.  **Define Core Types:** Migrate the core data structures (e.g., `Job`, `Worker`, `ExecutionHandle`, `ExecutionStatus`, `WorkResult`, `JobAssignment`, `VmConfig`, `VmInstance`, `VmState`, `JobRequirements`, `JobResult`, `VmAssignment`, `VmStats`, `HealthStatus`, `ResourceRequirements`, `ResourceInfo`, `BackendHealth`, `ExecutionConfig`, `ExecutionMetadata`) from the old `blixard-core` and `src` directories into `aspen/src/types/`. Ensure explicitly sized types are used where appropriate.
2.  **Define Core Traits:** Migrate the essential service traits (e.g., `ExecutionBackend`, `StateRepository`, `WorkRepository`, `WorkerRepository`, `VmManagement`) into `aspen/src/traits/`.
3.  **Update Module Declarations:** Update `aspen/src/types/mod.rs` and `aspen/src/traits/mod.rs` to properly declare all the newly added types and traits.
4.  **Verify:** Run `cargo check -p aspen` to ensure the crate compiles.

---

### **Phase 3: Implementing Core Services**

Once the abstractions are defined, I will begin implementing the core logic and integrating `aspen` into the broader application.

1.  **Implement Distributed Primitives:** I will build the `iroh` and `hiqlite` services within `aspen/src/distributed/`, providing the foundation for distributed state and communication.
2.  **Build Orchestration Services:** I will implement the high-level orchestration logic that consumes the traits and types defined in Phase 2.
3.  **Integrate `aspen`:** I will replace `blixard-core` with `aspen` in the other crates (`blixard-adapter-*`, `blixard-control-plane`, etc.) and begin removing the old, redundant code from the root `src` directory.
