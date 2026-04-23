use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use aspen_client_api::{ClientRpcRequest, ClientRpcResponse};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use syn::punctuated::Punctuated;
use syn::{
    Attribute, Expr, ExprLit, Field, Fields, File, GenericArgument, Item, ItemEnum, ItemStruct, Lit, LitStr,
    PathArguments, Token, Type,
};

const BASELINE_RELATIVE_PATH: &str =
    "../../openspec/changes/extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json";
const CANONICAL_BOOL: bool = true;
const CANONICAL_FLOAT: f64 = 7.5;
const CANONICAL_SEQUENCE_LEN: usize = 1;
const CANONICAL_SIGNED: i64 = -7;
const CANONICAL_UNSIGNED: u64 = 7;
const FEATURE_AUTH: &str = "auth";
const FEATURE_AUTOMERGE: &str = "automerge";
const FEATURE_CI: &str = "ci";
const FORGE_PROTOCOL_SOURCE: &str = "../aspen-forge-protocol/src/lib.rs";
const JOBS_PROTOCOL_SOURCE: &str = "../aspen-jobs-protocol/src/lib.rs";
const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
const MAX_RECURSION_DEPTH: usize = 32;
const MESSAGES_SOURCE_DIR: &str = "src/messages";
const REQUEST_ENUM_NAME: &str = "ClientRpcRequest";
const RESPONSE_ENUM_NAME: &str = "ClientRpcResponse";
const UPDATE_BASELINE_ENV_VAR: &str = "ASPEN_UPDATE_CLIENT_RPC_POSTCARD_BASELINE";
const WIRE_FORMAT: &str = "postcard";
const CANONICAL_BYTE_VALUES: [u8; 4] = [0x11, 0x22, 0x33, 0x44];
const COORDINATION_PROTOCOL_SOURCE: &str = "../aspen-coordination-protocol/src/lib.rs";

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct PostcardBaselineArtifact {
    format: String,
    requests: BTreeMap<String, String>,
    responses: BTreeMap<String, String>,
}

#[derive(Clone)]
struct FieldDefinition {
    rust_name: String,
    serde_name: String,
    ty: Type,
}

#[derive(Clone)]
enum FieldShape {
    Named(Vec<FieldDefinition>),
    Tuple(Vec<Type>),
    Unit,
}

#[derive(Clone)]
struct StructDefinition {
    fields: FieldShape,
}

#[derive(Clone)]
struct EnumVariantDefinition {
    rust_name: String,
    serde_name: String,
    fields: FieldShape,
}

#[derive(Clone)]
struct EnumDefinition {
    variants: Vec<EnumVariantDefinition>,
}

#[derive(Clone)]
enum TypeDefinition {
    Enum(EnumDefinition),
    Struct(StructDefinition),
}

#[derive(Default)]
struct TypeRegistry {
    definitions: BTreeMap<String, TypeDefinition>,
}

impl TypeRegistry {
    fn enum_definition(&self, type_name: &str) -> &EnumDefinition {
        match self.definitions.get(type_name) {
            Some(TypeDefinition::Enum(definition)) => definition,
            Some(TypeDefinition::Struct(_)) => panic!("{type_name} is a struct, not an enum"),
            None => panic!("missing type definition for {type_name}"),
        }
    }

    fn insert(&mut self, type_name: String, definition: TypeDefinition) {
        let previous = self.definitions.insert(type_name.clone(), definition);
        assert!(previous.is_none(), "duplicate type definition for {type_name}");
    }
}

