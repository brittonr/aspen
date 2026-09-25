pub mod startup_snapshot;

include!("parts/gate/p000/body.rs");
include!("parts/gate/p001/body.rs");
include!("parts/gate/p002/body.rs");
include!("parts/gate/p003/body.rs");
include!("parts/gate/p004/body.rs");
include!("parts/gate/p005/body.rs");
include!("parts/gate/p006/body.rs");
include!("parts/gate/p007/body.rs");
include!("parts/gate/p008/body.rs");
include!("parts/gate/p009/body.rs");
include!("parts/gate/p010/body.rs");

#[cfg(test)]
mod tests {
    include!("parts/gate/tests/m000/p000/body.rs");
    include!("parts/gate/tests/m000/p001/body.rs");
    include!("parts/gate/tests/m000/p002/body.rs");
}
