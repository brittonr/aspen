#!/usr/bin/env python3
"""Generate typed Nickel contracts from selected Rust-owned DTOs.

This intentionally covers only registry-approved operator evidence/protocol DTO
families. It is a freshness gate, not a general-purpose Rust parser.
"""

from __future__ import annotations

import argparse
import dataclasses
import difflib
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


@dataclasses.dataclass(frozen=True)
class ContractSet:
    id: str
    title: str
    source_path: Path
    output_path: Path
    types: tuple[str, ...]


CONTRACT_SETS = (
    ContractSet(
        id="dogfood-run-receipt",
        title="Dogfood run receipt",
        source_path=Path("crates/aspen-dogfood/src/receipt.rs"),
        output_path=Path("schemas/dogfood-run-receipt.ncl"),
        types=(
            "DogfoodRunMode",
            "DogfoodStageKind",
            "DogfoodStageStatus",
            "DogfoodFailureSummary",
            "DogfoodArtifactKind",
            "DogfoodArtifactReceipt",
            "DogfoodStageReceipt",
            "DogfoodRunReceipt",
        ),
    ),
    ContractSet(
        id="native-ci-run-receipt",
        title="Native CI run receipt",
        source_path=Path("crates/aspen-client-api/src/messages/ci.rs"),
        output_path=Path("schemas/ci-run-receipt.ncl"),
        types=(
            "CiArtifactInfo",
            "CiRunReceiptJob",
            "CiRunReceiptStage",
            "CiRunReceipt",
        ),
    ),
    ContractSet(
        id="sponsored-runtime-grant",
        title="Sponsored runtime grant",
        source_path=Path("crates/aspen-runtime-core/src/lib.rs"),
        output_path=Path("schemas/sponsored-runtime-grant.ncl"),
        types=(
            "SponsoredPrincipalRole",
            "SponsoredPrincipalRef",
            "RedactedValue",
            "SponsoredSettlementReference",
            "SponsoredResourceLimits",
            "RuntimeHostKind",
            "SponsoredGrantScope",
            "SponsoredRevocationRef",
            "SponsoredRuntimeGrant",
        ),
    ),
    ContractSet(
        id="sponsored-quota-ledger",
        title="Sponsored quota ledger",
        source_path=Path("crates/aspen-runtime-core/src/lib.rs"),
        output_path=Path("schemas/sponsored-quota-ledger.ncl"),
        types=(
            "SponsoredResourceLimits",
            "SponsoredQuotaReservation",
            "SponsoredQuotaConsumption",
            "SponsoredQuotaLedger",
        ),
    ),
    ContractSet(
        id="sponsored-usage-receipt",
        title="Sponsored usage receipt",
        source_path=Path("crates/aspen-runtime-core/src/lib.rs"),
        output_path=Path("schemas/sponsored-usage-receipt.ncl"),
        types=(
            "RedactedValue",
            "RuntimeDiagnostic",
            "SponsoredSettlementReference",
            "SponsoredResourceLimits",
            "SponsoredReceiptOutcome",
            "SponsoredUsageReceipt",
            "SignedSponsoredUsageReceipt",
        ),
    ),
)

PRIMITIVES = {
    "String": "String",
    "str": "String",
    "bool": "Bool",
    "u8": "NonNegativeNumber",
    "u16": "NonNegativeNumber",
    "u32": "NonNegativeNumber",
    "u64": "NonNegativeNumber",
    "usize": "NonNegativeNumber",
    "i8": "Number",
    "i16": "Number",
    "i32": "Number",
    "i64": "Number",
    "isize": "Number",
}


@dataclasses.dataclass(frozen=True)
class StructDef:
    name: str
    fields: tuple[tuple[str, str], ...]
    derives: tuple[str, ...]


@dataclasses.dataclass(frozen=True)
class EnumDef:
    name: str
    variants: tuple[str, ...]
    derives: tuple[str, ...]
    rename_all: str | None


RustDef = StructDef | EnumDef


def rename_variant(name: str, rename_all: str | None) -> str:
    if rename_all == "snake_case":
        return snake_case(name)
    if rename_all == "kebab-case":
        return snake_case(name).replace("_", "-")
    return name