#[test]
fn client_rpc_postcard_baseline() {
    let current_artifact = build_current_artifact();
    let rendered_artifact = render_artifact(&current_artifact);
    let baseline_path = baseline_artifact_path();

    maybe_update_baseline(&baseline_path, &rendered_artifact);

    let saved_artifact = load_saved_artifact(&baseline_path);
    assert_eq!(
        saved_artifact.requests.keys().collect::<Vec<_>>(),
        current_artifact.requests.keys().collect::<Vec<_>>(),
        "saved request baseline keys no longer match live {REQUEST_ENUM_NAME} variants",
    );
    assert_eq!(
        saved_artifact.responses.keys().collect::<Vec<_>>(),
        current_artifact.responses.keys().collect::<Vec<_>>(),
        "saved response baseline keys no longer match live {RESPONSE_ENUM_NAME} variants",
    );

    println!(
        "baseline file: {}\nrequest variants: {}\nresponse variants: {}",
        baseline_path.display(),
        current_artifact.requests.len(),
        current_artifact.responses.len(),
    );

    assert_eq!(
        render_artifact(&saved_artifact),
        rendered_artifact,
        "saved postcard baseline differs from current wire format; regenerate with {UPDATE_BASELINE_ENV_VAR}=1 if intentional",
    );
}

fn baseline_artifact_path() -> PathBuf {
    Path::new(MANIFEST_DIR).join(BASELINE_RELATIVE_PATH)
}

fn build_current_artifact() -> PostcardBaselineArtifact {
    PostcardBaselineArtifact {
        format: WIRE_FORMAT.to_string(),
        requests: generate_enum_samples::<ClientRpcRequest>(REQUEST_ENUM_NAME),
        responses: generate_enum_samples::<ClientRpcResponse>(RESPONSE_ENUM_NAME),
    }
}

fn render_artifact(artifact: &PostcardBaselineArtifact) -> String {
    let mut rendered = serde_json::to_string_pretty(artifact).expect("serialize postcard baseline artifact");
    rendered.push('\n');
    rendered
}

fn maybe_update_baseline(baseline_path: &Path, rendered_artifact: &str) {
    let should_update = env::var_os(UPDATE_BASELINE_ENV_VAR).is_some();
    if !should_update {
        return;
    }
    let parent = baseline_path.parent().expect("baseline path parent");
    fs::create_dir_all(parent).expect("create postcard baseline directory");
    fs::write(baseline_path, rendered_artifact).expect("write postcard baseline artifact");
    println!("updated postcard baseline artifact at {}", baseline_path.display());
}

fn load_saved_artifact(baseline_path: &Path) -> PostcardBaselineArtifact {
    let contents = fs::read_to_string(baseline_path).unwrap_or_else(|error| {
        panic!(
            "failed to read saved postcard baseline {}: {error}. Generate it with {UPDATE_BASELINE_ENV_VAR}=1 cargo test -p aspen-client-api client_rpc_postcard_baseline -- --nocapture",
            baseline_path.display(),
        )
    });
    serde_json::from_str(&contents).expect("parse postcard baseline artifact")
}

fn generate_enum_samples<T>(enum_name: &str) -> BTreeMap<String, String>
where
    T: DeserializeOwned + Serialize,
{
    let registry = type_registry();
    let enum_definition = registry.enum_definition(enum_name);
    let mut samples = BTreeMap::new();
    for variant in &enum_definition.variants {
        let variant_path = format!("{enum_name}::{}", variant.rust_name);
        let value = materialize_variant::<T>(&registry, variant, &variant_path);
        let postcard_hex = encode_postcard_hex(&value, &variant_path);
        let previous = samples.insert(variant.rust_name.clone(), postcard_hex);
        assert!(previous.is_none(), "duplicate variant sample for {variant_path}");
    }
    samples
}

fn materialize_variant<T>(registry: &TypeRegistry, variant: &EnumVariantDefinition, variant_path: &str) -> T
where
    T: DeserializeOwned,
{
    let json_value = json_for_variant(registry, variant, variant_path, 0);
    serde_json::from_value(json_value.clone()).unwrap_or_else(|error| {
        panic!(
            "failed to materialize {variant_path} from canonical JSON {}: {error}",
            json_value,
        )
    })
}

