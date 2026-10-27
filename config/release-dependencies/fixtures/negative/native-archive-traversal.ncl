let Profile = import "../../profile.ncl" in
Profile & {
  archive_receipts | force = std.array.map
    (fun receipt =>
      if receipt.id == "hardened-wasmtime-resumable" then
        receipt & {
          archive_path | force = ".cairn/../outside",
        }
      else receipt
    )
    Profile.archive_receipts,
}
