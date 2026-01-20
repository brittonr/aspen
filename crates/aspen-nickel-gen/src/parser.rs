//! Rust source file parser for configuration structs.

use std::path::Path;

use syn::Attribute;
use syn::Expr;
use syn::Fields;
use syn::GenericArgument;
use syn::Item;
use syn::Lit;
use syn::Meta;
use syn::PathArguments;
use syn::Token;
use syn::Type;
use syn::punctuated::Punctuated;

use crate::error::GeneratorError;
use crate::types::ConfigEnum;
use crate::types::ConfigField;
use crate::types::ConfigStruct;
use crate::types::EnumVariant;
use crate::types::FieldType;
use crate::types::ParsedConfig;
use crate::types::PrimitiveType;

/// Parse a Rust source file and extract configuration metadata.
pub fn parse_config_file(path: &Path) -> Result<ParsedConfig, GeneratorError> {
    let content = std::fs::read_to_string(path).map_err(|source| GeneratorError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;

    parse_config_str(&content)
}

/// Parse Rust source code string and extract configuration metadata.
pub fn parse_config_str(content: &str) -> Result<ParsedConfig, GeneratorError> {
    let file = syn::parse_file(content).map_err(|e| GeneratorError::ParseError { message: e.to_string() })?;

    let mut config = ParsedConfig::default();

    for item in file.items {
        match item {
            Item::Struct(item_struct) => {
                // Check if it has serde Serialize/Deserialize derives
                if has_serde_derive(&item_struct.attrs) || item_struct.ident.to_string().ends_with("Config") {
                    if let Some(parsed) = parse_struct(&item_struct) {
                        config.structs.push(parsed);
                    }
                }
            }
            Item::Enum(item_enum) => {
                // Check if it has serde derives or is used in config
                if has_serde_derive(&item_enum.attrs) {
                    if let Some(parsed) = parse_enum(&item_enum) {
                        config.enums.push(parsed);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(config)
}

/// Check if a type has serde Serialize or Deserialize derives.
fn has_serde_derive(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("derive") {
            if let Ok(nested) = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) {
                for meta in nested {
                    if let Meta::Path(path) = meta {
                        let ident = path.segments.last().map(|s| s.ident.to_string());
                        if matches!(ident.as_deref(), Some("Serialize") | Some("Deserialize")) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Check if a type has Default derive.
fn has_default_derive(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("derive") {
            if let Ok(nested) = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) {
                for meta in nested {
                    if let Meta::Path(path) = meta {
                        if path.is_ident("Default") {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Extract doc comments from attributes.
fn extract_doc(attrs: &[Attribute]) -> Option<String> {
    let docs: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                if let Meta::NameValue(meta) = &attr.meta {
                    if let Expr::Lit(expr_lit) = &meta.value {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            return Some(lit_str.value().trim().to_string());
                        }
                    }
                }
            }
            None
        })
        .collect();

    if docs.is_empty() { None } else { Some(docs.join("\n")) }
}

/// Parse a struct definition.
fn parse_struct(item: &syn::ItemStruct) -> Option<ConfigStruct> {
    let name = item.ident.to_string();
    let doc = extract_doc(&item.attrs);
    let has_default = has_default_derive(&item.attrs);

    let fields = match &item.fields {
        Fields::Named(fields) => fields.named.iter().filter_map(parse_field).collect(),
        _ => return None, // Skip tuple structs
    };

    Some(ConfigStruct {
        name,
        doc,
        fields,
        has_default,
    })
}

/// Parse a struct field.
fn parse_field(field: &syn::Field) -> Option<ConfigField> {
    let name = field.ident.as_ref()?.to_string();
    let doc = extract_doc(&field.attrs);

    // Parse serde attributes
    let serde_attrs = parse_serde_attrs(&field.attrs);

    // Skip fields marked with #[serde(skip)]
    if serde_attrs.skip {
        return None;
    }

    // Parse field type
    let (field_type, is_optional) = parse_type(&field.ty);

    Some(ConfigField {
        name,
        serde_name: serde_attrs.rename,
        field_type,
        doc,
        is_optional,
        default_fn: serde_attrs.default_fn,
        has_serde_default: serde_attrs.has_default,
        skip: serde_attrs.skip,
        flatten: serde_attrs.flatten,
    })
}

/// Parsed serde attributes for a field.
#[derive(Default)]
struct SerdeFieldAttrs {
    rename: Option<String>,
    default_fn: Option<String>,
    has_default: bool,
    skip: bool,
    flatten: bool,
}

/// Parse serde attributes from a field.
fn parse_serde_attrs(attrs: &[Attribute]) -> SerdeFieldAttrs {
    let mut result = SerdeFieldAttrs::default();

    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }

        let Ok(nested) = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) else {
            continue;
        };

        for meta in nested {
            match &meta {
                Meta::Path(path) => {
                    if path.is_ident("default") {
                        result.has_default = true;
                    } else if path.is_ident("skip") {
                        result.skip = true;
                    } else if path.is_ident("flatten") {
                        result.flatten = true;
                    }
                }
                Meta::NameValue(nv) => {
                    if nv.path.is_ident("default") {
                        if let Expr::Lit(expr_lit) = &nv.value {
                            if let Lit::Str(lit_str) = &expr_lit.lit {
                                result.default_fn = Some(lit_str.value());
                            }
                        }
                    } else if nv.path.is_ident("rename") {
                        if let Expr::Lit(expr_lit) = &nv.value {
                            if let Lit::Str(lit_str) = &expr_lit.lit {
                                result.rename = Some(lit_str.value());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    result
}

/// Parse a Rust type into our type representation.
fn parse_type(ty: &Type) -> (FieldType, bool) {
    match ty {
        Type::Path(type_path) => {
            let segment = match type_path.path.segments.last() {
                Some(s) => s,
                None => return (FieldType::Unknown("empty path".to_string()), false),
            };

            let ident = segment.ident.to_string();

            // Check for Option<T>
            if ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        let (inner_type, _) = parse_type(inner);
                        return (FieldType::Option(Box::new(inner_type.clone())), true);
                    }
                }
                return (FieldType::Unknown("Option<?>".to_string()), true);
            }

            // Check for Vec<T>
            if ident == "Vec" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        let (inner_type, _) = parse_type(inner);
                        return (FieldType::Vec(Box::new(inner_type)), false);
                    }
                }
                return (FieldType::Vec(Box::new(FieldType::Unknown("?".to_string()))), false);
            }

            // Primitive types
            let primitive = match ident.as_str() {
                "bool" => Some(PrimitiveType::Bool),
                "u8" => Some(PrimitiveType::U8),
                "u16" => Some(PrimitiveType::U16),
                "u32" => Some(PrimitiveType::U32),
                "u64" => Some(PrimitiveType::U64),
                "usize" => Some(PrimitiveType::Usize),
                "i8" => Some(PrimitiveType::I8),
                "i16" => Some(PrimitiveType::I16),
                "i32" => Some(PrimitiveType::I32),
                "i64" => Some(PrimitiveType::I64),
                "isize" => Some(PrimitiveType::Isize),
                "f32" => Some(PrimitiveType::F32),
                "f64" => Some(PrimitiveType::F64),
                _ => None,
            };

            if let Some(p) = primitive {
                return (FieldType::Primitive(p), false);
            }

            // String types
            if ident == "String" || ident == "str" {
                return (FieldType::String, false);
            }

            // Path types
            if ident == "PathBuf" || ident == "Path" {
                return (FieldType::PathBuf, false);
            }

            // Socket address
            if ident == "SocketAddr" {
                return (FieldType::SocketAddr, false);
            }

            // Check if it looks like an enum (simple naming heuristic)
            if ident.ends_with("Mode") || ident.ends_with("Backend") || ident.ends_with("Strategy") {
                return (FieldType::Enum(ident), false);
            }

            // Assume it's a nested struct/config type
            if ident.ends_with("Config") {
                return (FieldType::Struct(ident), false);
            }

            // Unknown type - treat as struct reference
            (FieldType::Struct(ident), false)
        }
        _ => (FieldType::Unknown(quote::quote!(#ty).to_string()), false),
    }
}

/// Parse an enum definition.
fn parse_enum(item: &syn::ItemEnum) -> Option<ConfigEnum> {
    let name = item.ident.to_string();
    let doc = extract_doc(&item.attrs);

    // Parse serde rename_all
    let rename_all = parse_serde_rename_all(&item.attrs);

    let mut variants = Vec::new();
    let mut default_variant = None;

    for variant in &item.variants {
        let variant_name = variant.ident.to_string();
        let variant_doc = extract_doc(&variant.attrs);
        let is_default = has_default_attr(&variant.attrs);

        if is_default {
            default_variant = Some(variant_name.clone());
        }

        // Get serde rename for variant
        let serde_name = parse_serde_variant_rename(&variant.attrs);

        variants.push(EnumVariant {
            name: variant_name,
            doc: variant_doc,
            is_default,
            serde_name,
        });
    }

    Some(ConfigEnum {
        name,
        doc,
        variants,
        rename_all,
        default_variant,
    })
}

/// Parse #[serde(rename_all = "...")] attribute.
fn parse_serde_rename_all(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }

        let Ok(nested) = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) else {
            continue;
        };

        for meta in nested {
            if let Meta::NameValue(nv) = meta {
                if nv.path.is_ident("rename_all") {
                    if let Expr::Lit(expr_lit) = &nv.value {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            return Some(lit_str.value());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Parse #[serde(rename = "...")] for enum variants.
fn parse_serde_variant_rename(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }

        let Ok(nested) = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) else {
            continue;
        };

        for meta in nested {
            if let Meta::NameValue(nv) = meta {
                if nv.path.is_ident("rename") {
                    if let Expr::Lit(expr_lit) = &nv.value {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            return Some(lit_str.value());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Check for #[default] attribute (used on enum variants).
fn has_default_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("default"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_struct() {
        let source = r#"
            /// My config struct.
            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct MyConfig {
                /// The name field.
                pub name: String,
                /// The count field.
                #[serde(default)]
                pub count: u32,
            }
        "#;

        let config = parse_config_str(source).unwrap();
        assert_eq!(config.structs.len(), 1);

        let s = &config.structs[0];
        assert_eq!(s.name, "MyConfig");
        assert_eq!(s.fields.len(), 2);

        let name_field = &s.fields[0];
        assert_eq!(name_field.name, "name");
        assert!(matches!(name_field.field_type, FieldType::String));

        let count_field = &s.fields[1];
        assert_eq!(count_field.name, "count");
        assert!(count_field.has_serde_default);
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
            #[derive(Debug, Clone, Serialize, Deserialize, Default)]
            #[serde(rename_all = "snake_case")]
            pub enum StorageBackend {
                InMemory,
                #[default]
                Redb,
            }
        "#;

        let config = parse_config_str(source).unwrap();
        assert_eq!(config.enums.len(), 1);

        let e = &config.enums[0];
        assert_eq!(e.name, "StorageBackend");
        assert_eq!(e.rename_all, Some("snake_case".to_string()));
        assert_eq!(e.default_variant, Some("Redb".to_string()));
        assert_eq!(e.variants.len(), 2);
    }

    #[test]
    fn test_parse_optional_field() {
        let source = r#"
            #[derive(Serialize, Deserialize)]
            pub struct Config {
                pub required: String,
                pub optional: Option<String>,
            }
        "#;

        let config = parse_config_str(source).unwrap();
        let s = &config.structs[0];

        assert!(!s.fields[0].is_optional);
        assert!(s.fields[1].is_optional);
    }

    #[test]
    fn test_parse_vec_field() {
        let source = r#"
            #[derive(Serialize, Deserialize)]
            pub struct Config {
                pub items: Vec<String>,
            }
        "#;

        let config = parse_config_str(source).unwrap();
        let field = &config.structs[0].fields[0];

        assert!(matches!(field.field_type, FieldType::Vec(_)));
    }
}
