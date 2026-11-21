{ pkgs, src }:
assert pkgs.tlaplus.version == "1.7.4";
let
  dive = builtins.fromJSON (builtins.readFile ../verification/tla-ts/dive-release.json);
  diveName = "dive-${dive.tag}-${dive.target}";
  diveArchive = pkgs.fetchurl {
    url = "https://github.com/Chelis-Lang/dive/releases/download/${dive.tag}/${diveName}.tar.gz";
    inherit (dive) sha256;
  };
in
pkgs.lib.optionalAttrs (pkgs.stdenv.hostPlatform.system == "x86_64-linux") {
  consensus-fastpath-model-parity = pkgs.runCommand "molten-consensus-fastpath-model-parity"
    {
      nativeBuildInputs = [ pkgs.tlaplus pkgs.nodejs pkgs.gnutar pkgs.gzip pkgs.patchelf pkgs.b3sum ];
      inherit src;
      meta.timeout = 900;
    }
    ''
      set -euo pipefail
      cp -R "$src" source
      chmod -R u+w source
      tar -xzf source/verification/tla-ts/tla-ts-0.0.10.tgz -C "$TMPDIR"
      # The release also ships a TLC shim; extract only Dive and leave TLC authoritative.
      tar -xzf ${diveArchive} -C "$TMPDIR" ${diveName}/dive
      dive="$TMPDIR/${diveName}/dive"
      patchelf --set-interpreter ${pkgs.stdenv.cc.bintools.dynamicLinker} \
        --set-rpath ${pkgs.lib.makeLibraryPath [ pkgs.stdenv.cc.cc.lib pkgs.glibc ]} "$dive"
      node --test source/verification/tla-ts/campaign.test.mjs
      node source/verification/tla-ts/campaign.mjs "$TMPDIR/campaign" "$TMPDIR" ${diveArchive} "$dive"
      mkdir -p "$out"
      cp -R "$TMPDIR/campaign/." "$out/"
    '';
}