fn encode_postcard_hex<T>(value: &T, variant_path: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    let encoded = postcard::to_allocvec(value).unwrap_or_else(|error| panic!("serialize {variant_path}: {error}"));
    let decoded: T = postcard::from_bytes(&encoded).unwrap_or_else(|error| panic!("deserialize {variant_path}: {error}"));
    let reencoded = postcard::to_allocvec(&decoded).unwrap_or_else(|error| panic!("re-serialize {variant_path}: {error}"));
    assert_eq!(encoded, reencoded, "postcard roundtrip changed bytes for {variant_path}");
    hex_encode(&encoded)
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut rendered = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        rendered.push(nibble_to_hex(byte >> 4));
        rendered.push(nibble_to_hex(byte & 0x0f));
    }
    rendered
}

fn nibble_to_hex(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        10..=15 => char::from(b'a' + (nibble - 10)),
        _ => panic!("invalid hex nibble {nibble}"),
    }
}

fn json_for_variant(
    registry: &TypeRegistry,
    variant: &EnumVariantDefinition,
    variant_path: &str,
    depth: usize,
) -> Value {
    assert!(depth <= MAX_RECURSION_DEPTH, "recursion depth exceeded while building {variant_path}");
    match &variant.fields {
        FieldShape::Unit => Value::String(variant.serde_name.clone()),
        FieldShape::Tuple(types) => {
            let payload = json_for_tuple_fields(registry, types, variant_path, depth + 1);
            Value::Object(Map::from_iter([(variant.serde_name.clone(), payload)]))
        }
        FieldShape::Named(fields) => {
            let payload = json_for_named_fields(registry, fields, variant_path, depth + 1);
            Value::Object(Map::from_iter([(variant.serde_name.clone(), payload)]))
        }
    }
}

fn json_for_local_type(registry: &TypeRegistry, type_name: &str, path: &str, depth: usize) -> Value {
    assert!(depth <= MAX_RECURSION_DEPTH, "recursion depth exceeded while building {path}");
    match registry.definitions.get(type_name) {
        Some(TypeDefinition::Struct(definition)) => json_for_field_shape(registry, &definition.fields, path, depth + 1),
        Some(TypeDefinition::Enum(definition)) => {
            let first_variant = definition
                .variants
                .first()
                .unwrap_or_else(|| panic!("enum {type_name} has no variants"));
            json_for_variant(registry, first_variant, path, depth + 1)
        }
        None => panic!("unsupported field type {type_name} at {path}"),
    }
}

fn json_for_field_shape(registry: &TypeRegistry, shape: &FieldShape, path: &str, depth: usize) -> Value {
    match shape {
        FieldShape::Unit => Value::Null,
        FieldShape::Tuple(types) => json_for_tuple_fields(registry, types, path, depth + 1),
        FieldShape::Named(fields) => json_for_named_fields(registry, fields, path, depth + 1),
    }
}

fn json_for_named_fields(
    registry: &TypeRegistry,
    fields: &[FieldDefinition],
    path: &str,
    depth: usize,
) -> Value {
    let mut object = Map::new();
    for field in fields {
        let field_path = format!("{path}.{}", field.rust_name);
        let field_value = json_for_type(registry, &field.ty, &field_path, depth + 1);
        object.insert(field.serde_name.clone(), field_value);
    }
    Value::Object(object)
}

fn json_for_tuple_fields(registry: &TypeRegistry, types: &[Type], path: &str, depth: usize) -> Value {
    let mut values = Vec::with_capacity(types.len());
    for (index, ty) in types.iter().enumerate() {
        let field_path = format!("{path}.{index}");
        values.push(json_for_type(registry, ty, &field_path, depth + 1));
    }
    match values.len() {
        1 => values.into_iter().next().expect("single tuple element"),
        _ => Value::Array(values),
    }
}

