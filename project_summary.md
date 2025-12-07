# Project Aspen: A Foundational Orchestration Layer for Blixard Ecosystem

Project Aspen is a new core Rust crate designed to be the foundational orchestration layer for the Blixard ecosystem. Its primary goal is to provide essential logic and distributed primitives for managing and coordinating distributed systems.

## Key Inspirations & Design Principles

Aspen draws inspiration from a diverse set of influential systems and concepts, aiming for a robust, efficient, and maintainable architecture:

### Distributed Systems Paradigms

* **Erlang/BEAM/Gleam:** Emphasizing robust actor-based concurrency, fault tolerance, and distributed computing principles.
* **Plan9:** Adopting its elegant approach to distributed file systems and the "everything is a file" philosophy.
* **Kubernetes (k8s):** Learning from its powerful orchestration capabilities, declarative APIs, and self-healing properties for managing workloads.
* **FoundationDB:** Striving for strong consistency, ACID properties, and a reliable distributed transaction model.
* **etcd:** Utilizing its principles for a reliable distributed key-value store, crucial for shared configuration and service discovery. Aspen implements a similar architecture in Rust, combining `openraft` for Raft consensus with `redb` as a local durable state machine, forming the foundation for a strongly consistent, cluster-wide KV store.
* **radicle.xyz:** Inspired by its decentralized code collaboration and peer-to-peer infrastructure, suggesting a focus on distributed trust and collaboration.
* **Antithesis:** For its pioneering work in deterministic testing and finding bugs in distributed systems through simulation.
* **Nix:** For its declarative package management, reproducible builds, and atomic upgrades, ensuring consistent development and deployment environments.

### Core Design Philosophy ("Tiger Style")

* **Safety:** Building reliable, trustworthy software that works predictably in all situations, with a strong emphasis on error handling, assertions, and **Jepsen-proof correctness** for distributed environments. This includes using explicitly sized types, static memory allocation, and minimizing variable scope.
* **Performance:** Designing for efficiency from the outset, optimizing resource use (network, disk, memory, CPU), and ensuring predictable execution paths.
* **Developer Experience:** Fostering maintainability and collaboration through clear and consistent naming, logical code organization, simplified interfaces, and robust tooling.

## Core Technologies & Modules

Aspen leverages Rust and several key technologies to achieve its goals:

* **`iroh`:** Provides the underlying peer-to-peer networking and content-addressed communication.
* **`irpc`:** Used for inter-process communication within distributed components.
* **`redb`:** Acts as the embedded, ACID, high-performance local storage engine, powering each node’s Raft state machine and local persistence.
* **`openraft`:** Implements the Raft consensus algorithm, providing cluster-wide linearizability and fault-tolerant replication for the global KV store.
* **Global KV store (`openraft` + `redb`):** The central distributed key-value store is built as a Raft-backed state machine with `redb` as the durable backend. This layer ensures strong consistency, leases, watches, transactions, and snapshotting for the orchestration system.
* **`snafu` & `anyhow`:** For ergonomic and robust error handling across the codebase.
* **`proptest`:** Property-based testing to ensure correctness and explore edge cases.
* **`iroh-dag-sync` (from `iroh-experiments`):** Supports efficient synchronization of DAGs, potentially for content addressing, snapshots, or large distributed documents.
* **`madsim`:** A deterministic simulator for distributed systems, enabling robust testing and debugging of distributed logic.
* **`mad-turmoil`:** A deterministic network simulator for distributed systems, providing a controlled environment for testing network-related logic and fault injection.

## Project Structure (Planned)

The project is structured into the following main modules:

* **`orchestration`:** Handles high-level orchestration logic and interacts with the global KV store to drive scheduling, reconciliation, and supervision.
* **`distributed`:** Implements distributed primitives using `iroh` and other technologies; integrates Raft-based state machine replication and snapshot management.
* **`kv` (new module):** Encapsulates the Raft-backed global KV store with `redb` as the storage engine, including transaction, lease, watch, and snapshot logic.
* **`traits`:** Defines core service traits for various components.
* **`types`:** Contains the core data structures and types used throughout the system.

## Overall Goal

Project Aspen aims to build a highly reliable, performant, and developer-friendly foundational layer for distributed orchestration within the Blixard ecosystem. By combining a Raft-based global KV store (`openraft` + `redb`), peer-to-peer networking, DAG-based content sync, and disciplined coding practices, Aspen provides robust primitives for distributed state, communication, and consensus, with a strong emphasis on correctness, fault tolerance, and maintainability.
