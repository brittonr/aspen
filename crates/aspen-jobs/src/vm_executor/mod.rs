//! VM-based job execution with Hyperlight micro-VMs and WASM components.
//!
//! This module provides sandboxed job execution using Hyperlight micro-VMs,
//! supporting pre-built binaries, on-demand Nix builds, and WASM Component Model
//! execution via hyperlight-wasm.
//!
//! # Choosing a Worker
//!
//! | Worker | Use When | Isolation | Startup | Guest Languages |
//! |---|---|---|---|---|
//! | [`HyperlightWorker`] | Running native ELF binaries as short-lived compute tasks | KVM micro-VM | ~1ms | Rust, C, any static ELF |
//! | [`WasmComponentWorker`] | Running portable, sandboxed logic with host API access (KV, blobs) | KVM + WASM sandbox | ~1ms | Rust/C compiled to WASM Component Model |
//!
//! The CI-specific [`CloudHypervisorWorker`](aspen_ci_executor_vm::CloudHypervisorWorker)
//! lives in `aspen-ci-executor-vm` and serves a different purpose:
//!
//! | Worker | Use When | Isolation | Startup | Guest Environment |
//! |---|---|---|---|---|
//! | `CloudHypervisorWorker` | Running full CI/CD jobs that need a Linux environment, filesystem, Nix store, and network | Full Cloud Hypervisor VM | ~seconds (warm pool) | Full Linux guest with virtiofs mounts |
//!
//! ## Decision Guide
//!
//! - **"I have a compiled Rust/C binary that does one thing"** -- [`HyperlightWorker`]. Fastest
//!   startup, minimal overhead. Binary runs directly in a KVM micro-VM with only `hl_println` and
//!   `hl_get_time` host functions. Supports sourcing binaries from blob store, Nix flakes, or
//!   inline Nix expressions.
//!
//! - **"I need the guest to call back into Aspen (KV, blobs, crypto)"** -- [`WasmComponentWorker`].
//!   Same KVM-backed isolation as Hyperlight, but the guest runs as a WASM component with 20 typed
//!   host functions (KV CRUD, blob store, HLC, crypto, etc.). Best for plugins and user-defined
//!   extensions that need to interact with cluster state.
//!
//! - **"I have a Python or JavaScript script"** -- `CloudHypervisorWorker`. Run scripts inside a
//!   full Linux VM with a POSIX-compatible interface. Upload the script and run it. Good for
//!   user-submitted scripts, webhooks, and lightweight automation.
//!
//! - **"I need to run `cargo build`, `nix build`, or a multi-step CI pipeline"** --
//!   `CloudHypervisorWorker`. Full Linux VM with `/nix/store` mounted via virtiofs, network access,
//!   and a persistent workspace directory. The warm pool amortizes boot time across jobs. Used
//!   exclusively by the CI system.
//!
//! ## Storage Backend
//!
//! All workers use Aspen's distributed storage (KV + iroh-blobs) as the
//! backing store. No worker relies on host-local state for persistent data.
//!
//! - **KV store** (Raft consensus): Metadata, small files (up to 1MB), directory structure,
//!   configuration, job state. Linearizable reads/writes.
//! - **iroh-blobs**: Large content-addressed data -- binaries, WASM components, NARs, build
//!   artifacts. P2P transfer across the cluster.
//!
//! For workers that expose a filesystem to the guest (`CloudHypervisorWorker`),
//! the mapping is **transparent**: the guest
//! sees a standard POSIX filesystem and has no knowledge of Aspen. On the
//! host side, `aspen-fuse` (`AspenFs`) serves the VirtioFS protocol,
//! routing file operations to KV for metadata and small files and to
//! iroh-blobs for large content. See `crates/aspen-fuse/src/virtiofs.rs`.
//!
//! Per-worker storage model:
//!
//! - **[`HyperlightWorker`]**: Binaries sourced from iroh-blobs (or built via Nix and cached back
//!   to blobs). No guest filesystem -- input/output is passed as in-memory byte buffers through the
//!   `execute` host call.
//!
//! - **[`WasmComponentWorker`]**: Component bytes from iroh-blobs. The guest accesses both KV and
//!   blobs directly via 20 typed host functions (KV CRUD + CAS + scan, blob get/put/has, HLC,
//!   crypto). No filesystem needed -- the host API *is* the storage interface.
//!
//! - **`CloudHypervisorWorker`**: VMs are full Aspen cluster members (`aspen-node --worker-only`).
//!   The guest mounts two virtiofs shares (`/nix/store` read-only, `/workspace` read-write) that
//!   appear as normal filesystems inside the VM. The host side should serve these via
//!   `AspenVirtioFsHandler` instead of plain `virtiofsd`:
//!   - **Nix store**: Backed by SNIX (store path metadata in KV, NAR content in blobs).
//!   - **Workspace**: Backed by `AspenFs` (paths map to KV keys, large artifacts stored as blobs).
//!
//!   The workspace share uses `AspenVirtioFsHandler` (backed by `AspenFs`) for in-process
//!   VirtioFS serving. The Nix store share still uses plain `virtiofsd` pending SNIX
//!   integration.

#[cfg(feature = "plugins-vm")]
mod hyperlight;
#[cfg(any(feature = "plugins-vm", feature = "plugins-wasm"))]
mod types;
#[cfg(feature = "plugins-wasm")]
mod wasm_component;
#[cfg(feature = "plugins-wasm")]
mod wasm_host;

#[cfg(feature = "plugins-vm")]
pub use hyperlight::HyperlightWorker;
#[cfg(any(feature = "plugins-vm", feature = "plugins-wasm"))]
pub use types::JobPayload;
#[cfg(any(feature = "plugins-vm", feature = "plugins-wasm"))]
pub use types::NixBuildOutput;
#[cfg(feature = "plugins-wasm")]
pub use wasm_component::WasmComponentWorker;