fn json_for_type(registry: &TypeRegistry, ty: &Type, path: &str, depth: usize) -> Value {
    assert!(depth <= MAX_RECURSION_DEPTH, "recursion depth exceeded while building {path}");
    match ty {
        Type::Array(array) => json_for_array_type(registry, array, path, depth + 1),
        Type::Group(group) => json_for_type(registry, &group.elem, path, depth + 1),
        Type::Paren(paren) => json_for_type(registry, &paren.elem, path, depth + 1),
        Type::Path(path_type) => json_for_path_type(registry, path_type, path, depth + 1),
        Type::Reference(reference) => json_for_type(registry, &reference.elem, path, depth + 1),
        Type::Tuple(tuple) => {
            let mut values = Vec::with_capacity(tuple.elems.len());
            for (index, elem) in tuple.elems.iter().enumerate() {
                values.push(json_for_type(registry, elem, &format!("{path}.{index}"), depth + 1));
            }
            Value::Array(values)
        }
        _ => panic!("unsupported field syntax at {path}"),
    }
}

fn json_for_array_type(registry: &TypeRegistry, array: &syn::TypeArray, path: &str, depth: usize) -> Value {
    let length = parse_array_length(&array.len, path);
    let mut values = Vec::with_capacity(length);
    if is_u8_type(&array.elem) {
        for index in 0..length {
            values.push(Value::Number(Number::from(canonical_byte_value(index))));
        }
        return Value::Array(values);
    }
    for index in 0..length {
        values.push(json_for_type(registry, &array.elem, &format!("{path}.{index}"), depth + 1));
    }
    Value::Array(values)
}

fn json_for_path_type(registry: &TypeRegistry, path_type: &syn::TypePath, path: &str, depth: usize) -> Value {
    let segment = path_type.path.segments.last().expect("path segment");
    let type_name = segment.ident.to_string();
    match type_name.as_str() {
        "bool" => Value::Bool(CANONICAL_BOOL),
        "String" => Value::String(canonical_string(path)),
        "f32" | "f64" => Value::Number(Number::from_f64(CANONICAL_FLOAT).expect("finite canonical float")),
        "HashMap" | "BTreeMap" => json_for_map_type(registry, segment, path, depth + 1),
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => Value::Number(Number::from(CANONICAL_SIGNED)),
        "Option" => json_for_option_type(registry, segment, path, depth + 1),
        "u8" => Value::Number(Number::from(CANONICAL_BYTE_VALUES[0])),
        "u16" | "u32" | "u64" | "u128" | "usize" => Value::Number(Number::from(CANONICAL_UNSIGNED)),
        "Vec" => json_for_vec_type(registry, segment, path, depth + 1),
        _ => json_for_local_type(registry, &type_name, path, depth + 1),
    }
}

fn json_for_map_type(
    registry: &TypeRegistry,
    segment: &syn::PathSegment,
    path: &str,
    depth: usize,
) -> Value {
    let arguments = generic_arguments(segment, path);
    assert!(arguments.len() == 2, "map type at {path} must have key and value arguments");
    let key_type = &arguments[0];
    let value_type = &arguments[1];
    let key_string = canonical_map_key(key_type, path);
    let value = json_for_type(registry, value_type, &format!("{path}.value"), depth + 1);
    Value::Object(Map::from_iter([(key_string, value)]))
}

fn json_for_option_type(
    registry: &TypeRegistry,
    segment: &syn::PathSegment,
    path: &str,
    depth: usize,
) -> Value {
    if depth > MAX_RECURSION_DEPTH {
        return Value::Null;
    }
    let arguments = generic_arguments(segment, path);
    assert!(arguments.len() == 1, "option type at {path} must have one argument");
    json_for_type(registry, &arguments[0], path, depth + 1)
}

