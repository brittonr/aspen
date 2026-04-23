//! vCard 4.0 parser (RFC 6350).
//!
//! Parses vCard text format into Contact structs. Supports:
//! - Line unfolding (continuation lines starting with space/tab)
//! - Property parameters (TYPE=work, PREF=1)
//! - Structured values (N, ADR with `;`-separated components)
//! - Multi-valued properties (CATEGORIES with `,`-separated values)

use crate::error::ContactsError;
use crate::types::Contact;
use crate::types::ContactAddress;
use crate::types::ContactEmail;
use crate::types::ContactPhone;

/// Parse a single vCard from text.
///
/// Returns a partially-filled Contact (caller must set `id`, `book_id`,
/// `created_at_ms`, `updated_at_ms`).
pub fn parse_vcard(input: &str) -> Result<Contact, ContactsError> {
    let lines = unfold_lines(input);
    let mut is_in_vcard = false;
    let mut contact = empty_contact();

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.eq_ignore_ascii_case("BEGIN:VCARD") {
            is_in_vcard = true;
            continue;
        }
        if trimmed.eq_ignore_ascii_case("END:VCARD") {
            break;
        }
        if !is_in_vcard || is_version_line(trimmed) {
            continue;
        }

        let (name, params, value) = parse_property(trimmed);
        apply_property(&mut contact, name, &params, value);
    }

    if contact.display_name.is_empty() && contact.uid.is_empty() {
        return Err(ContactsError::ParseVcard {
            reason: "missing FN or UID property".to_string(),
        });
    }

    debug_assert!(
        !contact.display_name.is_empty() || !contact.uid.is_empty(),
        "valid vCard must expose FN or UID"
    );
    Ok(contact)
}

fn empty_contact() -> Contact {
    Contact {
        id: String::new(),
        book_id: String::new(),
        uid: String::new(),
        display_name: String::new(),
        family_name: None,
        given_name: None,
        emails: Vec::new(),
        phones: Vec::new(),
        addresses: Vec::new(),
        organization: None,
        title: None,
        birthday: None,
        notes: None,
        photo_blob_hash: None,
        categories: Vec::new(),
        url: None,
        custom_fields: Vec::new(),
        created_at_ms: 0,
        updated_at_ms: 0,
    }
}

fn is_version_line(line: &str) -> bool {
    line.to_ascii_uppercase().starts_with("VERSION:")
}

fn apply_property(contact: &mut Contact, name: &str, params: &[(&str, &str)], value: &str) {
    debug_assert!(!name.is_empty(), "parsed property names should be non-empty");
    let name_upper = name.to_ascii_uppercase();
    match name_upper.as_str() {
        "FN" => contact.display_name = value.to_string(),
        "N" => parse_structured_name(contact, value),
        "EMAIL" => {
            let label = extract_type_param(params);
            let is_primary = params.iter().any(|(key, _)| key.eq_ignore_ascii_case("PREF"));
            contact.emails.push(ContactEmail {
                address: value.to_string(),
                label,
                is_primary,
            });
        }
        "TEL" => {
            let label = extract_type_param(params);
            let is_primary = params.iter().any(|(key, _)| key.eq_ignore_ascii_case("PREF"));
            contact.phones.push(ContactPhone {
                number: value.to_string(),
                label,
                is_primary,
            });
        }
        "ADR" => {
            let label = extract_type_param(params);
            parse_address(contact, value, label);
        }
        "ORG" => contact.organization = Some(value.to_string()),
        "TITLE" => contact.title = Some(value.to_string()),
        "BDAY" => contact.birthday = Some(value.to_string()),
        "NOTE" => contact.notes = Some(value.to_string()),
        "CATEGORIES" => {
            contact.categories = value.split(',').map(|part| part.trim().to_string()).collect();
        }
        "URL" => contact.url = Some(value.to_string()),
        "UID" => contact.uid = value.to_string(),
        _ if name_upper.starts_with("X-") => {
            contact.custom_fields.push((name.to_string(), value.to_string()));
        }
        _ => {}
    }
}

/// Parse multiple vCards from a single text block.
///
/// Splits on BEGIN:VCARD / END:VCARD boundaries.
pub fn parse_vcards(input: &str) -> Result<Vec<Contact>, ContactsError> {
    let mut contacts = Vec::with_capacity(
        input.lines().filter(|line| line.trim().eq_ignore_ascii_case("BEGIN:VCARD")).count(),
    );
    let mut current = String::new();
    let mut is_in_vcard = false;

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("BEGIN:VCARD") {
            is_in_vcard = true;
            current.clear();
            current.push_str(line);
            current.push('\n');
            continue;
        }
        if trimmed.eq_ignore_ascii_case("END:VCARD") {
            debug_assert!(is_in_vcard, "END:VCARD should follow BEGIN:VCARD");
            current.push_str(line);
            current.push('\n');
            is_in_vcard = false;
            contacts.push(parse_vcard(&current)?);
            continue;
        }
        if is_in_vcard {
            current.push_str(line);
            current.push('\n');
        }
    }

    debug_assert!(!is_in_vcard, "all BEGIN:VCARD blocks should terminate");
    Ok(contacts)
}

