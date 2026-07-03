include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/peer/parts/bootstrap/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/peer/parts/bootstrap/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/peer/parts/bootstrap/p002/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/peer/parts/bootstrap/p003/body.rs"));

pub mod handoff;
pub mod promotion;
pub mod session;
pub mod subscriber;

pub use handoff::*;
pub use promotion::*;
pub use session::*;
pub use subscriber::*;
