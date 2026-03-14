//! JSON Schema to Nickel contract converter.
//!
//! Converts `schemars::schema::RootSchema` into Nickel contract syntax
//! matching the style used in `ci_schema.ncl`. Used by snapshot tests to
//! generate `.ncl` files from `#[derive(JsonSchema)]` Rust types.

use std::collections::BTreeMap;
use std::fmt::Write;

use schemars::schema::InstanceType;
use schemars::schema::ObjectValidation;
use schemars::schema::Schema;
use schemars::schema::SchemaObject;
use schemars::schema::SingleOrVec;

/// Convert a JSON Schema object to a Nickel contract string.
///
/// Produces idiomatic Nickel using record contracts with `|` annotations,
/// `optional` for nullable/non-required fields, and `default` values.
pub fn schema_to_nickel(name: &str, schema: &schemars::schema::RootSchema) -> String {
    let mut out = String::new();
    writeln!(out, "# Auto-generated Nickel contract. Do not edit manually.").unwrap();
    writeln!(out, "# Source: #[derive(JsonSchema)] on Rust type `{name}`").unwrap();
    if let Some(desc) = &schema.schema.metadata.as_ref().and_then(|m| m.description.as_ref()) {
        writeln!(out, "#").unwrap();
        for line in desc.lines() {
            if line.is_empty() {
                writeln!(out, "#").unwrap();
            } else {
                writeln!(out, "# {line}").unwrap();
            }
        }
    }
    writeln!(out).unwrap();

    render_schema_object(&mut out, &schema.schema, &schema.definitions, 0);
    out
}

/// Convert multiple named schemas into a single `.ncl` file with `let` bindings.
pub fn schemas_to_nickel(comment: &str, schemas: &[(&str, &schemars::schema::RootSchema)]) -> String {
    let mut out = String::new();
    writeln!(out, "# {comment}").unwrap();
    writeln!(out, "# Auto-generated Nickel contracts. Do not edit manually.").unwrap();
    writeln!(out).unwrap();

    for (i, (name, schema)) in schemas.iter().enumerate() {
        if let Some(desc) = schema.schema.metadata.as_ref().and_then(|m| m.description.as_ref()) {
            for line in desc.lines() {
                if line.is_empty() {
                    writeln!(out, "#").unwrap();
                } else {
                    writeln!(out, "# {line}").unwrap();
                }
            }
        }

        write!(out, "let {name} = ").unwrap();
        render_schema_object(&mut out, &schema.schema, &schema.definitions, 0);

        if i < schemas.len() - 1 {
            writeln!(out, "in").unwrap();
            writeln!(out).unwrap();
        } else {
            // Last binding — export as record
            writeln!(out, "in").unwrap();
            writeln!(out).unwrap();
            writeln!(out, "{{").unwrap();
            for (export_name, _) in schemas {
                writeln!(out, "  {export_name},").unwrap();
            }
            writeln!(out, "}}").unwrap();
        }
    }
    out
}

fn render_schema_object(out: &mut String, schema: &SchemaObject, defs: &BTreeMap<String, Schema>, indent: usize) {
    // Handle $ref
    if let Some(ref_path) = &schema.reference {
        let type_name = ref_path.rsplit('/').next().unwrap_or(ref_path);
        write!(out, "{type_name}").unwrap();
        return;
    }

    // Handle oneOf (tagged enums)
    if let Some(subschemas) = &schema.subschemas {
        if let Some(one_of) = &subschemas.one_of {
            render_one_of(out, one_of, defs, indent);
            return;
        }
    }

    // Handle by instance type
    match schema.instance_type.as_ref() {
        Some(SingleOrVec::Single(t)) => {
            render_instance_type(out, t, schema, defs, indent);
        }
        Some(SingleOrVec::Vec(types)) => {
            // Nullable type: e.g., ["string", "null"]
            let non_null: Vec<_> = types.iter().filter(|t| **t != InstanceType::Null).collect();
            if non_null.len() == 1 {
                let nickel_type = instance_type_to_nickel(non_null[0]);
                write!(out, "{nickel_type}").unwrap();
            } else {
                write!(out, "Dyn").unwrap();
            }
        }
        None => {
            // No type specified — could be a $ref or any
            write!(out, "Dyn").unwrap();
        }
    }
}

