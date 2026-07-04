# Shared pure Nickel helpers for repository-owned authoring contracts.
# Runtime Rust consumes checked exports; it does not evaluate this prelude as authority.

let Blake3RefPattern = "^blake3:[0-9a-f]{64}$" in
let StableIdPattern = "^[A-Za-z0-9_.:-]+$" in
let SafeRelativeDirPattern = "^[A-Za-z0-9._-]+$" in
let AbsolutePathPattern = "^/[A-Za-z0-9._/-]+$" in
let SemverPattern = "^[0-9]+\\.[0-9]+\\.[0-9]+$" in

let non_empty_string = fun value =>
  std.is_string value && std.string.length value > 0 in

let blake3_ref = fun value =>
  std.is_string value && std.string.is_match Blake3RefPattern value in

let stable_id = fun value =>
  std.is_string value && std.string.is_match StableIdPattern value in

let semver = fun value =>
  std.is_string value && std.string.is_match SemverPattern value in

let absolute_path = fun value =>
  std.is_string value
  && std.string.length value > 1
  && std.string.is_match AbsolutePathPattern value
  && !std.string.is_match "(^|/)\\.\\.(/|$)" value in

let safe_relative_dir = fun value =>
  std.is_string value
  && std.string.length value > 0
  && std.string.is_match SafeRelativeDirPattern value
  && value != "."
  && value != ".."
  && !std.string.is_match "^\\." value in

let integer = fun value =>
  std.is_number value && std.number.is_integer value in

let positive_integer = fun value =>
  integer value && value > 0 in

let non_negative_integer = fun value =>
  integer value && value >= 0 in

let exact_string = fun expected value =>
  std.is_string value && value == expected in

let exact_integer = fun expected value =>
  integer value && value == expected in

let non_empty_array = fun values =>
  std.is_array values && std.array.length values > 0 in

let array_matching = fun predicate values =>
  non_empty_array values && std.array.all predicate values in

let optional_array_matching = fun predicate values =>
  std.is_array values && std.array.all predicate values in

let one_of = fun allowed value =>
  std.is_string value && std.array.elem value allowed in

let contains_all = fun actual required =>
  std.array.all (fun item => std.array.elem item actual) required in

let unique_strings = fun values =>
  std.array.length values == std.array.length (std.array.sort_dedup std.string.compare values) in

let unique_by = fun projection values =>
  unique_strings (std.array.map projection values) in

let mk_contract = fun predicate => std.contract.from_predicate predicate in
let MetadataEnvelope = {
  schema_id | mk_contract non_empty_string,
  schema_version | mk_contract positive_integer,
  source_language | mk_contract non_empty_string,
  export_identity | mk_contract non_empty_string,
  payload | Dyn,
} in
let make_metadata_envelope = fun schema_id schema_version source_language export_identity payload => {
  schema_id = schema_id,
  schema_version = schema_version,
  source_language = source_language,
  export_identity = export_identity,
  payload = payload,
} | MetadataEnvelope in

{
  patterns = {
    blake3_ref = Blake3RefPattern,
    stable_id = StableIdPattern,
    safe_relative_dir = SafeRelativeDirPattern,
    absolute_path = AbsolutePathPattern,
    semver = SemverPattern,
  },

  is_non_empty_string = non_empty_string,
  is_blake3_ref = blake3_ref,
  is_stable_id = stable_id,
  is_semver = semver,
  is_absolute_path = absolute_path,
  is_safe_relative_dir = safe_relative_dir,
  is_integer = integer,
  is_positive_integer = positive_integer,
  is_non_negative_integer = non_negative_integer,
  is_exact_string = exact_string,
  is_exact_integer = exact_integer,
  is_non_empty_array = non_empty_array,
  array_of = array_matching,
  optional_array_of = optional_array_matching,
  is_one_of = one_of,
  has_all = contains_all,
  distinct_strings = unique_strings,
  distinct_by = unique_by,
  contract = mk_contract,
  metadata_envelope = make_metadata_envelope,

  contracts = {
    NonEmptyString = mk_contract non_empty_string,
    Blake3Ref = mk_contract blake3_ref,
    StableId = mk_contract stable_id,
    Semver = mk_contract semver,
    AbsolutePath = mk_contract absolute_path,
    SafeRelativeDir = mk_contract safe_relative_dir,
    PositiveInteger = mk_contract positive_integer,
    NonNegativeInteger = mk_contract non_negative_integer,
    NonEmptyArray = mk_contract non_empty_array,
    MetadataEnvelope = MetadataEnvelope,
  },
}