def snake_case(name: str) -> str:
    out: list[str] = []
    for i, char in enumerate(name):
        if char.isupper() and i > 0 and (not name[i - 1].isupper() or (i + 1 < len(name) and name[i + 1].islower())):
            out.append("_")
        out.append(char.lower())
    return "".join(out)


def split_top_level_csv(text: str) -> list[str]:
    parts: list[str] = []
    depth = 0
    start = 0
    for i, char in enumerate(text):
        if char == "<":
            depth += 1
        elif char == ">":
            depth -= 1
        elif char == "," and depth == 0:
            parts.append(text[start:i].strip())
            start = i + 1
    tail = text[start:].strip()
    if tail:
        parts.append(tail)
    return parts


def inner_generic(ty: str, prefix: str) -> str | None:
    marker = prefix + "<"
    if not ty.startswith(marker) or not ty.endswith(">"):
        return None
    return ty[len(marker) : -1].strip()


def parse_rust_defs(source: str) -> dict[str, RustDef]:
    defs: dict[str, RustDef] = {}
    lines = source.splitlines()
    pending_derives: list[str] = []
    pending_rename_all: str | None = None
    i = 0
    while i < len(lines):
        stripped = lines[i].strip()
        if stripped.startswith("#[derive("):
            pending_derives = [item.strip() for item in stripped.removeprefix("#[derive(").removesuffix(")] ").removesuffix(")]").split(",")]
            i += 1
            continue
        rename_match = re.match(r'#\[serde\([^\]]*rename_all = "([^"]+)"', stripped)
        if rename_match:
            pending_rename_all = rename_match.group(1)
            i += 1
            continue

        struct_match = re.match(r"pub struct (\w+) \{", stripped)
        if struct_match:
            name = struct_match.group(1)
            fields: list[tuple[str, str]] = []
            i += 1
            while i < len(lines) and lines[i].strip() != "}":
                field = lines[i].strip()
                field_match = re.match(r"pub (\w+): (.+),$", field)
                if field_match:
                    fields.append((field_match.group(1), field_match.group(2).strip()))
                i += 1
            defs[name] = StructDef(name=name, fields=tuple(fields), derives=tuple(pending_derives))
            pending_derives = []
            pending_rename_all = None
            i += 1
            continue

        enum_match = re.match(r"pub enum (\w+) \{", stripped)
        if enum_match:
            name = enum_match.group(1)
            variants: list[str] = []
            i += 1
            while i < len(lines) and lines[i].strip() != "}":
                variant = lines[i].strip().rstrip(",")
                if variant and not variant.startswith("#") and re.match(r"^[A-Z]\w*$", variant):
                    variants.append(variant)
                i += 1
            defs[name] = EnumDef(
                name=name,
                variants=tuple(variants),
                derives=tuple(pending_derives),
                rename_all=pending_rename_all,
            )
            pending_derives = []
            pending_rename_all = None
            i += 1
            continue
        i += 1
    return defs


def assert_serde_backed(contract_set: ContractSet, definition: RustDef) -> None:
    derives = set(definition.derives)
    missing = {"Serialize", "Deserialize"} - derives
    if missing:
        raise SystemExit(
            f"{contract_set.source_path}:{definition.name} is missing derive(s) required for a Rust-owned serialized DTO: "
            f"{', '.join(sorted(missing))}"
        )


def nickel_type(ty: str) -> tuple[str, bool]:
    ty = ty.strip()
    optional = False
    option_inner = inner_generic(ty, "Option")
    if option_inner is not None:
        inner, _ = nickel_type(option_inner)
        return inner, True

    vec_inner = inner_generic(ty, "Vec")
    if vec_inner is not None:
        inner, _ = nickel_type(vec_inner)
        return f"Array {inner}", optional

    map_inner = inner_generic(ty, "BTreeMap")
    if map_inner is not None:
        parts = split_top_level_csv(map_inner)
        value_ty = parts[1] if len(parts) == 2 else "Dyn"
        value, _ = nickel_type(value_ty)
        return f"{{ _ : {value} }}", optional

    return PRIMITIVES.get(ty, ty), optional


