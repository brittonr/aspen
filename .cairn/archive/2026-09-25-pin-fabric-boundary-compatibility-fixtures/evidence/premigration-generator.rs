// Throwaway generator for the fabric boundary compatibility fixtures.
// Runs at the pre-migration commit 3de348149^ with the shared explicit inputs,
// placed at tests/fabric_boundary_fixture_generator.rs next to a copy of tests/fabricboundarycompat/.
#[path = "fabricboundarycompat/cases.rs"]
mod cases;
#[path = "fabricboundarycompat/inputs.rs"]
mod inputs;
#[path = "fabricboundarycompat/ports.rs"]
mod ports;

use cases::OrFail;

#[test]
fn generate_fabric_boundary_fixtures() -> cases::TestResult<()> {
    let out = std::path::PathBuf::from(std::env::var("FABRIC_BOUNDARY_FIXTURE_OUT").or_fail("output dir")?);
    std::fs::create_dir_all(&out).or_fail("output dir")?;
    let empty_transport_state = molten::fabric_transport::TransportState::default();
    let mut manifest = String::new();
    for case in cases::canonical_projections(&empty_transport_state)? {
        let bytes = molten::preserves_rail::canonical_bytes(&case.value).or_fail(case.name)?;
        assert_eq!(molten::preserves_rail::content_ref_from_bytes(&bytes), case.value_ref, "{}", case.name);
        std::fs::write(out.join(format!("{}.preserves", case.name)), &bytes).or_fail(case.name)?;
        manifest.push_str(&format!("{}\t{}\t{}\n", case.name, case.value_ref, bytes.len()));
    }
    std::fs::write(out.join("refs.tsv"), manifest).or_fail("manifest write")
}