/// Unfold continuation lines (RFC 6350 §3.2).
///
/// Lines starting with a space or tab are continuations of the previous line.
fn unfold_lines(input: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::with_capacity(input.lines().count());

    for raw_line in input.lines() {
        let line = raw_line.trim_end_matches('\r');
        if (line.starts_with(' ') || line.starts_with('\t')) && !result.is_empty() {
            if let Some(last) = result.last_mut() {
                match line.get(1..) {
                    Some(continuation) => last.push_str(continuation),
                    None => debug_assert!(false, "continuation lines always have a leading character"),
                }
            }
        } else {
            result.push(line.to_string());
        }
    }

    result
}

fn slice_after(text: &str, index: usize) -> &str {
    match index.checked_add(1).and_then(|next_index| text.get(next_index..)) {
        Some(suffix) => suffix,
        None => "",
    }
}

/// Parse a property line into (name, params, value).
///
/// Format: `NAME;PARAM1=VAL1;PARAM2=VAL2:VALUE`
fn parse_property(line: &str) -> (&str, Vec<(&str, &str)>, &str) {
    let colon_pos = find_value_colon(line);
    let (name_params, value) = if let Some(pos) = colon_pos {
        (&line[..pos], slice_after(line, pos))
    } else {
        (line, "")
    };

    let mut params = Vec::new();
    let name;
    if let Some(semi_pos) = name_params.find(';') {
        name = &name_params[..semi_pos];
        let params_str = slice_after(name_params, semi_pos);
        params = Vec::with_capacity(params_str.split(';').count());
        for param in params_str.split(';') {
            if let Some(eq_pos) = param.find('=') {
                params.push((&param[..eq_pos], slice_after(param, eq_pos)));
            } else {
                params.push((param, ""));
            }
        }
    } else {
        name = name_params;
    }

    debug_assert!(!name.is_empty() || line.is_empty(), "non-empty property lines should expose a name");
    (name, params, value)
}

/// Find the colon that separates property name+params from value.
///
/// Skips colons inside quoted parameter values.
fn find_value_colon(line: &str) -> Option<usize> {
    let mut is_in_quotes = false;
    for (i, ch) in line.char_indices() {
        match ch {
            '"' => is_in_quotes = !is_in_quotes,
            ':' if !is_in_quotes => return Some(i),
            _ => {}
        }
    }
    None
}

/// Parse structured N property: Family;Given;Additional;Prefix;Suffix
fn parse_structured_name(contact: &mut Contact, value: &str) {
    let parts: Vec<&str> = value.split(';').collect();
    if let Some(family) = parts.first()
        && !family.is_empty()
    {
        contact.family_name = Some(family.to_string());
    }
    if let Some(given) = parts.get(1)
        && !given.is_empty()
    {
        contact.given_name = Some(given.to_string());
    }
}

/// Parse structured ADR property: PO;Ext;Street;City;Region;PostalCode;Country
fn parse_address(contact: &mut Contact, value: &str, label: Option<String>) {
    let parts: Vec<&str> = value.split(';').collect();
    let get = |idx: usize| -> Option<String> { parts.get(idx).filter(|s| !s.is_empty()).map(|s| s.to_string()) };

    contact.addresses.push(ContactAddress {
        street: get(2),
        city: get(3),
        region: get(4),
        postal_code: get(5),
        country: get(6),
        label,
    });
}

