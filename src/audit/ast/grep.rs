include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/audit/parts/ast_grep/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/audit/parts/ast_grep/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/audit/parts/ast_grep/p003/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/audit/parts/ast_grep/p004/body.rs"));

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/audit/parts/ast_grep/tests/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/audit/parts/ast_grep/tests/p001/body.rs"));
}
