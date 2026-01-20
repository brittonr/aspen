//! Type definitions for parsed configuration metadata.

/// A parsed configuration file containing structs and enums.
#[derive(Debug, Clone, Default)]
pub struct ParsedConfig {
    /// All configuration structs found.
    pub structs: Vec<ConfigStruct>,
    /// All enums used in configuration.
    pub enums: Vec<ConfigEnum>,
}

/// A parsed configuration struct.
#[derive(Debug, Clone)]
pub struct ConfigStruct {
    /// Struct name.
    pub name: String,
    /// Documentation comment.
    pub doc: Option<String>,
    /// All fields in the struct.
    pub fields: Vec<ConfigField>,
    /// Whether this struct derives Default.
    pub has_default: bool,
}

/// A parsed struct field.
#[derive(Debug, Clone)]
pub struct ConfigField {
    /// Field name (Rust identifier).
    pub name: String,
    /// Serde-renamed field name (if different).
    pub serde_name: Option<String>,
    /// Field type.
    pub field_type: FieldType,
    /// Documentation comment.
    pub doc: Option<String>,
    /// Whether the field is optional (Option<T>).
    pub is_optional: bool,
    /// Default value expression (from #[serde(default = "...")]).
    pub default_fn: Option<String>,
    /// Whether #[serde(default)] is present (uses type's Default).
    pub has_serde_default: bool,
    /// Whether the field is skipped in serialization.
    pub skip: bool,
    /// Whether the field is flattened.
    pub flatten: bool,
}

/// Parsed field type information.
#[derive(Debug, Clone)]
pub enum FieldType {
    /// Primitive types: bool, u32, u64, i32, i64, f32, f64, usize, isize.
    Primitive(PrimitiveType),
    /// String type.
    String,
    /// PathBuf type.
    PathBuf,
    /// SocketAddr type.
    SocketAddr,
    /// Vec<T> type.
    Vec(Box<FieldType>),
    /// Option<T> type (handled separately via is_optional).
    Option(Box<FieldType>),
    /// A nested struct type.
    Struct(String),
    /// An enum type.
    Enum(String),
    /// Unknown or unsupported type.
    Unknown(String),
}

/// Primitive numeric and boolean types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    Usize,
    I8,
    I16,
    I32,
    I64,
    Isize,
    F32,
    F64,
}

impl PrimitiveType {
    /// Convert to Nickel contract type.
    pub fn to_nickel(&self) -> &'static str {
        match self {
            Self::Bool => "Bool",
            Self::U8
            | Self::U16
            | Self::U32
            | Self::U64
            | Self::Usize
            | Self::I8
            | Self::I16
            | Self::I32
            | Self::I64
            | Self::Isize => "Number",
            Self::F32 | Self::F64 => "Number",
        }
    }
}

/// A parsed enum definition.
#[derive(Debug, Clone)]
pub struct ConfigEnum {
    /// Enum name.
    pub name: String,
    /// Documentation comment.
    pub doc: Option<String>,
    /// Enum variants.
    pub variants: Vec<EnumVariant>,
    /// Serde rename_all attribute (e.g., "snake_case", "lowercase").
    pub rename_all: Option<String>,
    /// Default variant (if #[default] is present).
    pub default_variant: Option<String>,
}

/// A parsed enum variant.
#[derive(Debug, Clone)]
pub struct EnumVariant {
    /// Variant name.
    pub name: String,
    /// Documentation comment.
    pub doc: Option<String>,
    /// Whether this is the default variant.
    pub is_default: bool,
    /// Serde-renamed variant name.
    pub serde_name: Option<String>,
}

impl FieldType {
    /// Convert to a Nickel contract expression.
    pub fn to_nickel(&self) -> String {
        match self {
            Self::Primitive(p) => p.to_nickel().to_string(),
            Self::String => "String".to_string(),
            Self::PathBuf => "String".to_string(),
            Self::SocketAddr => "String".to_string(),
            Self::Vec(inner) => format!("Array {}", inner.to_nickel()),
            Self::Option(inner) => inner.to_nickel(),
            Self::Struct(name) => name.clone(),
            Self::Enum(name) => name.clone(),
            Self::Unknown(s) => format!("Dyn /* unknown: {s} */"),
        }
    }
}
