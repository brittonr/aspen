#!/usr/bin/env bash
# Runs the fabric_execution test module 200 consecutive times against one built test binary.
set -uo pipefail
cd /home/brittonr/git/aspen-worktrees/fabric-exec-epipe
export CARGO_TARGET_DIR=/home/brittonr/git/aspen-worktrees/fabric-exec-epipe-target
log=/home/brittonr/git/aspen-worktrees/fabric-exec-epipe-scratch/loop-200.log
bin=$(nix develop -c cargo test -p molten --lib --no-run --message-format=json 2>/dev/null \
  | jq -r 'select(.reason=="compiler-artifact" and .target.name=="molten" and .profile.test==true) | .executable' | tail -n1)
: > "$log"
echo "binary=$bin head=$(git rev-parse HEAD) dirty=$(git status --porcelain -- src | wc -l)" >> "$log"
pass=0; fail=0
for i in $(seq 1 200); do
  if out=$("$bin" fabric_execution:: --test-threads=1 2>&1); then
    pass=$((pass+1)); echo "run $i ok $(grep -o '[0-9]* passed' <<<"$out")" >> "$log"
  else
    fail=$((fail+1)); echo "run $i FAIL" >> "$log"; echo "$out" >> "$log"
  fi
done
echo "summary pass=$pass fail=$fail" >> "$log"
tail -n1 "$log"
