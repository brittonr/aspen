{ pkgs, src }:
assert pkgs.tlaplus.version == "1.7.4";
{
  # Bounded TLC models of audit findings #44, #46, #54, #55, #56, #57 and #59.
  # Each configuration must reproduce its recorded outcome in
  # verification/tla/audit/expectations.tsv: current-code configurations must
  # still violate their property, fixed and witness configurations must match.
  # The pinned, unchanged TLA-ts publication checks the same source and
  # configurations; verdicts must agree and exhaustive passes must agree on
  # distinct-state counts.
  audit-findings-tla = pkgs.runCommand "molten-audit-findings-tla"
    {
      nativeBuildInputs = [ pkgs.tlaplus pkgs.nodejs pkgs.jq pkgs.gnutar pkgs.gzip pkgs.bash pkgs.gawk pkgs.gnugrep pkgs.coreutils ];
      inherit src;
      meta.timeout = 1800;
    }
    ''
      set -euo pipefail
      cp -R "$src/verification/tla/audit" models
      chmod -R u+w models
      tar -xzf "$src/verification/tla-ts/tla-ts-0.0.10.tgz" -C "$TMPDIR"
      bash models/check.sh "$PWD/models" "$out" "$TMPDIR" "$src/verification/tla-ts/check.mjs"
    '';
}
