#!/usr/bin/env bash
# Validate/export shared test harness metadata and resolve focused suite selection.
#
# Usage:
#   scripts/test-harness.sh export
#   scripts/test-harness.sh check
#   scripts/test-harness.sh list --layer patchbay
#   scripts/test-harness.sh run --suite multi-node-kv-vm

set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/test-harness.sh export
  scripts/test-harness.sh check
  scripts/test-harness.sh list [--suite ID] [--layer LAYER] [--owner OWNER] [--runtime-class CLASS] [--tag TAG] [--format ids|json|commands]
  scripts/test-harness.sh run  [--suite ID] [--layer LAYER] [--owner OWNER] [--runtime-class CLASS] [--tag TAG]
  scripts/test-harness.sh report [--junit-xml PATH] [--output PATH]
  scripts/test-harness.sh coverage
EOF
}

repo_root=$(unset CDPATH; cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
inventory_path="$repo_root/test-harness/generated/inventory.json"

run_inventory_tool() {
  cargo run -q -p aspen-testing --bin aspen-test-harness -- "$@"
}

ensure_inventory_is_current() {
  run_inventory_tool check >/dev/null
}

jq_selection() {
  local suite_id=$1
  local layer=$2
  local owner=$3
  local runtime_class=$4
  local tag=$5

  jq \
    --arg suite_id "$suite_id" \
    --arg layer "$layer" \
    --arg owner "$owner" \
    --arg runtime_class "$runtime_class" \
    --arg tag "$tag" \
    '
      .suites
      | map(
          select(
            ($suite_id == "" or .id == $suite_id)
            and ($layer == "" or .layer == $layer)
            and ($owner == "" or .owner == $owner)
            and ($runtime_class == "" or .runtime_class == $runtime_class)
            and ($tag == "" or (.tags | index($tag) != null))
          )
        )
    ' "$inventory_path"
}

command_from_suite() {
  local suite_json=$1
  local kind
  kind=$(jq -r '.target.kind' <<<"$suite_json")

  case "$kind" in
    cargo-nextest)
      local -a cmd=(cargo nextest run)
      local profile package test_name feature_csv run_ignored
      profile=$(jq -r '.target.profile // empty' <<<"$suite_json")
      package=$(jq -r '.target.package // empty' <<<"$suite_json")
      test_name=$(jq -r '.target.test // empty' <<<"$suite_json")
      feature_csv=$(jq -r '.target.features | join(",")' <<<"$suite_json")
      run_ignored=$(jq -r '.target.run_ignored // "default"' <<<"$suite_json")

      if [[ -n "$profile" ]]; then
        cmd+=(-P "$profile")
      fi
      if [[ -n "$package" ]]; then
        cmd+=(-p "$package")
      fi
      if [[ -n "$test_name" ]]; then
        cmd+=(--test "$test_name")
      fi
      if [[ -n "$feature_csv" ]]; then
        cmd+=(--features "$feature_csv")
      fi
      if [[ "$run_ignored" != "default" ]]; then
        cmd+=(--run-ignored "$run_ignored")
      fi
      printf '%q ' "${cmd[@]}"
      printf '\n'
      ;;
    nix-build)
      local flake_attr
      flake_attr=$(jq -r '.target.flake_attr' <<<"$suite_json")
      printf 'nix build .#%q\n' "$flake_attr"
      ;;
    *)
      echo "unsupported suite target kind: $kind" >&2
      exit 1
      ;;
  esac
}

subcommand=${1:-}
if [[ -z "$subcommand" ]]; then
  usage >&2
  exit 1
fi
shift

case "$subcommand" in
  export|check)
    cd "$repo_root"
    run_inventory_tool "$subcommand"
    ;;
  list|run)
    cd "$repo_root"
    ensure_inventory_is_current

    suite_id=""
    layer=""
    owner=""
    runtime_class=""
    tag=""
    format="ids"

    while [[ $# -gt 0 ]]; do
      case "$1" in
        --suite)
          suite_id=${2:?missing suite id}
          shift 2
          ;;
        --layer)
          layer=${2:?missing layer}
          shift 2
          ;;
        --owner)
          owner=${2:?missing owner}
          shift 2
          ;;
        --runtime-class)
          runtime_class=${2:?missing runtime class}
          shift 2
          ;;
        --tag)
          tag=${2:?missing tag}
          shift 2
          ;;
        --format)
          format=${2:?missing format}
          shift 2
          ;;
        *)
          usage >&2
          exit 1
          ;;
      esac
    done

    selected=$(jq_selection "$suite_id" "$layer" "$owner" "$runtime_class" "$tag")
    count=$(jq 'length' <<<"$selected")
    if [[ "$count" -eq 0 ]]; then
      echo "no suites matched the requested filters" >&2
      exit 1
    fi

    if [[ "$subcommand" == "list" ]]; then
      case "$format" in
        json)
          jq . <<<"$selected"
          ;;
        commands)
          while IFS= read -r suite_json; do
            command_from_suite "$suite_json"
          done < <(jq -c '.[]' <<<"$selected")
          ;;
        ids)
          jq -r '.[].id' <<<"$selected"
          ;;
        *)
          echo "unsupported format: $format" >&2
          exit 1
          ;;
      esac
      exit 0
    fi

    while IFS= read -r suite_json; do
      suite_id=$(jq -r '.id' <<<"$suite_json")
      resolved_command=$(command_from_suite "$suite_json")
      echo "+ [$suite_id] $resolved_command"
      eval "$resolved_command"
    done < <(jq -c '.[]' <<<"$selected")
    ;;
  report)
    junit_xml="target/nextest/default/junit.xml"
    output_arg=""
    shift
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --junit-xml) junit_xml=${2:?missing junit xml path}; shift 2 ;;
        --output) output_arg="--output ${2:?missing output path}"; shift 2 ;;
        *) usage >&2; exit 1 ;;
      esac
    done
    # shellcheck disable=SC2086
    run_inventory_tool report --junit-xml "$junit_xml" $output_arg
    ;;
  coverage)
    run_inventory_tool coverage
    ;;
  *)
    usage >&2
    exit 1
    ;;
esac
