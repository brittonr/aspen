//! vCard 4.0 serializer.
//!
//! Serializes Contact structs to vCard text format (RFC 6350).

use crate::types::Contact;

/// Serialize a Contact to vCard 4.0 format.
pub fn serialize_vcard(contact: &Contact) -> String {
    let mut lines = Vec::with_capacity(20);

    lines.push("BEGIN:VCARD".to_string());
    lines.push("VERSION:4.0".to_string());

    if !contact.uid.is_empty() {
        lines.push(format!("UID:{}", contact.uid));
    }

    lines.push(format!("FN:{}", contact.display_name));

    // N: Family;Given;Additional;Prefix;Suffix
    let family = contact.family_name.as_deref().unwrap_or("");
    let given = contact.given_name.as_deref().unwrap_or("");
    lines.push(format!("N:{family};{given};;;"));

    for email in &contact.emails {
        let mut params = Vec::new();
        if let Some(ref label) = email.label {
            params.push(format!("TYPE={label}"));
        }
        if email.is_primary {
            params.push("PREF".to_string());
        }
        let param_str = if params.is_empty() {
            String::new()
        } else {
            format!(";{}", params.join(";"))
        };
        lines.push(format!("EMAIL{param_str}:{}", email.address));
    }

    for phone in &contact.phones {
        let mut params = Vec::new();
        if let Some(ref label) = phone.label {
            params.push(format!("TYPE={label}"));
        }
        if phone.is_primary {
            params.push("PREF".to_string());
        }
        let param_str = if params.is_empty() {
            String::new()
        } else {
            format!(";{}", params.join(";"))
        };
        lines.push(format!("TEL{param_str}:{}", phone.number));
    }

    for addr in &contact.addresses {
        let mut params = Vec::new();
        if let Some(ref label) = addr.label {
            params.push(format!("TYPE={label}"));
        }
        let param_str = if params.is_empty() {
            String::new()
        } else {
            format!(";{}", params.join(";"))
        };
        // ADR: PO;Ext;Street;City;Region;PostalCode;Country
        let street = addr.street.as_deref().unwrap_or("");
        let city = addr.city.as_deref().unwrap_or("");
        let region = addr.region.as_deref().unwrap_or("");
        let postal = addr.postal_code.as_deref().unwrap_or("");
        let country = addr.country.as_deref().unwrap_or("");
        lines.push(format!("ADR{param_str}:;;{street};{city};{region};{postal};{country}"));
    }

    if let Some(ref org) = contact.organization {
        lines.push(format!("ORG:{org}"));
    }
    if let Some(ref title) = contact.title {
        lines.push(format!("TITLE:{title}"));
    }
    if let Some(ref bday) = contact.birthday {
        lines.push(format!("BDAY:{bday}"));
    }
    if let Some(ref note) = contact.notes {
        lines.push(format!("NOTE:{note}"));
    }
    if !contact.categories.is_empty() {
        lines.push(format!("CATEGORIES:{}", contact.categories.join(",")));
    }
    if let Some(ref url) = contact.url {
        lines.push(format!("URL:{url}"));
    }

    for (name, value) in &contact.custom_fields {
        lines.push(format!("{name}:{value}"));
    }

    lines.push("END:VCARD".to_string());

    lines.join("\r\n") + "\r\n"
}

/// Serialize multiple contacts to a concatenated vCard block.
pub fn serialize_vcards(contacts: &[Contact]) -> String {
    contacts.iter().map(serialize_vcard).collect::<Vec<_>>().join("")
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
