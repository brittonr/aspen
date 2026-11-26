{ pkgs }:
assert pkgs.tlaplus.version == "1.7.4";
let
  # Only the model, its configurations and the shared TLC transcript reader;
  # unrelated repository edits do not rebuild the check.
  src = pkgs.lib.fileset.toSource {
    root = ../.;
    fileset = pkgs.lib.fileset.unions [
      (pkgs.lib.fileset.fileFilter
        (file: (pkgs.lib.hasPrefix "AspenRaft" file.name || pkgs.lib.hasPrefix "RaftReadIndexWindow" file.name)
          && (file.hasExt "tla" || file.hasExt "cfg"))
        ../verification/tla)
      ../verification/tla/raft-campaign.mjs
      ../verification/tla/raft-campaign.test.mjs
      ../verification/tla-ts/campaign.mjs
      ../verification/tla-ts/dive.mjs
      ../verification/tla-ts/dive-release.json
    ];
  };
in
pkgs.lib.optionalAttrs (pkgs.stdenv.hostPlatform.system == "x86_64-linux") {
  production-raft-model = pkgs.runCommand "molten-production-raft-model"
    {
      nativeBuildInputs = [ pkgs.tlaplus pkgs.nodejs ];
      inherit src;
      meta.timeout = 7200;
    }
    ''
      set -euo pipefail
      cp -R "$src" source
      chmod -R u+w source
      node --test source/verification/tla/raft-campaign.test.mjs
      cores="''${NIX_BUILD_CORES:-1}"
      if [ "$cores" -lt 1 ]; then cores="$(nproc)"; fi
      node source/verification/tla/raft-campaign.mjs "$TMPDIR/campaign" "$cores"
      mkdir -p "$out"
      cp -R "$TMPDIR/campaign/." "$out/"
    '';
}
