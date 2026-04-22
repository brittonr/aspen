pub(super) const REQUIRED_APP_VARIANTS: &[&str] = &[
    "ContactsAddToGroup",
    "ContactsCreateBook",
    "ContactsCreateContact",
    "ContactsCreateGroup",
    "ContactsDeleteBook",
    "ContactsDeleteContact",
    "ContactsDeleteGroup",
    "ContactsExportVcard",
    "ContactsGetContact",
    "ContactsImportVcard",
    "ContactsListBooks",
    "ContactsListContacts",
    "ContactsListGroups",
    "ContactsRemoveFromGroup",
    "ContactsSearchContacts",
    "ContactsUpdateContact",
    "NetPublish",
    "NetUnpublish",
    "NetLookup",
    "NetList",
];

pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    if REQUIRED_APP_VARIANTS.contains(&variant_name) {
        Some("contacts")
    } else {
        None
    }
}
