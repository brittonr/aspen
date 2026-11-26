//! Kernel tests driven directly through the transition function, without Tokio, sockets, or
//! storage: elections, replication, read index, snapshots, bounds, liveness, and rejoin.

mod bounds;
mod liveness;
mod properties;
mod rejoin;
mod steps;
pub(crate) mod support;

use support::*;

use crate::model::*;
