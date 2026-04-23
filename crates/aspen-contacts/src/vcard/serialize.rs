//! vCard 4.0 serializer.
//!
//! Serializes Contact structs to vCard text format (RFC 6350).

use crate::types::Contact;
use crate::types::ContactAddress;
use crate::types::ContactEmail;
use crate::types::ContactPhone;

const VCARD_START_LINE: &str = "BEGIN:VCARD";
const VCARD_END_LINE: &str = "END:VCARD";
const VCARD_VERSION_LINE: &str = "VERSION:4.0";
const VCARD_LINE_SEPARATOR: &str = "\r\n";
const VCARD_LINE_CAPACITY: usize = 20;
const MIN_VCARD_LINE_COUNT: usize = 4;

/// Serialize a Contact to vCard 4.0 format.
pub fn serialize_vcard(contact: &Contact) -> String {
    let mut lines = Vec::with_capacity(VCARD_LINE_CAPACITY);

    push_required_lines(&mut lines, contact);
    push_email_lines(&mut lines, &contact.emails);
    push_phone_lines(&mut lines, &contact.phones);
    push_address_lines(&mut lines, &contact.addresses);
    push_optional_lines(&mut lines, contact);
    push_custom_field_lines(&mut lines, &contact.custom_fields);
    lines.push(VCARD_END_LINE.to_string());

    debug_assert!(
        !contact.display_name.is_empty(),
        "vCard serialization expects a non-empty display name"
    );
    debug_assert!(lines.len() >= MIN_VCARD_LINE_COUNT);
    debug_assert_eq!(lines.first().map(String::as_str), Some(VCARD_START_LINE));
    debug_assert_eq!(lines.last().map(String::as_str), Some(VCARD_END_LINE));

    join_vcard_lines(lines)
}

/// Serialize multiple contacts to a concatenated vCard block.
pub fn serialize_vcards(contacts: &[Contact]) -> String {
    contacts.iter().map(serialize_vcard).collect::<Vec<_>>().join("")
}

fn push_required_lines(lines: &mut Vec<String>, contact: &Contact) {
    lines.push(VCARD_START_LINE.to_string());
    lines.push(VCARD_VERSION_LINE.to_string());

    if !contact.uid.is_empty() {
        lines.push(format!("UID:{}", contact.uid));
    }

    lines.push(format!("FN:{}", contact.display_name));
    lines.push(format!(
        "N:{};{};;;",
        contact.family_name.as_deref().unwrap_or(""),
        contact.given_name.as_deref().unwrap_or("")
    ));
}

fn push_email_lines(lines: &mut Vec<String>, emails: &[ContactEmail]) {
    for email in emails {
        let param_suffix = format_typed_param_suffix(email.label.as_deref(), email.is_primary);
        lines.push(format!("EMAIL{param_suffix}:{}", email.address));
    }
}

fn push_phone_lines(lines: &mut Vec<String>, phones: &[ContactPhone]) {
    for phone in phones {
        let param_suffix = format_typed_param_suffix(phone.label.as_deref(), phone.is_primary);
        lines.push(format!("TEL{param_suffix}:{}", phone.number));
    }
}

fn push_address_lines(lines: &mut Vec<String>, addresses: &[ContactAddress]) {
    for address in addresses {
        let param_suffix = format_labeled_param_suffix(address.label.as_deref());
        lines.push(format!(
            "ADR{param_suffix}:;;{};{};{};{};{}",
            address.street.as_deref().unwrap_or(""),
            address.city.as_deref().unwrap_or(""),
            address.region.as_deref().unwrap_or(""),
            address.postal_code.as_deref().unwrap_or(""),
            address.country.as_deref().unwrap_or("")
        ));
    }
}

fn push_optional_lines(lines: &mut Vec<String>, contact: &Contact) {
    push_optional_line(lines, "ORG", contact.organization.as_deref());
    push_optional_line(lines, "TITLE", contact.title.as_deref());
    push_optional_line(lines, "BDAY", contact.birthday.as_deref());
    push_optional_line(lines, "NOTE", contact.notes.as_deref());

    if !contact.categories.is_empty() {
        lines.push(format!("CATEGORIES:{}", contact.categories.join(",")));
    }

    push_optional_line(lines, "URL", contact.url.as_deref());
}

fn push_optional_line(lines: &mut Vec<String>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        lines.push(format!("{key}:{value}"));
    }
}

fn push_custom_field_lines(lines: &mut Vec<String>, custom_fields: &[(String, String)]) {
    for (name, value) in custom_fields {
        lines.push(format!("{name}:{value}"));
    }
}

fn format_typed_param_suffix(label: Option<&str>, is_primary: bool) -> String {
    let mut params = Vec::with_capacity(2);
    if let Some(label) = label {
        params.push(format!("TYPE={label}"));
    }
    if is_primary {
        params.push("PREF".to_string());
    }
    format_param_suffix(params)
}

fn format_labeled_param_suffix(label: Option<&str>) -> String {
    let mut params = Vec::with_capacity(1);
    if let Some(label) = label {
        params.push(format!("TYPE={label}"));
    }
    format_param_suffix(params)
}

fn format_param_suffix(params: Vec<String>) -> String {
    if params.is_empty() {
        String::new()
    } else {
        format!(";{}", params.join(";"))
    }
}

fn join_vcard_lines(lines: Vec<String>) -> String {
    let mut serialized = lines.join(VCARD_LINE_SEPARATOR);
    serialized.push_str(VCARD_LINE_SEPARATOR);
    serialized
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vcard::parse::parse_vcard;

    #[test]
    fn test_serialize_roundtrip() {
        let vcard_input = "\
BEGIN:VCARD\r\n\
VERSION:4.0\r\n\
FN:John Doe\r\n\
N:Doe;John;;;\r\n\
EMAIL;TYPE=work:john@example.com\r\n\
TEL;TYPE=cell:+1-555-0123\r\n\
UID:roundtrip-001\r\n\
END:VCARD";

        let contact = parse_vcard(vcard_input).unwrap();
        let serialized = serialize_vcard(&contact);
        let reparsed = parse_vcard(&serialized).unwrap();

        assert_eq!(contact.display_name, reparsed.display_name);
        assert_eq!(contact.family_name, reparsed.family_name);
        assert_eq!(contact.given_name, reparsed.given_name);
        assert_eq!(contact.uid, reparsed.uid);
        assert_eq!(contact.emails.len(), reparsed.emails.len());
        assert_eq!(contact.emails[0].address, reparsed.emails[0].address);
        assert_eq!(contact.phones.len(), reparsed.phones.len());
        assert_eq!(contact.phones[0].number, reparsed.phones[0].number);
    }
}
