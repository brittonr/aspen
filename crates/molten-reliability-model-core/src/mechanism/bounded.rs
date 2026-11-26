//! Deserializers that bound profile text and collections before any value is retained.

use alloc::borrow::ToOwned;

pub const MAX_TEXT_BYTES: usize = 256;
pub const MAX_ITEMS: usize = 4_096;

pub fn text<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<alloc::string::String, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = alloc::string::String;
        fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            formatter.write_str("a string of at most 256 bytes")
        }
        fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<alloc::string::String, E> {
            if value.len() > MAX_TEXT_BYTES {
                return Err(E::custom("profile text exceeds 256 bytes"));
            }
            Ok(value.to_owned())
        }
        fn visit_string<E: serde::de::Error>(self, value: alloc::string::String) -> Result<alloc::string::String, E> {
            if value.len() > MAX_TEXT_BYTES {
                return Err(E::custom("profile text exceeds 256 bytes"));
            }
            Ok(value)
        }
    }
    deserializer.deserialize_string(Visitor)
}

pub fn texts<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<alloc::vec::Vec<alloc::string::String>, D::Error> {
    struct Visitor;
    #[derive(serde::Deserialize)]
    struct Text(#[serde(deserialize_with = "text")] alloc::string::String);
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = alloc::vec::Vec<alloc::string::String>;
        fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            formatter.write_str("at most 4096 bounded strings")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            if sequence.size_hint().is_some_and(|size| size > MAX_ITEMS) {
                return Err(serde::de::Error::custom("profile collection exceeds 4096 items"));
            }
            let mut values = alloc::vec::Vec::new();
            while let Some(Text(value)) = sequence.next_element()? {
                if values.len() >= MAX_ITEMS {
                    return Err(serde::de::Error::custom("profile collection exceeds 4096 items"));
                }
                values.push(value);
            }
            Ok(values)
        }
    }
    deserializer.deserialize_seq(Visitor)
}

pub fn optional_text<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<alloc::string::String>, D::Error> {
    #[derive(serde::Deserialize)]
    struct Present(#[serde(deserialize_with = "text")] alloc::string::String);
    <Option<Present> as serde::Deserialize>::deserialize(deserializer).map(|value| value.map(|Present(text)| text))
}
