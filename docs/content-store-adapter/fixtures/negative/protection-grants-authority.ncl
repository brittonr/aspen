let Template = import "../../profile-template.ncl" in
Template.make {
  redb_indexed = {
    protection_grants_retention_authority = true,
    pin_grants_read_authority = true,
    unprotect_grants_delete_authority = true,
  },
}
