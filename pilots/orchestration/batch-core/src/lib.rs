// SPDX-License-Identifier: AGPL-3.0-or-later
#![no_std]
#![forbid(unsafe_code)]
//! Pure batch and worker state machines. Every observation is explicit input.
extern crate alloc;
mod controller;
mod model;
mod worker;
pub use controller::*;
pub use model::*;
pub use worker::*;

mod policy;
pub use policy::*;
