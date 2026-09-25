include!("parts/mod/p000/body.rs");
include!("parts/mod/p001/body.rs");
include!("gc.rs");

#[cfg(test)]
mod tests {
    include!("parts/mod/tests/m000/p000/body.rs");
    include!("parts/mod/tests/m000/p001/body.rs");
}