fn json_for_vec_type(registry: &TypeRegistry, segment: &syn::PathSegment, path: &str, depth: usize) -> Value {
    let arguments = generic_arguments(segment, path);
    assert!(arguments.len() == 1, "vec type at {path} must have one argument");
    let inner_type = &arguments[0];
    let mut values = Vec::with_capacity(CANONICAL_SEQUENCE_LEN.max(CANONICAL_BYTE_VALUES.len()));
    if is_u8_type(inner_type) {
        for &byte in &CANONICAL_BYTE_VALUES {
            values.push(Value::Number(Number::from(byte)));
        }
        return Value::Array(values);
    }
    for index in 0..CANONICAL_SEQUENCE_LEN {
        values.push(json_for_type(registry, inner_type, &format!("{path}.{index}"), depth + 1));
    }
    Value::Array(values)
}

fn generic_arguments(segment: &syn::PathSegment, path: &str) -> Vec<Type> {
    match &segment.arguments {
        PathArguments::AngleBracketed(arguments) => collect_type_arguments(&arguments.args),
        _ => panic!("expected generic arguments at {path}"),
    }
}

fn collect_type_arguments(arguments: &Punctuated<GenericArgument, syn::token::Comma>) -> Vec<Type> {
    arguments
        .iter()
        .filter_map(|argument| match argument {
            GenericArgument::Type(ty) => Some(ty.clone()),
            _ => None,
        })
        .collect()
}

fn is_u8_type(ty: &Type) -> bool {
    match ty {
        Type::Path(path_type) => path_type
            .path
            .segments
            .last()
            .map(|segment| segment.ident == "u8")
            .unwrap_or(false),
        _ => false,
    }
}

fn canonical_byte_value(index: usize) -> u8 {
    CANONICAL_BYTE_VALUES[index % CANONICAL_BYTE_VALUES.len()]
}

fn canonical_map_key(key_type: &Type, path: &str) -> String {
    match key_type {
        Type::Path(path_type) => {
            let type_name = path_type.path.segments.last().expect("map key segment").ident.to_string();
            match type_name.as_str() {
                "String" => canonical_string(path),
                "u8" => CANONICAL_BYTE_VALUES[0].to_string(),
                "u16" | "u32" | "u64" | "u128" | "usize" => CANONICAL_UNSIGNED.to_string(),
                "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => CANONICAL_SIGNED.to_string(),
                _ => canonical_string(path),
            }
        }
        _ => canonical_string(path),
    }
}

fn canonical_string(path: &str) -> String {
    format!("sample::{path}")
}

fn parse_array_length(length: &Expr, path: &str) -> usize {
    match length {
        Expr::Lit(ExprLit { lit: Lit::Int(value), .. }) => {
            value.base10_parse().unwrap_or_else(|error| panic!("parse array length at {path}: {error}"))
        }
        _ => panic!("unsupported array length expression at {path}"),
    }
}

fn type_registry() -> TypeRegistry {
    load_type_registry()
}

fn load_type_registry() -> TypeRegistry {
    let mut registry = TypeRegistry::default();
    for source_path in source_paths() {
        let file_contents = fs::read_to_string(&source_path)
            .unwrap_or_else(|error| panic!("read source file {}: {error}", source_path.display()));
        let syntax = syn::parse_file(&file_contents)
            .unwrap_or_else(|error| panic!("parse source file {}: {error}", source_path.display()));
        register_file_items(&mut registry, &syntax);
    }
    registry
}

fn source_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    paths.extend(rust_files_under(&Path::new(MANIFEST_DIR).join(MESSAGES_SOURCE_DIR)));
    paths.push(Path::new(MANIFEST_DIR).join(COORDINATION_PROTOCOL_SOURCE));
    paths.push(Path::new(MANIFEST_DIR).join(FORGE_PROTOCOL_SOURCE));
    paths.push(Path::new(MANIFEST_DIR).join(JOBS_PROTOCOL_SOURCE));
    paths
}

