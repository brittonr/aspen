//! Records the marble object-store pilot measurement and keep-or-replace
//! decision for one executed run.
//!
//! Usage: `marble-store-measure [batches] [objects-per-batch] [object-bytes]`
//!
//! Prints the canonical measurement and decision Preserves values to standard
//! output. Timings are recorded diagnostics from this single run; they are
//! not deterministic playback artifacts and prove nothing beyond themselves.
//! See `docs/marble-object-store-pilot.md` for the claim boundary.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let batches: u32 = std::env::args().nth(1).map_or(Ok(64), parse_bounded)?.clamp(1, 4_096);
    let objects_per_batch: u32 = std::env::args().nth(2).map_or(Ok(16), parse_bounded)?.clamp(1, 4_096);
    let object_bytes: u32 = std::env::args().nth(3).map_or(Ok(4 * 1024), parse_bounded)?.clamp(64, 1_048_576);
    let root = std::env::temp_dir().join(format!("molten-marble-store-measure-{}", std::process::id()));
    let heap_path = root.join("heap");
    let chunk_root = root.join("chunks");
    let input = molten::marble_store::MeasureInput {
        heap_path: &heap_path,
        chunk_root: &chunk_root,
        batches,
        objects_per_batch,
        object_bytes,
    };
    let report = molten::marble_store::compare_paths(&input)?;
    let decision = molten::marble_store::keep_or_replace(&report);
    println!("{}", molten::preserves_rail::to_text(&molten::marble_store::measurement_value(&report))?);
    println!("{}", molten::preserves_rail::to_text(&molten::marble_store::decision_value(&report, &decision))?);
    Ok(())
}

fn parse_bounded(value: String) -> Result<u32, Box<dyn std::error::Error>> {
    value
        .parse::<u32>()
        .map_err(|error| format!("expected a bounded u32 argument, got {value}: {error}").into())
}
