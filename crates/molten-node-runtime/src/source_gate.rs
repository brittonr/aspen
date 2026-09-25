// Evaluate the same strict source gate implementation as root, without
// compiling root's independent Octet unit-test fixtures into this package.
#[path = "../../../src/octet/startup_snapshot.rs"]
pub mod startup_snapshot;

include!("../../../src/octet/parts/gate/p000/body.rs");
include!("../../../src/octet/parts/gate/p001/body.rs");
include!("../../../src/octet/parts/gate/p002/body.rs");
include!("../../../src/octet/parts/gate/p003/body.rs");
include!("../../../src/octet/parts/gate/p004/body.rs");
include!("../../../src/octet/parts/gate/p005/body.rs");
include!("../../../src/octet/parts/gate/p006/body.rs");
include!("../../../src/octet/parts/gate/p007/body.rs");
include!("../../../src/octet/parts/gate/p008/body.rs");
include!("../../../src/octet/parts/gate/p009/body.rs");
include!("../../../src/octet/parts/gate/p010/body.rs");
