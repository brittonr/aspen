include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/nixos/parts/vm/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/nixos/parts/vm/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/nixos/parts/vm/p002/body.rs"));

#[path = "vm/validation.rs"]
mod validation;
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/nixos/parts/vm/p003/body.rs"));

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
