fn main() {
    // Re-run build if workflow changed.
    println!("cargo:rerun-if-changed=workflows");
    // Builds the project located in `workflows/module1`
    // into a Flawless WebAssembly artifact.
    flawless_utils::build_release("module1");
}
