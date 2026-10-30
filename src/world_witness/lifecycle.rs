//! Durable staging, publication, repeated-read and shell-port lifecycle of one witnessed head
//! transition.

pub(super) mod adapters;
pub(super) mod current;
pub(super) mod durable;
pub(super) mod effects;
pub(super) mod local;
pub(super) mod reserve;
pub(super) mod shell;
