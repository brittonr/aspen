fn main() {
    if let Err(error) = molten::addressable_actor::counter::run_stdio() {
        eprintln!("addressable counter callback failed: {error}");
        std::process::exit(1);
    }
}
