#[test]
fn molten_maps_the_shared_planning_fixture_without_effects() -> Result<(), Box<dyn std::error::Error>> {
    let report = schema_migration_conformance::shared_fixture_report()?;
    assert_eq!(report.decision(), "selected");
    assert_eq!(report.step_count(), 1);
    assert!(!report.has_conditional_recovery());
    assert!(report.plan_id().is_some());
    Ok(())
}

#[test]
fn molten_rejects_unknown_planning_fixtures() {
    assert!(schema_migration_conformance::run_scenario("molten-unknown").is_err());
}

#[test]
fn cargo_and_nix_source_declarations_match() {
    let source = include_str!("../source.ncl");
    let flake = include_str!("../../../flake.nix");
    let revision = "3f7b4315c8e1d07726446f1e53ba45bc091c5275";
    assert_eq!(source.matches(revision).count(), 1);
    assert_eq!(source.matches("= Revision").count(), 3);
    assert!(!source.contains("../schema-migration-core"));
    assert!(flake.contains(revision));
    assert!(flake.contains("schema-migration-core-src"));
}
