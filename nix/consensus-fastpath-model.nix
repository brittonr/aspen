{ pkgs, src }:
assert pkgs.tlaplus.version == "1.7.4";
{
  consensus-fastpath-model-parity = pkgs.runCommand "molten-consensus-fastpath-model-parity"
    {
      nativeBuildInputs = [ pkgs.tlaplus pkgs.nodejs pkgs.gnutar pkgs.gzip ];
      inherit src;
      meta.timeout = 900;
    }
    ''
      set -euo pipefail
      cp -R "$src" source
      chmod -R u+w source
      tar -xzf source/verification/tla-ts/tla-ts-0.0.10.tgz -C "$TMPDIR"
      node --test source/verification/tla-ts/campaign.test.mjs
      node source/verification/tla-ts/campaign.mjs "$TMPDIR/campaign" "$TMPDIR"
      mkdir -p "$out"
      cp -R "$TMPDIR/campaign/." "$out/"
    '';
}
