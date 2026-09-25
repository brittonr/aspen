
fn parse_tasks(value: &Value<IoValue>) -> Result<Vec<UpgradeTask>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, "tasks", 1)?;
    let items = required_sequence(&fields[0], "upgrade tasks")?;
    let mut tasks = Vec::with_capacity(items.len());
    for item in items.iter() {
        tasks.push(parse_task(&value_to_iovalue(item))?);
    }
    Ok(tasks)
}
