//! Record-reader denials, rendered exactly as the `molten` rail rendered them.

type String = alloc::string::String;

/// The `Display` of the rail's `Failure::InvalidHarness`. A rail content-reference denial nested
/// that failure inside its own message, so the nested text keeps it.
pub const NESTED_INVALID_HARNESS_PREFIX: &str = "invalid harness artifact: ";

/// Why a record reader or a typed id parser denied a value.
///
/// `Display` is the message of the rail's `Failure` for the same denial;
/// [`ViewError::failure_kind`] (through [`crate::fault::Classified`]) is its class. Every
/// reader denial is an invalid-harness denial, like the rail's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewError {
    /// `expected <{label} ...> with arity {arity}`.
    RecordShape { label: String, arity: u64 },
    /// `expected string for {field}`.
    ExpectedString { field: String },
    /// `expected sequence for {field}`.
    ExpectedSequence { field: String },
    /// A string that is not a canonical content reference.
    ContentRef {
        field: String,
        value: String,
        error: crate::identity::RefError,
    },
    /// `{field} must be <none> or <some "blake3:...">`.
    OptionalRefShape { field: String },
    /// `unsupported {context} check status {status}`: only `pass`, `fail` and `deny` are admitted.
    UnsupportedContextCheckStatus { context: String, status: String },
    /// `duplicate {context} check {name}`.
    DuplicateCheck { context: String, name: String },
    /// `missing {context} check {name}`.
    MissingCheck { context: String, name: String },
    /// `{label} cannot be empty`.
    EmptyId { label: String },
    /// A stable id with a character outside `[A-Za-z0-9_.:-]`.
    IdCharacters { label: String, value: String },
    /// An operation id that does not start with a lowercase ASCII letter.
    OperationIdFirstCharacter { value: String },
    /// `unsupported decision {value}`.
    UnsupportedDecision { value: String },
    /// `unsupported check status {value}`.
    UnsupportedCheckStatus { value: String },
    /// `unsupported replay class {value}`.
    UnsupportedReplayClass { value: String },
    /// A count bound on a sequence the reader walked.
    Bound(crate::bounded::BoundError),
}

impl core::fmt::Display for ViewError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::RecordShape { label, arity } => write!(formatter, "expected <{label} ...> with arity {arity}"),
            Self::ExpectedString { field } => write!(formatter, "expected string for {field}"),
            Self::ExpectedSequence { field } => write!(formatter, "expected sequence for {field}"),
            Self::ContentRef { field, value, error } => write!(
                formatter,
                "{field} must be a canonical content ref: {NESTED_INVALID_HARNESS_PREFIX}{error}, got {value}"
            ),
            Self::OptionalRefShape { field } => write!(formatter, "{field} must be <none> or <some \"blake3:...\">"),
            Self::UnsupportedContextCheckStatus { context, status } => {
                write!(formatter, "unsupported {context} check status {status}")
            }
            Self::DuplicateCheck { context, name } => write!(formatter, "duplicate {context} check {name}"),
            Self::MissingCheck { context, name } => write!(formatter, "missing {context} check {name}"),
            Self::EmptyId { label } => write!(formatter, "{label} cannot be empty"),
            Self::IdCharacters { label, value } => {
                write!(formatter, "{label} must contain only ASCII letters, digits, '_', '.', ':', or '-', got {value}")
            }
            Self::OperationIdFirstCharacter { value } => {
                write!(formatter, "operation id first character must be lowercase ascii, got {value}")
            }
            Self::UnsupportedDecision { value } => write!(formatter, "unsupported decision {value}"),
            Self::UnsupportedCheckStatus { value } => write!(formatter, "unsupported check status {value}"),
            Self::UnsupportedReplayClass { value } => write!(formatter, "unsupported replay class {value}"),
            Self::Bound(error) => core::fmt::Display::fmt(error, formatter),
        }
    }
}

/// No `source`: `Display` already carries the nested reference or bound diagnostic, as the rail's
/// message did.
impl core::error::Error for ViewError {}

impl crate::fault::Classified for ViewError {
    fn failure_kind(&self) -> crate::fault::FailureKind {
        crate::fault::FailureKind::InvalidHarness
    }
}

impl From<crate::bounded::BoundError> for ViewError {
    fn from(error: crate::bounded::BoundError) -> Self {
        Self::Bound(error)
    }
}
