let Contracts = import "../../contracts.ncl" in
let Valid = (import "../../profile.ncl").local_live_pilot in
(
  Valid
  & {
    schema | force = "molten.system-extension.native-host-profile.v1",
    alpn | force = "molten/system-extension/native/v1",
    framing | force = "preserves-packed-single-frame-v1",
  }
) | Contracts.NativeHostProfile