fn render_instance_type(
    out: &mut String,
    instance_type: &InstanceType,
    schema: &SchemaObject,
    defs: &BTreeMap<String, Schema>,
    indent: usize,
) {
    match instance_type {
        InstanceType::Object => {
            render_object(out, schema.object.as_ref(), schema, defs, indent);
        }
        InstanceType::Array => {
            if let Some(arr) = &schema.array {
                if let Some(Schema::Object(item_schema)) = arr.items.as_ref().and_then(|i| match i {
                    SingleOrVec::Single(s) => Some(s.as_ref()),
                    _ => None,
                }) {
                    write!(out, "Array ").unwrap();
                    render_schema_object(out, item_schema, defs, indent);
                } else {
                    write!(out, "Array Dyn").unwrap();
                }
            } else {
                write!(out, "Array Dyn").unwrap();
            }
        }
        other => {
            write!(out, "{}", instance_type_to_nickel(other)).unwrap();
        }
    }
}

fn render_object(
    out: &mut String,
    obj: Option<&Box<ObjectValidation>>,
    _schema: &SchemaObject,
    defs: &BTreeMap<String, Schema>,
    indent: usize,
) {
    let obj = match obj {
        Some(o) => o,
        None => {
            write!(out, "{{}}").unwrap();
            return;
        }
    };

    let pad = "  ".repeat(indent);
    let inner_pad = "  ".repeat(indent + 1);

    writeln!(out, "{{").unwrap();
    for (field_name, field_schema) in &obj.properties {
        let is_required = obj.required.contains(field_name);

        if let Schema::Object(field_obj) = field_schema {
            // Description as comment
            if let Some(desc) = field_obj.metadata.as_ref().and_then(|m| m.description.as_ref()) {
                for line in desc.lines() {
                    if line.is_empty() {
                        writeln!(out, "{inner_pad}#").unwrap();
                    } else {
                        writeln!(out, "{inner_pad}# {line}").unwrap();
                    }
                }
            }

            // Field with contract annotation
            write!(out, "{inner_pad}{field_name} | ").unwrap();

            let is_nullable = is_nullable_type(field_obj);

            // Render type
            render_schema_object(out, field_obj, defs, indent + 1);

            // Default value
            if let Some(default) = field_obj.metadata.as_ref().and_then(|m| m.default.as_ref()) {
                write!(out, " | default = {}", nickel_value(default)).unwrap();
            } else if !is_required || is_nullable {
                write!(out, " | optional").unwrap();
            }

            writeln!(out, ",").unwrap();
        }
    }
    write!(out, "{pad}}}").unwrap();
}

fn render_one_of(out: &mut String, variants: &[Schema], defs: &BTreeMap<String, Schema>, indent: usize) {
    let pad = "  ".repeat(indent);
    let inner_pad = "  ".repeat(indent + 1);

    // Try to detect tagged enum pattern (each variant has a "type" discriminator)
    let is_tagged = variants.iter().all(|v| {
        if let Schema::Object(obj) = v {
            obj.object.as_ref().map_or(false, |o| o.properties.contains_key("type"))
        } else {
            false
        }
    });

    if is_tagged {
        writeln!(out, "[").unwrap();
        for variant in variants {
            if let Schema::Object(obj) = variant {
                if let Some(desc) = obj.metadata.as_ref().and_then(|m| m.description.as_ref()) {
                    writeln!(out, "{inner_pad}# {desc}").unwrap();
                }

                // Extract the tag value
                if let Some(tag) = extract_enum_tag(obj) {
                    write!(out, "{inner_pad}").unwrap();
                    render_schema_object(out, obj, defs, indent + 1);
                    writeln!(out, ",  # tag: {tag}").unwrap();
                } else {
                    write!(out, "{inner_pad}").unwrap();
                    render_schema_object(out, obj, defs, indent + 1);
                    writeln!(out, ",").unwrap();
                }
            }
        }
        write!(out, "{pad}]").unwrap();
    } else {
        // Simple enum — render as Dyn with comment
        write!(out, "Dyn").unwrap();
    }
}

