let Profile = import "../../profile.ncl" in
Profile & {
  dependencies | force = std.array.map
    (fun row =>
      if row.manifest_dependency == "hardened-wasmtime-runtime" then
        row & {
          source_coordinate | force = "rad://z3hRCegTsS8jpJVgxYfb9psEzxHpG/../delegate",
        }
      else row
    )
    Profile.dependencies,
}