fn rust_files_under(directory: &Path) -> Vec<PathBuf> {
    let mut directories = vec![directory.to_path_buf()];
    let mut files = Vec::new();
    while let Some(current) = directories.pop() {
        let entries = fs::read_dir(&current).unwrap_or_else(|error| panic!("read directory {}: {error}", current.display()));
        for entry in entries {
            let entry = entry.unwrap_or_else(|error| panic!("read directory entry in {}: {error}", current.display()));
            let path = entry.path();
            if path.is_dir() {
                directories.push(path);
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn register_file_items(registry: &mut TypeRegistry, syntax: &File) {
    for item in &syntax.items {
        match item {
            Item::Enum(item_enum) if cfg_is_enabled(&item_enum.attrs) => {
                registry.insert(item_enum.ident.to_string(), TypeDefinition::Enum(enum_definition(item_enum)));
            }
            Item::Struct(item_struct) if cfg_is_enabled(&item_struct.attrs) => {
                registry.insert(
                    item_struct.ident.to_string(),
                    TypeDefinition::Struct(struct_definition(item_struct)),
                );
            }
            _ => {}
        }
    }
}

fn struct_definition(item_struct: &ItemStruct) -> StructDefinition {
    StructDefinition {
        fields: field_shape(&item_struct.fields),
    }
}

fn enum_definition(item_enum: &ItemEnum) -> EnumDefinition {
    let mut variants = Vec::new();
    for variant in &item_enum.variants {
        if !cfg_is_enabled(&variant.attrs) {
            continue;
        }
        variants.push(EnumVariantDefinition {
            rust_name: variant.ident.to_string(),
            serde_name: serde_name(&variant.attrs, &variant.ident.to_string()),
            fields: field_shape(&variant.fields),
        });
    }
    EnumDefinition { variants }
}

fn field_shape(fields: &Fields) -> FieldShape {
    match fields {
        Fields::Named(named_fields) => FieldShape::Named(named_fields.named.iter().map(field_definition).collect()),
        Fields::Unnamed(unnamed_fields) => FieldShape::Tuple(
            unnamed_fields
                .unnamed
                .iter()
                .map(|field| field.ty.clone())
                .collect(),
        ),
        Fields::Unit => FieldShape::Unit,
    }
}

fn field_definition(field: &Field) -> FieldDefinition {
    let rust_name = field.ident.as_ref().expect("named field ident").to_string();
    FieldDefinition {
        serde_name: serde_name(&field.attrs, &rust_name),
        rust_name,
        ty: field.ty.clone(),
    }
}

fn serde_name(attributes: &[Attribute], default_name: &str) -> String {
    let mut rename = None;
    for attribute in attributes {
        if !attribute.path().is_ident("serde") {
            continue;
        }
        let parse_result = attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value: LitStr = meta.value()?.parse()?;
                rename = Some(value.value());
                return Ok(());
            }
            if meta.input.peek(Token![=]) {
                let _: Lit = meta.value()?.parse()?;
            }
            Ok(())
        });
        if let Err(error) = parse_result {
            panic!("parse serde attribute on {default_name}: {error}");
        }
    }
    rename.unwrap_or_else(|| default_name.to_string())
}

fn cfg_is_enabled(attributes: &[Attribute]) -> bool {
    for attribute in attributes {
        if !attribute.path().is_ident("cfg") {
            continue;
        }
        let mut enabled = true;
        let parse_result = attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("feature") {
                let value: LitStr = meta.value()?.parse()?;
                enabled &= feature_is_enabled(&value.value());
            }
            Ok(())
        });
        if let Err(error) = parse_result {
            panic!("parse cfg attribute: {error}");
        }
        if !enabled {
            return false;
        }
    }
    true
}

fn feature_is_enabled(feature_name: &str) -> bool {
    let is_auth_feature = feature_name == FEATURE_AUTH && cfg!(feature = "auth");
    let is_automerge_feature = feature_name == FEATURE_AUTOMERGE && cfg!(feature = "automerge");
    let is_ci_feature = feature_name == FEATURE_CI && cfg!(feature = "ci");
    is_auth_feature || is_automerge_feature || is_ci_feature
}
