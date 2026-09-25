
#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/delivery/parts/idempotency/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/delivery/parts/idempotency/tests/m000/p001/body.rs"));
}