/// Extract TYPE parameter value from params list.
fn extract_type_param(params: &[(&str, &str)]) -> Option<String> {
    for (key, val) in params {
        if key.eq_ignore_ascii_case("TYPE") {
            return Some(val.to_lowercase());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_vcard() {
        let vcard = "\
BEGIN:VCARD\r\n\
VERSION:4.0\r\n\
FN:John Doe\r\n\
N:Doe;John;;;\r\n\
EMAIL;TYPE=work:john@example.com\r\n\
TEL;TYPE=cell:+1-555-0123\r\n\
UID:contact-001\r\n\
END:VCARD";

        let contact = parse_vcard(vcard).unwrap();
        assert_eq!(contact.display_name, "John Doe");
        assert_eq!(contact.family_name.as_deref(), Some("Doe"));
        assert_eq!(contact.given_name.as_deref(), Some("John"));
        assert_eq!(contact.emails.len(), 1);
        assert_eq!(contact.emails[0].address, "john@example.com");
        assert_eq!(contact.emails[0].label.as_deref(), Some("work"));
        assert_eq!(contact.phones.len(), 1);
        assert_eq!(contact.phones[0].number, "+1-555-0123");
        assert_eq!(contact.uid, "contact-001");
    }

    #[test]
    fn test_parse_full_contact() {
        let vcard = "\
BEGIN:VCARD\r\n\
VERSION:4.0\r\n\
FN:Jane Smith\r\n\
N:Smith;Jane;;Dr.;\r\n\
EMAIL;TYPE=work;PREF:jane@work.com\r\n\
EMAIL;TYPE=home:jane@home.com\r\n\
TEL;TYPE=work:+1-555-0001\r\n\
TEL;TYPE=cell;PREF:+1-555-0002\r\n\
ADR;TYPE=work:;;123 Main St;Springfield;IL;62701;US\r\n\
ORG:Acme Corp\r\n\
TITLE:Engineer\r\n\
BDAY:1985-03-15\r\n\
NOTE:A note about Jane.\r\n\
CATEGORIES:friend,coworker\r\n\
URL:https://jane.example.com\r\n\
UID:contact-002\r\n\
X-CUSTOM:custom-value\r\n\
END:VCARD";

        let contact = parse_vcard(vcard).unwrap();
        assert_eq!(contact.display_name, "Jane Smith");
        assert_eq!(contact.emails.len(), 2);
        assert!(contact.emails[0].is_primary);
        assert!(!contact.emails[1].is_primary);
        assert_eq!(contact.phones.len(), 2);
        assert!(contact.phones[1].is_primary);
        assert_eq!(contact.addresses.len(), 1);
        assert_eq!(contact.addresses[0].street.as_deref(), Some("123 Main St"));
        assert_eq!(contact.addresses[0].city.as_deref(), Some("Springfield"));
        assert_eq!(contact.addresses[0].country.as_deref(), Some("US"));
        assert_eq!(contact.organization.as_deref(), Some("Acme Corp"));
        assert_eq!(contact.title.as_deref(), Some("Engineer"));
        assert_eq!(contact.birthday.as_deref(), Some("1985-03-15"));
        assert_eq!(contact.notes.as_deref(), Some("A note about Jane."));
        assert_eq!(contact.categories, vec!["friend", "coworker"]);
        assert_eq!(contact.url.as_deref(), Some("https://jane.example.com"));
        assert_eq!(contact.custom_fields.len(), 1);
        assert_eq!(contact.custom_fields[0].0, "X-CUSTOM");
    }

    #[test]
    fn test_parse_line_unfolding() {
        // RFC 6350 §3.2: CRLF + single whitespace is the fold marker.
        // The fold marker replaces a position in the original text.
        // "has been " is split to "has been \r\n folded" — unfold strips
        // CRLF + the single continuation space, leaving "has been folded".
        let vcard = concat!(
            "BEGIN:VCARD\r\n",
            "VERSION:4.0\r\n",
            "FN:John Doe\r\n",
            "NOTE:This is a very long note that has been \r\n",
            " folded across multiple lines for \r\n",
            " readability.\r\n",
            "UID:fold-test\r\n",
            "END:VCARD",
        );

        let contact = parse_vcard(vcard).unwrap();
        assert_eq!(
            contact.notes.as_deref(),
            Some("This is a very long note that has been folded across multiple lines for readability.")
        );
    }

    #[test]
    fn test_parse_multiple_vcards() {
        let data = "\
BEGIN:VCARD\r\n\
VERSION:4.0\r\n\
FN:Alice\r\n\
UID:alice\r\n\
END:VCARD\r\n\
BEGIN:VCARD\r\n\
VERSION:4.0\r\n\
FN:Bob\r\n\
UID:bob\r\n\
END:VCARD";

        let contacts = parse_vcards(data).unwrap();
        assert_eq!(contacts.len(), 2);
        assert_eq!(contacts[0].display_name, "Alice");
        assert_eq!(contacts[1].display_name, "Bob");
    }

    #[test]
    fn test_parse_empty_vcard_fails() {
        let vcard = "\
BEGIN:VCARD\r\n\
VERSION:4.0\r\n\
END:VCARD";

        assert!(parse_vcard(vcard).is_err());
    }

    #[test]
    fn test_parse_utf8_names() {
        let vcard = "\
BEGIN:VCARD\r\n\
VERSION:4.0\r\n\
FN:田中太郎\r\n\
N:田中;太郎;;;\r\n\
UID:utf8-test\r\n\
END:VCARD";

        let contact = parse_vcard(vcard).unwrap();
        assert_eq!(contact.display_name, "田中太郎");
        assert_eq!(contact.family_name.as_deref(), Some("田中"));
        assert_eq!(contact.given_name.as_deref(), Some("太郎"));
    }
}
