pub fn entry(value: u64) -> Result<(), String> { validate_bound(value) }
fn validate_bound(value: u64) -> Result<(), String> {
    if value > 0 {
        return Err(format!("bound {value}"));
    }
    Ok(())
}
