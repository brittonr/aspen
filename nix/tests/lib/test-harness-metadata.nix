{lib}: let
  staleMessage = "test harness inventory is stale; run `scripts/test-harness.sh export`";

  readManifestEntries = dir: prefix: let
    entries = builtins.readDir dir;
  in
    lib.concatMap
    (name: let
      kind = entries.${name};
      relative =
        if prefix == ""
        then name
        else "${prefix}/${name}";
      path = dir + "/${name}";
    in
      if kind == "directory"
      then readManifestEntries path relative
      else if kind == "regular" && lib.hasSuffix ".ncl" name
      then [
        {
          inherit path relative;
        }
      ]
      else [])
    (builtins.attrNames entries);

  currentMetadataImpl = repoRoot: let
    manifestEntries = readManifestEntries (repoRoot + "/test-harness/suites") "test-harness/suites";
    manifestPaths = map (entry: entry.relative) manifestEntries;
    inputLines =
      [
        "test-harness/schema.ncl:${builtins.hashFile "sha256" (repoRoot + "/test-harness/schema.ncl")}"
      ]
      ++ map (entry: "${entry.relative}:${builtins.hashFile "sha256" entry.path}") manifestEntries;
  in {
    manifest_paths = manifestPaths;
    inputs_sha256 = builtins.hashString "sha256" (lib.concatStringsSep "\n" inputLines);
  };
in {
  currentMetadata = currentMetadataImpl;

  ensureInventoryFresh = repoRoot: inventory: let
    metadata = inventory.metadata or (throw "test harness inventory is missing metadata");
    current = currentMetadataImpl repoRoot;
  in
    if metadata.manifest_paths != current.manifest_paths
    then throw staleMessage
    else if metadata.inputs_sha256 != current.inputs_sha256
    then throw staleMessage
    else inventory;
}
