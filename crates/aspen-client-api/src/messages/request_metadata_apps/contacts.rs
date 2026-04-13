pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    match variant_name {
        "ContactsAddToGroup"
        | "ContactsCreateBook"
        | "ContactsCreateContact"
        | "ContactsCreateGroup"
        | "ContactsDeleteBook"
        | "ContactsDeleteContact"
        | "ContactsDeleteGroup"
        | "ContactsExportVcard"
        | "ContactsGetContact"
        | "ContactsImportVcard"
        | "ContactsListBooks"
        | "ContactsListContacts"
        | "ContactsListGroups"
        | "ContactsRemoveFromGroup"
        | "ContactsSearchContacts"
        | "ContactsUpdateContact"
        | "NetPublish"
        | "NetUnpublish"
        | "NetLookup"
        | "NetList" => Some("contacts"),
        _ => None,
    }
}
