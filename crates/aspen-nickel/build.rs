fn main() {
    // Track include_str!'d non-Rust files so cargo (and buildRustCrate) rebuilds
    // when their content changes.
    println!("cargo:rerun-if-changed=src/schema/node_config.ncl");
}