fn extract_enum_tag(schema: &SchemaObject) -> Option<String> {
    let obj = schema.object.as_ref()?;
    let type_schema = obj.properties.get("type")?;
    if let Schema::Object(type_obj) = type_schema {
        if let Some(enum_values) = &type_obj.enum_values {
            if let Some(first) = enum_values.first() {
                return Some(first.to_string());
            }
        }
    }
    None
}

fn is_nullable_type(schema: &SchemaObject) -> bool {
    match &schema.instance_type {
        Some(SingleOrVec::Vec(types)) => types.contains(&InstanceType::Null),
        _ => false,
    }
}

fn instance_type_to_nickel(t: &InstanceType) -> &'static str {
    match t {
        InstanceType::Null => "Dyn",
        InstanceType::Boolean => "Bool",
        InstanceType::Object => "{ _ : Dyn }",
        InstanceType::Array => "Array Dyn",
        InstanceType::Number => "Number",
        InstanceType::String => "String",
        InstanceType::Integer => "Number",
    }
}

fn nickel_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("\"{s}\""),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(nickel_value).collect();
            format!("[{}]", items.join(", "))
        }
        serde_json::Value::Object(map) => {
            let fields: Vec<String> = map.iter().map(|(k, v)| format!("{k} = {}", nickel_value(v))).collect();
            format!("{{ {} }}", fields.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_struct_to_nickel() {
        #[derive(schemars::JsonSchema)]
        /// A test request.
        struct TestRequest {
            /// The name field.
            name: String,
            /// Optional count.
            count: Option<u32>,
            /// Is it active?
            active: bool,
        }

        let schema = schemars::schema_for!(TestRequest);
        let ncl = schema_to_nickel("TestRequest", &schema);

        assert!(ncl.contains("name | String"), "should have name field: {ncl}");
        assert!(ncl.contains("active | Bool"), "should have active field: {ncl}");
        assert!(ncl.contains("count | Number | optional"), "should have optional count: {ncl}");
        assert!(ncl.contains("Auto-generated"), "should have header comment: {ncl}");
    }

    #[test]
    fn test_default_values() {
        #[derive(schemars::JsonSchema)]
        struct WithDefaults {
            #[schemars(default = "default_enabled")]
            enabled: bool,
        }

        fn default_enabled() -> bool {
            true
        }

        let schema = schemars::schema_for!(WithDefaults);
        let ncl = schema_to_nickel("WithDefaults", &schema);

        assert!(ncl.contains("default = true"), "should have default value: {ncl}");
    }

    #[test]
    fn test_multiple_schemas() {
        #[derive(schemars::JsonSchema)]
        /// Request type.
        struct Req {
            url: String,
        }

        #[derive(schemars::JsonSchema)]
        /// Response type.
        struct Resp {
            status: u32,
        }

        let req_schema = schemars::schema_for!(Req);
        let resp_schema = schemars::schema_for!(Resp);

        let ncl = schemas_to_nickel("Test protocol", &[("Req", &req_schema), ("Resp", &resp_schema)]);

        assert!(ncl.contains("let Req ="), "should have Req binding: {ncl}");
        assert!(ncl.contains("let Resp ="), "should have Resp binding: {ncl}");
        assert!(ncl.contains("Req,"), "should export Req: {ncl}");
        assert!(ncl.contains("Resp,"), "should export Resp: {ncl}");
    }
}
