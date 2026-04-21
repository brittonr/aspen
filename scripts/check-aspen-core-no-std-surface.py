#!/usr/bin/env python3
"""Validate the documented alloc-only `aspen-core` surface and emit review artifacts."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import re
import sys
from typing import Iterable

CFG_PREFIX = "#[cfg"
EMPTY_TEXT = ""
GUARD_NONE = "always"
LIB_RS_NAME = "lib.rs"
PRELUDE_RS_NAME = "prelude.rs"
SPEC_MOD_RS_PATH = Path("spec/mod.rs")
VERIFIED_MOD_RS_PATH = Path("verified/mod.rs")
VERIFIED_SCAN_RS_PATH = Path("verified/scan.rs")
SURFACE_OUTPUT_FILE_NAME = "surface-inventory.md"
EXPORT_MAP_FILE_NAME = "export-map.md"
SOURCE_AUDIT_FILE_NAME = "source-audit.txt"
PUBLIC_VISIBILITY = "pub"
CRATE_VISIBILITY = "pub(crate)"
STD_GUARD = '#[cfg(feature = "std")]'
SQL_GUARD = '#[cfg(feature = "sql")]'
LAYER_GUARD = '#[cfg(all(feature = "std", feature = "layer"))]'
GLOBAL_DISCOVERY_GUARD = '#[cfg(all(feature = "std", feature = "global-discovery"))]'
TEST_STD_GUARD = '#[cfg(all(test, feature = "std"))]'
GLOB_SUFFIX = "::*"
TEST_CFG_PATTERN = re.compile(r"^#\[cfg\([^\]]*\btest\b")
MODULE_PATTERN = re.compile(r"^(pub(?:\([^)]*\))?)\s+mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;")
REEXPORT_PATTERN = re.compile(r"^(pub(?:\([^)]*\))?)\s+use\s+(.+?)\s*;")
PUBLIC_FUNCTION_PATTERN = re.compile(r"^pub\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)", re.MULTILINE)
ALIAS_SPLIT_PATTERN = re.compile(r"\s+as\s+")
SHELL_BACKEDGE_PATTERN = re.compile(r"crate::(app_registry|context|layer|simulation|test_support|transport|utils)\b")
STD_GATED_SQL_ARC_IMPL_PATTERN = re.compile(
    r"#\[cfg\(feature = \"std\"\)\]\s*#\[async_trait\]\s*impl<T: SqlQueryExecutor \+ \?Sized> SqlQueryExecutor for std::sync::Arc<T>",
    re.DOTALL,
)
STD_GATED_SQL_ARC_IMPL_AWAIT_PATTERN = re.compile(
    r"#\[cfg\(feature = \"std\"\)\]\s*#\[async_trait\]\s*impl<T: SqlQueryExecutor \+ \?Sized> SqlQueryExecutor for std::sync::Arc<T>\s*\{.*?\(\*\*self\)\.execute_sql\(request\)\.await",
    re.DOTALL,
)
STD_GATED_SQL_AWAIT_CALL_PATTERN = re.compile(r"\(\*\*self\)\.execute_sql\(request\)\.await")

EXPECTED_PUBLIC_MODULES = frozenset(
    {
        ("app_registry", STD_GUARD),
        ("circuit_breaker", GUARD_NONE),
        ("cluster", GUARD_NONE),
        ("constants", GUARD_NONE),
        ("context", STD_GUARD),
        ("crypto", GUARD_NONE),
        ("error", GUARD_NONE),
        ("hlc", GUARD_NONE),
        ("kv", GUARD_NONE),
        ("layer", LAYER_GUARD),
        ("prelude", GUARD_NONE),
        ("protocol", GUARD_NONE),
        ("simulation", STD_GUARD),
        ("spec", GUARD_NONE),
        ("sql", SQL_GUARD),
        ("storage", GUARD_NONE),
        ("traits", GUARD_NONE),
        ("transport", STD_GUARD),
        ("types", GUARD_NONE),
        ("utils", STD_GUARD),
        ("vault", GUARD_NONE),
        ("verified", GUARD_NONE),
    }
)

EXPECTED_ROOT_REEXPORTS = frozenset(
    {
        ("app_registry::AppManifest", STD_GUARD),
        ("app_registry::AppRegistry", STD_GUARD),
        ("app_registry::SharedAppRegistry", STD_GUARD),
        ("app_registry::shared_registry", STD_GUARD),
        ("cluster::AddLearnerRequest", GUARD_NONE),
        ("cluster::ChangeMembershipRequest", GUARD_NONE),
        ("cluster::ClusterNode", GUARD_NONE),
        ("cluster::ClusterState", GUARD_NONE),
        ("cluster::InitRequest", GUARD_NONE),
        ("cluster::TrustConfig", GUARD_NONE),
        ("constants::api::*", GUARD_NONE),
        ("constants::ci::*", GUARD_NONE),
        ("constants::coordination::*", GUARD_NONE),
        ("constants::directory::*", GUARD_NONE),
        ("constants::network::*", GUARD_NONE),
        ("constants::raft::*", GUARD_NONE),
        ("constants::raft_compat::MEMBERSHIP_COOLDOWN", STD_GUARD),
        ("context::AspenDocsTicket", STD_GUARD),
        ("context::ContentDiscovery", GLOBAL_DISCOVERY_GUARD),
        ("context::ContentNodeAddr", GLOBAL_DISCOVERY_GUARD),
        ("context::ContentProviderInfo", GLOBAL_DISCOVERY_GUARD),
        ("context::DocsEntry", STD_GUARD),
        ("context::DocsStatus", STD_GUARD),
        ("context::DocsSyncProvider", STD_GUARD),
        ("context::EndpointProvider", STD_GUARD),
        ("context::InMemoryWatchRegistry", STD_GUARD),
        ("context::KeyOrigin", STD_GUARD),
        ("context::NetworkFactory", STD_GUARD),
        ("context::PeerConnectionState", STD_GUARD),
        ("context::PeerImporter", STD_GUARD),
        ("context::PeerInfo", STD_GUARD),
        ("context::PeerManager", STD_GUARD),
        ("context::ServiceExecutor", STD_GUARD),
        ("context::ShardTopology", STD_GUARD),
        ("context::StateMachineProvider", STD_GUARD),
        ("context::SubscriptionFilter", STD_GUARD),
        ("context::SyncStatus", STD_GUARD),
        ("context::WatchInfo", STD_GUARD),
        ("context::WatchRegistry", STD_GUARD),
        ("crypto::Signature", GUARD_NONE),
        ("error::ControlPlaneError", GUARD_NONE),
        ("error::KeyValueStoreError", GUARD_NONE),
        ("hlc::HLC", GUARD_NONE),
        ("hlc::HlcTimestamp", GUARD_NONE),
        ('hlc::ID as HlcId', GUARD_NONE),
        ("hlc::NTP64", GUARD_NONE),
        ("hlc::SerializableTimestamp", GUARD_NONE),
        ("hlc::create_hlc", GUARD_NONE),
        ("hlc::new_timestamp", GUARD_NONE),
        ("hlc::to_unix_ms", GUARD_NONE),
        ("hlc::update_from_timestamp", GUARD_NONE),
        ("kv::BatchCondition", GUARD_NONE),
        ("kv::BatchOperation", GUARD_NONE),
        ("kv::CompareOp", GUARD_NONE),
        ("kv::CompareTarget", GUARD_NONE),
        ("kv::DeleteRequest", GUARD_NONE),
        ("kv::DeleteResult", GUARD_NONE),
        ("kv::KeyValueWithRevision", GUARD_NONE),
        ("kv::ReadConsistency", GUARD_NONE),
        ("kv::ReadRequest", GUARD_NONE),
        ("kv::ReadResult", GUARD_NONE),
        ("kv::ScanRequest", GUARD_NONE),
        ("kv::ScanResult", GUARD_NONE),
        ("kv::TxnCompare", GUARD_NONE),
        ("kv::TxnOp", GUARD_NONE),
        ("kv::TxnOpResult", GUARD_NONE),
        ("kv::WriteCommand", GUARD_NONE),
        ("kv::WriteOp", GUARD_NONE),
        ("kv::WriteRequest", GUARD_NONE),
        ("kv::WriteResult", GUARD_NONE),
        ("kv::validate_write_command", GUARD_NONE),
        ("layer::AllocationError", LAYER_GUARD),
        ("layer::DirectoryError", LAYER_GUARD),
        ("layer::DirectoryLayer", LAYER_GUARD),
        ("layer::DirectorySubspace", LAYER_GUARD),
        ("layer::Element", LAYER_GUARD),
        ("layer::HighContentionAllocator", LAYER_GUARD),
        ("layer::Subspace", LAYER_GUARD),
        ("layer::SubspaceError", LAYER_GUARD),
        ("layer::Tuple", LAYER_GUARD),
        ("layer::TupleError", LAYER_GUARD),
        ("protocol::Alarm", GUARD_NONE),
        ("protocol::Envelope", GUARD_NONE),
        ("protocol::ProtocolCtx", GUARD_NONE),
        ("protocol::TestCtx", GUARD_NONE),
        ("simulation::SimulationArtifact", STD_GUARD),
        ("simulation::SimulationArtifactBuilder", STD_GUARD),
        ("simulation::SimulationStatus", STD_GUARD),
        ("sql::SqlColumnInfo", SQL_GUARD),
        ("sql::SqlConsistency", SQL_GUARD),
        ("sql::SqlQueryError", SQL_GUARD),
        ("sql::SqlQueryExecutor", SQL_GUARD),
        ("sql::SqlQueryRequest", SQL_GUARD),
        ("sql::SqlQueryResult", SQL_GUARD),
        ("sql::SqlValue", SQL_GUARD),
        ("sql::effective_sql_limit", SQL_GUARD),
        ("sql::effective_sql_timeout_ms", SQL_GUARD),
        ("sql::validate_sql_query", SQL_GUARD),
        ("sql::validate_sql_request", SQL_GUARD),
        ("storage::KvEntry", GUARD_NONE),
        ("storage::SM_KV_TABLE", GUARD_NONE),
        ("traits::ClusterController", GUARD_NONE),
        ("traits::CoordinationBackend", GUARD_NONE),
        ("traits::KeyValueStore", GUARD_NONE),
        ("transport::BlobAnnouncedCallback", STD_GUARD),
        ("transport::BlobAnnouncedInfo", STD_GUARD),
        ("transport::DiscoveredPeer", STD_GUARD),
        ("transport::DiscoveryHandle", STD_GUARD),
        ("transport::IrohTransportExt", STD_GUARD),
        ("transport::MembershipAddressUpdater", STD_GUARD),
        ("transport::NetworkTransport", STD_GUARD),
        ("transport::PeerDiscoveredCallback", STD_GUARD),
        ("transport::PeerDiscovery", STD_GUARD),
        ("transport::StaleTopologyInfo", STD_GUARD),
        ("transport::TopologyStaleCallback", STD_GUARD),
        ("types::ClusterMetrics", GUARD_NONE),
        ("types::NodeAddress", GUARD_NONE),
        ("types::NodeId", GUARD_NONE),
        ("types::NodeState", GUARD_NONE),
        ("types::SnapshotLogId", GUARD_NONE),
        ("utils::ensure_disk_space_available", STD_GUARD),
        ("vault::SYSTEM_PREFIX", GUARD_NONE),
        ("vault::VaultError", GUARD_NONE),
        ("vault::is_system_key", GUARD_NONE),
        ("vault::validate_client_key", GUARD_NONE),
        ("verified::build_scan_metadata", GUARD_NONE),
        ("verified::decode_continuation_token", GUARD_NONE),
        ("verified::encode_continuation_token", GUARD_NONE),
        ("verified::execute_scan", GUARD_NONE),
        ("verified::filter_scan_entries", GUARD_NONE),
        ("verified::normalize_scan_limit", GUARD_NONE),
        ("verified::paginate_entries", GUARD_NONE),
    }
)

EXPECTED_VERIFIED_MODULES = frozenset({("scan", GUARD_NONE)})
EXPECTED_VERIFIED_REEXPORTS = frozenset(
    {
        ("scan::build_scan_metadata", GUARD_NONE),
        ("scan::decode_continuation_token", GUARD_NONE),
        ("scan::encode_continuation_token", GUARD_NONE),
        ("scan::execute_scan", GUARD_NONE),
        ("scan::filter_scan_entries", GUARD_NONE),
        ("scan::normalize_scan_limit", GUARD_NONE),
        ("scan::paginate_entries", GUARD_NONE),
        ("crate::hlc::SerializableTimestamp", GUARD_NONE),
        ("crate::hlc::create_hlc", GUARD_NONE),
        ("crate::hlc::to_unix_ms", GUARD_NONE),
        ("crate::kv::validate_write_command", GUARD_NONE),
        ("crate::types::NodeId", GUARD_NONE),
        ("crate::types::NodeState", GUARD_NONE),
    }
)
EXPECTED_SPEC_MODULES = frozenset({("verus_shim", GUARD_NONE)})
EXPECTED_SPEC_REEXPORTS = frozenset({("verus_shim::*", GUARD_NONE)})
EXPECTED_SCAN_FUNCTIONS = frozenset(
    {
        "build_scan_metadata",
        "decode_continuation_token",
        "encode_continuation_token",
        "execute_scan",
        "filter_scan_entries",
        "normalize_scan_limit",
        "paginate_entries",
    }
)
EXPECTED_PRELUDE_REEXPORTS = frozenset(
    {
        ("crate::cluster::ClusterNode", GUARD_NONE),
        ("crate::cluster::ClusterState", GUARD_NONE),
        ("crate::constants::api::DEFAULT_SCAN_LIMIT", GUARD_NONE),
        ("crate::constants::api::MAX_KEY_SIZE", GUARD_NONE),
        ("crate::constants::api::MAX_SCAN_RESULTS", GUARD_NONE),
        ("crate::constants::api::MAX_VALUE_SIZE", GUARD_NONE),
        ("crate::error::ControlPlaneError", GUARD_NONE),
        ("crate::error::KeyValueStoreError", GUARD_NONE),
        ("crate::kv::DeleteRequest", GUARD_NONE),
        ("crate::kv::DeleteResult", GUARD_NONE),
        ("crate::kv::KeyValueWithRevision", GUARD_NONE),
        ("crate::kv::ReadRequest", GUARD_NONE),
        ("crate::kv::ReadResult", GUARD_NONE),
        ("crate::kv::ScanRequest", GUARD_NONE),
        ("crate::kv::ScanResult", GUARD_NONE),
        ("crate::kv::WriteCommand", GUARD_NONE),
        ("crate::kv::WriteRequest", GUARD_NONE),
        ("crate::kv::WriteResult", GUARD_NONE),
        ("crate::traits::ClusterController", GUARD_NONE),
        ("crate::traits::CoordinationBackend", GUARD_NONE),
        ("crate::traits::KeyValueStore", GUARD_NONE),
        ("crate::types::ClusterMetrics", GUARD_NONE),
        ("crate::types::NodeAddress", GUARD_NONE),
        ("crate::types::NodeId", GUARD_NONE),
        ("crate::types::NodeState", GUARD_NONE),
    }
)

EXPORT_GROUP_ORDER = (
    ("Alloc-only root exports", GUARD_NONE),
    ("Std-gated root exports", STD_GUARD),
    ("Global-discovery root exports", GLOBAL_DISCOVERY_GUARD),
    ("Layer root exports", LAYER_GUARD),
    ("SQL root exports", SQL_GUARD),
)

ALLOC_ONLY_MODULE_PATH_FAMILIES = (
    "aspen_core::circuit_breaker::*",
    "aspen_core::prelude::*",
    "aspen_core::spec::*",
    "aspen_core::verified::scan::{build_scan_metadata, decode_continuation_token, encode_continuation_token, execute_scan, filter_scan_entries, normalize_scan_limit, paginate_entries}",
    "aspen_core::verified::{SerializableTimestamp, create_hlc, to_unix_ms, validate_write_command, NodeId, NodeState}",
)

ALLOC_ONLY_AUDIT_FILES = (
    Path("circuit_breaker.rs"),
    Path("cluster.rs"),
    Path("constants/mod.rs"),
    Path("crypto.rs"),
    Path("error.rs"),
    Path("hlc.rs"),
    Path("kv/mod.rs"),
    Path("prelude.rs"),
    Path("protocol.rs"),
    Path("spec/mod.rs"),
    Path("spec/verus_shim.rs"),
    Path("sql.rs"),
    Path("storage.rs"),
    Path("traits.rs"),
    Path("types.rs"),
    Path("vault.rs"),
    Path("verified/mod.rs"),
    Path("verified/scan.rs"),
)

FORBIDDEN_TOKEN_PATTERNS = {
    "ambient environment reads": re.compile(r"\bstd::env::"),
    "process access": re.compile(r"\bstd::process::"),
    "filesystem I/O": re.compile(r"\bstd::fs::|\bstd::io::|\bfs::(read|read_to_string|write|create_dir_all|remove_dir_all)\b"),
    "runtime shell crates": re.compile(r"\banyhow::|\biroh(?:_blobs)?::|\btokio(?:_util)?::|\bn0_future::"),
    "implicit randomness": re.compile(r"\brand::(?:rng|thread_rng|random)\b|\bgetrandom\b"),
    "process-global state": re.compile(r"\bOnceLock\b|\bOnceCell\b|\blazy_static!\b|\bthread_local!\b|static\s+mut\b"),
}

AWAIT_PATTERN = re.compile(r"\.await\b")
ASYNC_FUNCTION_PATTERN = re.compile(r"\basync\s+fn\b")
PRELUDE_STD_GUARD_PATTERN = re.compile(r'feature = "std"')


@dataclass(frozen=True)
class ModuleEntry:
    visibility: str
    name: str
    guard: str


@dataclass(frozen=True)
class ReexportEntry:
    visibility: str
    source: str
    exported_name: str
    guard: str


@dataclass(frozen=True)
class SurfaceInventory:
    file_path: Path
    modules: tuple[ModuleEntry, ...]
    reexports: tuple[ReexportEntry, ...]


class SurfaceError(RuntimeError):
    pass


def normalized_line(raw_line: str) -> str:
    return raw_line.split("//", maxsplit=1)[0].strip()


def joined_guard(pending_attributes: list[str]) -> str:
    if not pending_attributes:
        return GUARD_NONE
    return " and ".join(pending_attributes)


def exported_name_for_source(source: str) -> str:
    if source.endswith(GLOB_SUFFIX):
        return "*"
    alias_parts = ALIAS_SPLIT_PATTERN.split(source, maxsplit=1)
    if len(alias_parts) == 2:
        return alias_parts[1].strip()
    return source.rsplit("::", maxsplit=1)[-1].strip()


def parse_surface(file_path: Path) -> SurfaceInventory:
    if not file_path.is_file():
        raise SurfaceError(f"missing {file_path}")

    modules: list[ModuleEntry] = []
    reexports: list[ReexportEntry] = []
    pending_attributes: list[str] = []

    for raw_line in file_path.read_text().splitlines():
        line = normalized_line(raw_line)
        if line == EMPTY_TEXT:
            continue
        if line.startswith(CFG_PREFIX):
            pending_attributes.append(line)
            continue

        module_match = MODULE_PATTERN.match(line)
        if module_match:
            visibility, name = module_match.groups()
            modules.append(ModuleEntry(visibility=visibility, name=name, guard=joined_guard(pending_attributes)))
            pending_attributes.clear()
            continue

        reexport_match = REEXPORT_PATTERN.match(line)
        if reexport_match:
            visibility, source = reexport_match.groups()
            reexports.append(
                ReexportEntry(
                    visibility=visibility,
                    source=source.strip(),
                    exported_name=exported_name_for_source(source.strip()),
                    guard=joined_guard(pending_attributes),
                )
            )
            pending_attributes.clear()
            continue

        if not line.startswith("#"):
            pending_attributes.clear()

    return SurfaceInventory(file_path=file_path, modules=tuple(modules), reexports=tuple(reexports))


def public_functions_in_file(file_path: Path) -> frozenset[str]:
    source = production_source(file_path.read_text())
    return frozenset(
        match.group(1)
        for match in PUBLIC_FUNCTION_PATTERN.finditer(source)
    )


def public_modules(surface: SurfaceInventory) -> frozenset[tuple[str, str]]:
    return frozenset((entry.name, entry.guard) for entry in surface.modules if entry.visibility == PUBLIC_VISIBILITY)


def public_reexports(surface: SurfaceInventory) -> frozenset[tuple[str, str]]:
    return frozenset((entry.source, entry.guard) for entry in surface.reexports if entry.visibility == PUBLIC_VISIBILITY)


def crate_private_module(surface: SurfaceInventory, name: str) -> ModuleEntry | None:
    for entry in surface.modules:
        if entry.name == name:
            return entry
    return None


def diff_lines(actual: frozenset[tuple[str, str]], expected: frozenset[tuple[str, str]], kind: str) -> list[str]:
    lines: list[str] = []
    unexpected = sorted(actual - expected)
    missing = sorted(expected - actual)
    for item, guard in missing:
        lines.append(f"missing {kind}: `{item}` guarded by `{guard}`")
    for item, guard in unexpected:
        lines.append(f"unexpected {kind}: `{item}` guarded by `{guard}`")
    return lines


def validate_lib_surface(surface: SurfaceInventory) -> list[str]:
    errors = diff_lines(public_modules(surface), EXPECTED_PUBLIC_MODULES, "public module")
    errors.extend(diff_lines(public_reexports(surface), EXPECTED_ROOT_REEXPORTS, "root re-export"))

    test_support = crate_private_module(surface, "test_support")
    if test_support is None:
        errors.append("missing crate-private test_support module declaration")
    elif test_support.visibility != CRATE_VISIBILITY or test_support.guard != TEST_STD_GUARD:
        errors.append(
            "test_support module must stay `pub(crate)` behind `#[cfg(all(test, feature = \"std\"))]`"
        )

    if any(entry.source.startswith("test_support::") or "::test_support::" in entry.source for entry in surface.reexports):
        errors.append("public root re-exports must not expose `aspen_core::test_support`")

    return errors


def validate_verified_surface(surface: SurfaceInventory, scan_functions: frozenset[str]) -> list[str]:
    errors = diff_lines(public_modules(surface), EXPECTED_VERIFIED_MODULES, "verified module")
    errors.extend(diff_lines(public_reexports(surface), EXPECTED_VERIFIED_REEXPORTS, "verified re-export"))

    missing_scan_functions = sorted(EXPECTED_SCAN_FUNCTIONS - scan_functions)
    unexpected_scan_functions = sorted(scan_functions - EXPECTED_SCAN_FUNCTIONS)
    for name in missing_scan_functions:
        errors.append(f"missing `verified::scan::{name}` public function")
    for name in unexpected_scan_functions:
        errors.append(f"unexpected `verified::scan::{name}` public function")

    return errors


def validate_spec_surface(surface: SurfaceInventory) -> list[str]:
    errors = diff_lines(public_modules(surface), EXPECTED_SPEC_MODULES, "spec module")
    errors.extend(diff_lines(public_reexports(surface), EXPECTED_SPEC_REEXPORTS, "spec re-export"))
    return errors


def validate_prelude_surface(surface: SurfaceInventory) -> list[str]:
    errors = diff_lines(public_reexports(surface), EXPECTED_PRELUDE_REEXPORTS, "prelude re-export")
    for entry in surface.reexports:
        if entry.visibility != PUBLIC_VISIBILITY:
            continue
        if entry.guard == GUARD_NONE:
            continue
        if PRELUDE_STD_GUARD_PATTERN.search(entry.guard) is None:
            errors.append(f"prelude re-export `{entry.source}` must be alloc-safe or std-gated")
    return errors


def module_table_rows(entries: Iterable[ModuleEntry]) -> list[str]:
    rows = ["| Module | Guard |", "| --- | --- |"]
    for entry in entries:
        if entry.visibility != PUBLIC_VISIBILITY:
            continue
        rows.append(f"| `{entry.name}` | `{entry.guard}` |")
    return rows


def reexport_table_rows(entries: Iterable[ReexportEntry]) -> list[str]:
    rows = ["| Export | Source | Guard |", "| --- | --- | --- |"]
    for entry in entries:
        if entry.visibility != PUBLIC_VISIBILITY:
            continue
        rows.append(f"| `{entry.exported_name}` | `{entry.source}` | `{entry.guard}` |")
    return rows


def markdown_for_inventory(crate_dir: Path, lib_surface: SurfaceInventory, notes: list[str]) -> str:
    public_module_count = sum(1 for entry in lib_surface.modules if entry.visibility == PUBLIC_VISIBILITY)
    public_reexport_count = sum(1 for entry in lib_surface.reexports if entry.visibility == PUBLIC_VISIBILITY)
    sections = [
        "# Aspen Core Surface Inventory",
        EMPTY_TEXT,
        f"- Crate dir: `{crate_dir}`",
        f"- Source: `{lib_surface.file_path}`",
        f"- Public modules: `{public_module_count}`",
        f"- Root re-exports: `{public_reexport_count}`",
        EMPTY_TEXT,
        "## Public modules",
        *module_table_rows(lib_surface.modules),
        EMPTY_TEXT,
        "## Root re-exports",
        *reexport_table_rows(lib_surface.reexports),
        EMPTY_TEXT,
        "## Validation notes",
        *[f"- {note}" for note in notes],
        EMPTY_TEXT,
    ]
    return "\n".join(sections)


def grouped_reexports(surface: SurfaceInventory, guard: str) -> list[str]:
    rows = []
    for entry in surface.reexports:
        if entry.visibility != PUBLIC_VISIBILITY or entry.guard != guard:
            continue
        rows.append(f"- `{entry.source}`")
    return rows


def export_map_markdown(lib_surface: SurfaceInventory) -> str:
    sections = [
        "# Aspen Core Export Map",
        EMPTY_TEXT,
        f"- Source: `{lib_surface.file_path}`",
        "- Status: validated against the OpenSpec no-std export contract.",
        EMPTY_TEXT,
    ]
    for title, guard in EXPORT_GROUP_ORDER:
        sections.append(f"## {title}")
        sections.extend(grouped_reexports(lib_surface, guard))
        sections.append(EMPTY_TEXT)

    sections.extend(
        [
            "## Alloc-only module-path families",
            *[f"- `{family}`" for family in ALLOC_ONLY_MODULE_PATH_FAMILIES],
            EMPTY_TEXT,
            "## Test-only shell infrastructure",
            f"- `test_support` stays crate-private behind `{TEST_STD_GUARD}` and has no public root path.",
            EMPTY_TEXT,
        ]
    )
    return "\n".join(sections)


def production_source(raw_text: str) -> str:
    lines: list[str] = []
    for raw_line in raw_text.splitlines():
        if TEST_CFG_PATTERN.match(raw_line.strip()):
            break
        lines.append(raw_line)
    return "\n".join(lines)


def strip_comments_and_strings(source: str) -> str:
    result: list[str] = []
    index = 0
    length = len(source)
    in_string = False
    in_char = False
    string_delimiter = EMPTY_TEXT
    while index < length:
        current = source[index]
        next_char = source[index + 1] if index + 1 < length else EMPTY_TEXT

        if in_string:
            if current == "\\" and index + 1 < length:
                result.append(" ")
                result.append(" ")
                index += 2
                continue
            if current == string_delimiter:
                in_string = False
            result.append("\n" if current == "\n" else " ")
            index += 1
            continue

        if in_char:
            if current == "\\" and index + 1 < length:
                result.append(" ")
                result.append(" ")
                index += 2
                continue
            if current == "'":
                in_char = False
            result.append("\n" if current == "\n" else " ")
            index += 1
            continue

        if current == "/" and next_char == "/":
            while index < length and source[index] != "\n":
                result.append(" ")
                index += 1
            continue

        if current == "/" and next_char == "*":
            result.append(" ")
            result.append(" ")
            index += 2
            while index < length:
                block_current = source[index]
                block_next = source[index + 1] if index + 1 < length else EMPTY_TEXT
                if block_current == "*" and block_next == "/":
                    result.append(" ")
                    result.append(" ")
                    index += 2
                    break
                result.append("\n" if block_current == "\n" else " ")
                index += 1
            continue

        if current in {'"', "'"}:
            if current == '"':
                in_string = True
                string_delimiter = current
            else:
                in_char = True
            result.append(" ")
            index += 1
            continue

        result.append(current)
        index += 1

    return "".join(result)


@dataclass(frozen=True)
class AuditResult:
    relative_path: Path
    checked_categories: tuple[str, ...]


def ensure_no_forbidden_tokens(relative_path: Path, stripped_source: str) -> list[str]:
    errors: list[str] = []
    for label, pattern in FORBIDDEN_TOKEN_PATTERNS.items():
        if pattern.search(stripped_source) is not None:
            errors.append(f"`{relative_path}` contains forbidden {label}")
    if SHELL_BACKEDGE_PATTERN.search(stripped_source) is not None:
        errors.append(f"`{relative_path}` reaches shell-only modules from alloc-only source")
    return errors


def validate_sql_source(sql_source: str) -> list[str]:
    errors = []
    stripped_sql_source = strip_comments_and_strings(sql_source)
    non_std_sql_source = STD_GATED_SQL_AWAIT_CALL_PATTERN.sub(EMPTY_TEXT, stripped_sql_source)
    for label, pattern in FORBIDDEN_TOKEN_PATTERNS.items():
        if pattern.search(non_std_sql_source) is not None:
            errors.append(f"`sql.rs` contains forbidden {label} outside std-gated shell conveniences")
    if SHELL_BACKEDGE_PATTERN.search(non_std_sql_source) is not None:
        errors.append("`sql.rs` reaches shell-only modules from alloc-safe code")
    if STD_GATED_SQL_ARC_IMPL_PATTERN.search(sql_source) is None:
        errors.append("`sql.rs` must keep the Arc<T> convenience impl behind `#[cfg(feature = \"std\")]`")
    if STD_GATED_SQL_ARC_IMPL_AWAIT_PATTERN.search(sql_source) is None:
        errors.append("`sql.rs` std-gated Arc<T> convenience impl must remain the only runtime-bound async body")
    if AWAIT_PATTERN.search(non_std_sql_source) is not None:
        errors.append("`sql.rs` alloc-safe surface must not contain runtime-bound async bodies outside the std-gated Arc impl")
    return errors


def validate_async_bodies(relative_path: Path, stripped_source: str) -> list[str]:
    if AWAIT_PATTERN.search(stripped_source) is not None:
        return [f"`{relative_path}` contains runtime-bound `.await` in alloc-only production code"]
    if ASYNC_FUNCTION_PATTERN.search(stripped_source) is not None:
        return [f"`{relative_path}` contains `async fn` in alloc-only production code"]
    return []


def source_audit(crate_dir: Path) -> tuple[list[str], str]:
    errors: list[str] = []
    pass_lines: list[str] = []

    for relative_path in ALLOC_ONLY_AUDIT_FILES:
        absolute_path = crate_dir / relative_path
        raw_source = production_source(absolute_path.read_text())
        stripped_source = strip_comments_and_strings(raw_source)

        if relative_path == Path("sql.rs"):
            sql_errors = validate_sql_source(raw_source)
            errors.extend(sql_errors)
            if not sql_errors:
                pass_lines.append(
                    "PASS sql.rs keeps alloc-safe validation/query types pure and gates the Arc<T> convenience impl behind `feature = \"std\"`."
                )
            continue

        file_errors = ensure_no_forbidden_tokens(relative_path, stripped_source)
        file_errors.extend(validate_async_bodies(relative_path, stripped_source))
        errors.extend(file_errors)
        if not file_errors:
            pass_lines.append(f"PASS `{relative_path}` has no shell-module backedges, forbidden runtime tokens, or runtime-bound async bodies.")

    audit_lines = [
        "# Aspen Core Source Audit",
        EMPTY_TEXT,
        "Audit scope: alloc-only production modules and helper families documented for the no-std surface.",
        "Pure contract traits may declare async signatures, but alloc-only production code must not contain runtime-bound async bodies or `.await` paths.",
        EMPTY_TEXT,
        *pass_lines,
        EMPTY_TEXT,
    ]
    return errors, "\n".join(audit_lines)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--crate-dir", required=True, type=Path, help="Path to `crates/aspen-core/src`")
    parser.add_argument("--output-dir", required=True, type=Path, help="Directory for generated inventory artifacts")
    return parser.parse_args(argv)


def write_outputs(output_dir: Path, surface_inventory: str, export_map: str, source_audit_text: str) -> tuple[Path, Path, Path]:
    output_dir.mkdir(parents=True, exist_ok=True)
    surface_path = output_dir / SURFACE_OUTPUT_FILE_NAME
    export_map_path = output_dir / EXPORT_MAP_FILE_NAME
    source_audit_path = output_dir / SOURCE_AUDIT_FILE_NAME
    surface_path.write_text(surface_inventory)
    export_map_path.write_text(export_map)
    source_audit_path.write_text(source_audit_text)
    return surface_path, export_map_path, source_audit_path


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    crate_dir = args.crate_dir

    try:
        lib_surface = parse_surface(crate_dir / LIB_RS_NAME)
        verified_surface = parse_surface(crate_dir / VERIFIED_MOD_RS_PATH)
        spec_surface = parse_surface(crate_dir / SPEC_MOD_RS_PATH)
        prelude_surface = parse_surface(crate_dir / PRELUDE_RS_NAME)
    except SurfaceError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    validation_errors: list[str] = []
    validation_errors.extend(validate_lib_surface(lib_surface))
    validation_errors.extend(validate_verified_surface(verified_surface, public_functions_in_file(crate_dir / VERIFIED_SCAN_RS_PATH)))
    validation_errors.extend(validate_spec_surface(spec_surface))
    validation_errors.extend(validate_prelude_surface(prelude_surface))
    source_errors, source_audit_text = source_audit(crate_dir)
    validation_errors.extend(source_errors)

    if validation_errors:
        for error in validation_errors:
            print(f"error: {error}", file=sys.stderr)
        return 1

    surface_inventory = markdown_for_inventory(
        crate_dir=crate_dir,
        lib_surface=lib_surface,
        notes=[
            "The root public module and re-export inventory matches the documented no-std contract.",
            "`verified::*` is limited to scan helpers plus core pure re-exports from `hlc`, `kv`, and `types`.",
            "`test_support` remains crate-private and test-only; there is no public `aspen_core::test_support` path.",
            "Alloc-only module-path families remain present for `circuit_breaker`, `prelude`, `spec`, and `verified` scan helpers.",
        ],
    )
    export_map = export_map_markdown(lib_surface)
    surface_path, export_map_path, source_audit_path = write_outputs(args.output_dir, surface_inventory, export_map, source_audit_text)

    print(surface_path)
    print(export_map_path)
    print(source_audit_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
