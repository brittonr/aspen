//! Forge web frontend served over HTTP/3 via iroh-h3.
//! No axum — raw h3 request handling.

pub mod routes;
pub mod server;
pub mod state;
pub mod templates;