def render_contract_set(contract_set: ContractSet, all_defs: dict[str, RustDef]) -> str:
    missing = [name for name in contract_set.types if name not in all_defs]
    if missing:
        raise SystemExit(f"{contract_set.source_path}: missing expected type(s): {', '.join(missing)}")

    for name in contract_set.types:
        assert_serde_backed(contract_set, all_defs[name])

    lines = [
        "# Auto-generated Nickel contracts. Do not edit manually.",
        f"# Contract family: {contract_set.id}",
        f"# Source: {contract_set.source_path.as_posix()}",
        "# Regenerate with: python3 scripts/generate-typed-nickel-contracts.py --write",
        "",
        "let contains = fun values value =>",
        "  std.array.fold_left (fun found item => found || item == value) false values in",
        "",
        "let enum_string = fun field allowed =>",
        "  std.contract.from_validator (fun value =>",
        "    if std.is_string value && contains allowed value then",
        "      'Ok",
        "    else",
        "      'Error { message = \"%{field} must be one of %{std.serialize 'Json allowed}\" }",
        "  ) in",
        "",
        "let NonNegativeNumber = std.contract.from_validator (fun value =>",
        "  if std.is_number value && value >= 0 then",
        "    'Ok",
        "  else",
        "    'Error { message = \"number must be non-negative\" }",
        ") in",
        "",
    ]

    for type_name in contract_set.types:
        definition = all_defs[type_name]
        if isinstance(definition, EnumDef):
            values = [rename_variant(variant, definition.rename_all) for variant in definition.variants]
            rendered_values = ", ".join(f'"{value}"' for value in values)
            lines.append(f"let {definition.name} = enum_string \"{definition.name}\" [{rendered_values}] in")
            lines.append("")
        else:
            lines.append(f"let {definition.name} = {{")
            for field_name, field_ty in definition.fields:
                field_contract, optional = nickel_type(field_ty)
                suffix = " | optional" if optional else ""
                lines.append(f"  {field_name} | {field_contract}{suffix},")
            lines.append("} in")
            lines.append("")

    for type_name in contract_set.types:
        lines.append(f"let {type_name}Contract = {type_name} in")
    lines.append("")
    lines.append("{")
    for type_name in contract_set.types:
        lines.append(f"  {type_name} = {type_name}Contract,")
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


def generate_all() -> dict[Path, str]:
    generated: dict[Path, str] = {}
    for contract_set in CONTRACT_SETS:
        source = (ROOT / contract_set.source_path).read_text()
        rust_defs = parse_rust_defs(source)
        generated[contract_set.output_path] = render_contract_set(contract_set, rust_defs)
    return generated


def write_outputs(generated: dict[Path, str]) -> None:
    for path, content in generated.items():
        full_path = ROOT / path
        full_path.parent.mkdir(parents=True, exist_ok=True)
        full_path.write_text(content)
        print(f"wrote {path}")


def check_outputs(generated: dict[Path, str]) -> int:
    failed = False
    for path, expected in generated.items():
        full_path = ROOT / path
        if not full_path.exists():
            print(f"missing generated contract: {path}", file=sys.stderr)
            failed = True
            continue
        actual = full_path.read_text()
        if actual != expected:
            print(f"stale generated contract: {path}", file=sys.stderr)
            diff = difflib.unified_diff(
                actual.splitlines(),
                expected.splitlines(),
                fromfile=f"{path} (current)",
                tofile=f"{path} (expected)",
                lineterm="",
            )
            for line in diff:
                print(line, file=sys.stderr)
            failed = True
    if failed:
        return 1
    print(f"typed Nickel generated contracts fresh: {len(generated)} files")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="overwrite generated contract files")
    parser.add_argument("--check", action="store_true", help="fail if generated contract files are stale")
    args = parser.parse_args()

    generated = generate_all()
    if args.write:
        write_outputs(generated)
        return 0
    if args.check:
        return check_outputs(generated)
    for path, content in generated.items():
        print(f"# === {path} ===")
        print(content)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
