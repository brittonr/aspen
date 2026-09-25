include!("parts/cache/p000/body.rs");
include!("parts/cache/p001/body.rs");
include!("parts/cache/p003/body.rs");
include!("parts/cache/p004/body.rs");
include!("parts/cache/p005/body.rs");
include!("parts/cache/p006/body.rs");
include!("parts/cache/p007/body.rs");
include!("parts/cache/p008/body.rs");
include!("invalidation.rs");

#[cfg(test)]
mod tests {
    include!("parts/cache/tests/m000/p000/body.rs");
    include!("parts/cache/tests/m000/p001/body.rs");
    include!("parts/cache/tests/m000/p002/body.rs");
}
