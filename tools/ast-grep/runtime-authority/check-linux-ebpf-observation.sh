#!/usr/bin/env bash
set -euo pipefail

src=$1
authority="$src/tools/ast-grep/runtime-authority"
report="$TMPDIR/ebpf-observation.json"
boundary=ebpf-observation-mutation-boundary
extension=extension-bpf-mutation-handles
loader=molten-bpf-loader-library
example=examples/ebpf-observe.rs
node=src/cli/ops/node/lifecycle/linuxobservation.rs
admission=src/fabric_observability/nodeadmission.rs
observer=src/fabric_observability/nodeadmission
adapter_dirs=(src/fabric_observability/kernel crates/molten-core/src/fabric_observability/ebpf)
extension_dirs=(src/system_extension crates/molten-core/src/system_extension)
adapter_probes=("${adapter_dirs[@]/%//mod.rs}" "$admission" "$observer/observerpoll.rs" "$node" "$example")
extension_probes=("${extension_dirs[@]/%//mod.rs}")
loader_probes=("${adapter_probes[@]}" "${extension_probes[@]}")

# Mirror real source paths so the plants exercise the rules' production globs.
# Both fixtures are parsed as text only. Never build or execute them.
for probe in "${loader_probes[@]}"; do
  test -f "$src/$probe"
  install -D "$authority/fixtures/positive/ebpf_observation_boundary.rs" "$TMPDIR/denied/$probe"
  install -D "$authority/fixtures/negative/ebpf_observation_read.rs" "$TMPDIR/safe/$probe"
done

scan() (
  cd "$1" || exit
  local rule=$2
  shift 2
  ast-grep scan --rule "$authority/rules/$rule.yml" --json=compact \
    --no-ignore vcs --no-ignore dot --no-ignore parent "$@" > "$report"
)

assert_report() {
  if ! jq -e "$@" > /dev/null; then
    echo "Unexpected findings for $rule ($scenario):" >&2
    cat "$report" >&2
    exit 1
  fi
}

check_fixtures() {
  local rule=$1 expected=$2 status=0 scenario=denied
  shift 2
  scan "$TMPDIR/denied" "$rule" "$@" || status=$?
  if [[ $status != 1 ]]; then
    echo "$rule: denied fixture expected ast-grep diagnostic status 1, got $status" >&2
    exit 1
  fi
  assert_report --arg rule "$rule" --argjson expected "$expected" '
    all(.[]; .ruleId == $rule and .severity == "error")
    and (map([.file, .text]) | sort) == (
      [$ARGS.positional[] as $path | $expected[] | [$path, .]] | sort
    )
  ' "$report" --args "$@"

  scenario=safe
  scan "$TMPDIR/safe" "$rule" "$@"
  assert_report '. == []' "$report"
}

check_fixtures "$boundary" '[
  "aya", "MapMut", "LinkFd", "libc::syscall(PLANTED_ONLY)", "std::fs::read_dir(PLANTED_ONLY)",
  "\"/sys/fs/bpf\"", "r\"/proc/planted-only\"", "caps::set(PLANTED_ONLY)", "planted_root.write(PLANTED_ONLY)"
]' "${adapter_probes[@]}"
check_fixtures "$extension" '["aya", "MapMut", "LinkFd"]' "${extension_probes[@]}"
check_fixtures "$loader" '["aya"]' "${loader_probes[@]}"

scenario=production
for rule in "$extension" "$loader"; do
  if [[ $rule == "$extension" ]]; then
    scan "$src" "$rule" "${extension_dirs[@]}"
  else
    scan "$src" "$rule" src crates "$example"
  fi
  assert_report '. == []' "$report"
done

# Allow only the example's own-status locator and the node's exact read of the
# running boot ID. All endpoint and profile reads stay capability-relative.
rule=$boundary
status=0
scan "$src" "$rule" "${adapter_dirs[@]}" "$admission" "$observer" "$node" "$example" || status=$?
assert_report --argjson status "$status" --arg rule "$rule" --arg example "$example" --arg node "$node" '
  if $status == 0 then
    . == []
  elif $status == 1 then
    length > 0 and all(.[];
      .ruleId == $rule and .severity == "error" and (
        (.file == $example and .text == "\"/proc/{}\""
         and (.lines | rtrimstr("\n")) == "    let own_process = std::path::PathBuf::from(format!(\"/proc/{}\", std::process::id()));")
        or (.file == $node and ( .text == "\"/proc/sys/kernel/random/boot_id\""
                                or .text == "std::fs::File::open(\"/proc/sys/kernel/random/boot_id\")")
            and (.lines | rtrimstr("\n")) == "    std::fs::File::open(\"/proc/sys/kernel/random/boot_id\")")
      )
    )
  else false end
' "$report"
