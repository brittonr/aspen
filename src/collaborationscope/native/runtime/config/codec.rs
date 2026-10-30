pub(super) const MAX_BYTES: usize = 1024 * 1024;
const MAX_ITEMS: usize = 4096;
const MAX_DEPTH: usize = 16;

pub(super) fn invalid(error: impl std::fmt::Display) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(format!("collaboration native runtime: {error}"))
}

/// Every encoded node occupies at least one byte, so a frame within `MAX_BYTES` never reaches this
/// bound.
const MAX_NODES: usize = MAX_BYTES;

/// Explicit traversal work: visit a JSON node, wrap the last built value as an entry, or close a
/// container.
enum Task<'a> {
    Visit(&'a serde_json::Value, usize),
    Entry(&'a str),
    Close(&'static str, usize),
}

/// Builds the canonical record bottom-up from an explicit work stack in depth-first source order.
pub(super) fn value(json: &serde_json::Value, depth: usize) -> crate::error::Result<preserves::IOValue> {
    let mut stacks = Stacks {
        pending: vec![Task::Visit(json, depth)],
        built: Vec::new(),
    };
    while let Some(task) = stacks.pending.pop() {
        if stacks.built.len() >= MAX_NODES {
            return Err(invalid("configuration node bound"));
        }
        let built = &mut stacks.built;
        match task {
            Task::Visit(json, depth) => visit(json, depth, &mut stacks)?,
            Task::Entry(key) => {
                let item = built.pop().ok_or_else(|| invalid("configuration entry value missing"))?;
                built.push(crate::preserves_rail::record("entry", vec![crate::preserves_rail::string(key), item]));
            }
            Task::Close(label, count) => {
                let start =
                    built.len().checked_sub(count).ok_or_else(|| invalid("configuration container fields missing"))?;
                let fields = built.split_off(start);
                built.push(crate::preserves_rail::record(label, fields));
            }
        }
    }
    let root = stacks.built.pop().ok_or_else(|| invalid("configuration value missing"))?;
    if !stacks.built.is_empty() {
        return Err(invalid("configuration traversal left unconsumed values"));
    }
    Ok(root)
}

/// Explicit depth-first work and result stacks; growth is bounded by `MAX_NODES` in `value`.
struct Stacks<'a> {
    pending: Vec<Task<'a>>,
    built: Vec<preserves::IOValue>,
}

fn visit<'a>(json: &'a serde_json::Value, depth: usize, stacks: &mut Stacks<'a>) -> crate::error::Result<()> {
    if depth > MAX_DEPTH {
        return Err(invalid("configuration nesting bound"));
    }
    let Stacks { pending, built } = stacks;
    match json {
        serde_json::Value::Null => built.push(crate::preserves_rail::record("null", Vec::new())),
        serde_json::Value::Bool(flag) => {
            built.push(crate::preserves_rail::record(if *flag { "true" } else { "false" }, Vec::new()));
        }
        serde_json::Value::Number(number) => {
            let number = number.as_u64().ok_or_else(|| invalid("unsigned integers required"))?;
            built.push(crate::preserves_rail::record("uint", vec![crate::preserves_rail::u64_value(number)]));
        }
        serde_json::Value::String(text) => {
            if text.len() > 65536 {
                return Err(invalid("configuration text bound"));
            }
            built.push(crate::preserves_rail::record("text", vec![crate::preserves_rail::string(text)]));
        }
        serde_json::Value::Array(items) => {
            if items.len() > MAX_ITEMS {
                return Err(invalid("configuration array bound"));
            }
            pending.push(Task::Close("array", items.len()));
            pending.extend(items.iter().rev().map(|item| Task::Visit(item, depth + 1)));
        }
        serde_json::Value::Object(entries) => {
            if entries.len() > 128 {
                return Err(invalid("configuration object bound"));
            }
            pending.push(Task::Close("object", entries.len()));
            pending.extend(
                entries
                    .iter()
                    .rev()
                    .flat_map(|(key, item)| [Task::Entry(key.as_str()), Task::Visit(item, depth + 1)]),
            );
        }
    }
    Ok(())
}

pub(super) fn parse(value: &preserves::IOValue, depth: usize) -> crate::error::Result<serde_json::Value> {
    if depth > MAX_DEPTH {
        return Err(invalid("configuration nesting bound"));
    }
    for (label, result) in [
        ("null", serde_json::Value::Null),
        ("true", true.into()),
        ("false", false.into()),
    ] {
        if value.collect_simple_record(label, Some(0)).is_some() {
            return Ok(result);
        }
    }
    if let Ok(fields) = super::super::super::decode::fields(value, "text", 1) {
        let text = super::super::super::decode::string(&fields[0])?;
        if text.len() > 65536 {
            return Err(invalid("configuration text bound"));
        }
        return Ok(text.into());
    }
    if let Ok(fields) = super::super::super::decode::fields(value, "uint", 1) {
        return Ok(super::super::super::decode::integer(&fields[0])?.into());
    }
    if let Some(items) = value.collect_simple_record("array", None) {
        if items.fields_iter().count() > MAX_ITEMS {
            return Err(invalid("configuration array bound"));
        }
        return Ok(serde_json::Value::Array(
            items
                .fields_iter()
                .map(|item| parse(&crate::preserves_rail::value_to_iovalue(item), depth + 1))
                .collect::<crate::error::Result<_>>()?,
        ));
    }
    object(value, depth)
}

fn object(value: &preserves::IOValue, depth: usize) -> crate::error::Result<serde_json::Value> {
    let entries = value.collect_simple_record("object", None).ok_or_else(|| invalid("unknown configuration record"))?;
    if entries.fields_iter().count() > 128 {
        return Err(invalid("configuration object bound"));
    }
    let mut result = serde_json::Map::new();
    for entry in entries.fields_iter() {
        let fields = super::super::super::decode::fields(&crate::preserves_rail::value_to_iovalue(entry), "entry", 2)?;
        let key = super::super::super::decode::string(&fields[0])?;
        if result.insert(key, parse(&fields[1], depth + 1)?).is_some() {
            return Err(invalid("duplicate configuration key"));
        }
    }
    Ok(serde_json::Value::Object(result))
}
