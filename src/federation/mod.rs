pub mod admission;
pub mod envelope;
pub mod hints;
pub mod lifecycle;
pub mod liveproof;
pub mod simulation;
pub mod simulationworld;
pub mod transfer;

#[cfg(test)]
mod fixture {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/federation/parts/mod/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/federation/parts/mod/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/federation/parts/mod/p002/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/federation/parts/mod/p003/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/federation/parts/mod/p004/body.rs"));
}
