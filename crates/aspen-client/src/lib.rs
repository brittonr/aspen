//! Pure client library for Aspen distributed systems.
//!
//! This crate provides a lightweight client for connecting to Aspen clusters
//! without pulling in the heavy raft/cluster dependencies. It is designed for
//! external consumers who only need to interact with Aspen as a client.
//!
//! # Key Components
//!
//! - [`AspenClient`]: Main client for sending RPC requests to cluster nodes
//! - [`ClientRpcRequest`]: All available RPC request types
//! - [`ClientRpcResponse`]: All available RPC response types
//! - [`AspenClusterTicket`]: Ticket for bootstrapping connections
//!
//! # Example
//!
//! ```rust,ignore
//! use aspen_client::{AspenClient, ClientRpcRequest};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Connect to a cluster using a ticket
//!     let client = AspenClient::connect(
//!         "aspen...",  // ticket string
//!         Duration::from_secs(5),
//!         None,  // no auth token
//!     ).await?;
//!
//!     // Send a request
//!     let response = client.send(ClientRpcRequest::Ping).await?;
//!     println!("Response: {:?}", response);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Feature Flags
//!
//! - `pijul`: Include Pijul version control RPC variants
//! - `forge`: Include Forge decentralized Git RPC variants

mod client;
mod constants;
mod rpc;
mod ticket;

// Re-export all public types at crate root
pub use client::AspenClient;
pub use client::AuthToken;
pub use client::AuthenticatedRequest;
pub use constants::CLIENT_ALPN;
pub use constants::MAX_CLIENT_MESSAGE_SIZE;
pub use rpc::*;
pub use ticket::AspenClusterTicket;
pub use ticket::SignedAspenClusterTicket;

// Re-export iroh types that clients need
pub use iroh::EndpointAddr;
pub use iroh::EndpointId;
