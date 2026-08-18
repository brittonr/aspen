fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = schema_migration_conformance::shared_fixture_report()?;
    let plan_id = report.plan_id().ok_or_else(|| std::io::Error::other("shared fixture did not produce a plan"))?;
    println!("{plan_id}");
    Ok(())
}
