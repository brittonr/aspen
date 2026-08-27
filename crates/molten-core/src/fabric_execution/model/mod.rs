mod profile;
mod request;

pub use profile::*;
pub use request::*;

// r[impl molten.fabric_execution.port_contract]
pub const EXECUTION_PROFILE_SCHEMA: &str = "molten.fabric.execution-profile.v1";
pub const EXECUTION_REQUEST_SCHEMA: &str = "molten.fabric.execution-request.v1";
pub const EXECUTION_OUTCOME_SCHEMA: &str = "molten.fabric.execution-outcome.v1";
pub const EXECUTION_RECEIPT_SCHEMA: &str = "molten.fabric.execution-receipt.v1";
pub const EXECUTION_PORT_ID: &str = "molten.fabric.execution.bounded-process";
pub const EXECUTION_PORT_VERSION: &str = "v1";
pub const EXECUTION_INPUT_SCHEMA: &str = "molten.fabric.execution-request.v1";
pub const EXECUTION_OUTPUT_SCHEMA: &str = "molten.fabric.execution-receipt.v1";

pub const BOUNDED_EXEC_REPOSITORY: &str = "https://git.onix.computer/z2CpqLFpdP36fZXYUK5ZNWxMibpCo.git";
pub const BOUNDED_EXEC_REVISION: &str = "29dac88ecded94457572db3fdfaaaab95fa91525";
pub const BOUNDED_EXEC_LICENSE: &str = "AGPL-3.0-or-later";
pub const BOUNDED_EXEC_PACKAGE: &str = "bounded-exec";
