#!/usr/bin/env bash
# Run every audit-finding TLA+ configuration through TLC and compare the outcome
# with expectations.tsv. Usage: check.sh MODEL_DIR OUTPUT_DIR [TLA_TS_ROOT TLA_TS_CHECK]
# A configuration fails the rail when TLC does not finish, reports a different
# outcome, or reports a different violated invariant. When a TLA-ts root and the
# repository's check.mjs are given, the same source and configuration also run
# through TLA-ts: its verdict and invariant name must match, and exhaustive
# passes must report the same distinct-state count as TLC. Counts from runs that
# stop at the first violation are prefix observations and are not compared.
# Raw output from both checkers is retained.
set -euo pipefail
model_dir=$1
out_dir=$2
tlats_root=${3:-}
tlats_check=${4:-}
mkdir -p "$out_dir"
failures=0
while IFS=$'\t' read -r config module expected issue meaning; do
  case "$config" in ''|'#'*) continue ;; esac
  log="$out_dir/${config%.cfg}.txt"
  set +e
  (cd "$model_dir" && tlc -workers auto -metadir "$out_dir/meta-${config%.cfg}" \
    -config "$config" "$module.tla") > "$log" 2>&1
  set -e
  rm -rf "$out_dir/meta-${config%.cfg}"
  if grep -q 'Model checking completed. No error has been found.' "$log"; then
    actual=pass
  elif violated=$(grep -m1 -oE 'Invariant [A-Za-z0-9_]+ is violated' "$log"); then
    actual="invariant:$(echo "$violated" | awk '{print $2}')"
  elif grep -q 'Temporal properties were violated' "$log"; then
    actual=temporal
  else
    actual=incomplete
  fi
  parity=tlc-only
  if [ -n "$tlats_root" ]; then
    ts_log="$out_dir/${config%.cfg}.tla-ts.json"
    set +e
    (cd "$model_dir" && node "$tlats_check" "$module.tla" "$config" "$tlats_root") > "$ts_log" 2>&1
    set -e
    ts_line=$(grep '^{' "$ts_log" | tail -1 || true)
    ts_status=$(printf '%s' "$ts_line" | jq -r '.status // "no-output"' 2>/dev/null || echo no-output)
    case "$ts_status" in
      success) ts_actual=pass ;;
      invariant-violation) ts_actual="invariant:$(printf '%s' "$ts_line" | jq -r '.violation.name')" ;;
      liveness-violation) ts_actual=temporal ;;
      *) ts_actual="tla-ts-$ts_status" ;;
    esac
    if [ "$ts_actual" != "$actual" ]; then
      parity="tla-ts-mismatch:$ts_actual"
    elif [ "$actual" = pass ]; then
      tlc_distinct=$(grep -oE '[0-9,]+ distinct states found' "$log" | tail -1 | tr -d ',' | awk '{print $1}')
      ts_distinct=$(printf '%s' "$ts_line" | jq -r '.distinctStates')
      if [ "$tlc_distinct" = "$ts_distinct" ]; then parity="tla-ts-agrees:$ts_distinct"; else parity="tla-ts-count-mismatch:$tlc_distinct/$ts_distinct"; fi
    else
      parity=tla-ts-agrees
    fi
  fi
  if [ "$actual" = "$expected" ] && { [ "$parity" = tlc-only ] || [ "${parity%%:*}" = tla-ts-agrees ]; }; then
    printf 'ok\t%s\t%s\t%s\t#%s\n' "$config" "$actual" "$parity" "$issue" | tee -a "$out_dir/summary.tsv"
  else
    printf 'FAIL\t%s\texpected=%s\tactual=%s\t%s\t#%s\n' "$config" "$expected" "$actual" "$parity" "$issue" \
      | tee -a "$out_dir/summary.tsv"
    failures=$((failures + 1))
  fi
done < "$model_dir/expectations.tsv"
if [ "$failures" -ne 0 ]; then
  echo "$failures configuration(s) did not match expectations.tsv or checker parity" >&2
  exit 1
fi
