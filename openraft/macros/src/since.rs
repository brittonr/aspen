use std::str::FromStr;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::Parser;
use syn::spanned::Spanned;

use crate::utils;

pub struct Since {
    pub(crate) version: Option<String>,
    pub(crate) date: Option<String>,
    /// The change message.
    pub(crate) change: Vec<String>,
}

impl Since {
    /// Build a `Since` struct from the given attribute arguments.
    pub(crate) fn new(args: TokenStream) -> Result<Since, syn::Error> {
        let mut since = Since {
            version: None,
            date: None,
            change: vec![],
        };

        type AttributeArgs = syn::punctuated::Punctuated<syn::Meta, syn::Token![,]>;

        let parsed_args = AttributeArgs::parse_terminated.parse(args.clone())?;

        for arg in parsed_args {
            match arg {
                syn::Meta::NameValue(namevalue) => {
                    let q = namevalue
                        .path
                        .get_ident()
                        .ok_or_else(|| syn::Error::new_spanned(&namevalue, "Must have specified ident"))?;

                    let ident = q.to_string().to_lowercase();

                    match ident.as_str() {
                        "version" => {
                            since.set_version(namevalue.value.clone(), Spanned::span(&namevalue.value))?;
                        }

                        "date" => {
                            since.set_date(namevalue.value.clone(), Spanned::span(&namevalue.value))?;
                        }

                        "change" => {
                            since.set_change(namevalue.value.clone(), Spanned::span(&namevalue.value))?;
                        }

                        name => {
                            let msg =
                                format!("Unknown attribute {} is specified; expected one of: `version`, `date`", name,);
                            return Err(syn::Error::new_spanned(namevalue, msg));
                        }
                    }
                }
                other => {
                    return Err(syn::Error::new_spanned(other, "Unknown attribute inside the macro"));
                }
            }
        }

        if since.version.is_none() {
            return Err(syn::Error::new_spanned(proc_macro2::TokenStream::from(args), "Missing `version` attribute"));
        }

        Ok(since)
    }
    /// Append a `since` doc such as `Since: 1.0.0` to the bottom of the doc section.
    pub(crate) fn append_since_doc(self, item: TokenStream) -> Result<TokenStream, syn::Error> {
        let item = proc_macro2::TokenStream::from(item);

        let token_count = item.clone().into_iter().count();

        // Present docs to skip, in order to append `since` at bottom of doc section.
        let mut present_docs = Vec::with_capacity(token_count);

        // Tokens left after present docs.
        let mut last_non_doc = Vec::with_capacity(token_count);

        let tokens: Vec<_> = item.clone().into_iter().collect();
        let mut token_index = 0usize;
        while token_index < tokens.len() {
            let curr = tokens[token_index].clone();
            let next_index = token_index.saturating_add(1);
            if next_index >= tokens.len() {
                last_non_doc.push(curr);
                break;
            }

            let next = tokens[next_index].clone();
            if utils::is_doc(&curr, &next) {
                present_docs.push(curr);
                present_docs.push(next);
                token_index = token_index.saturating_add(2);
                continue;
            }

            last_non_doc.push(curr);
            last_non_doc.push(next);
            token_index = token_index.saturating_add(2);
            break;
        }

        let since_docs_str = if present_docs.is_empty() {
            format!(r#"#[doc = " {}"]"#, self.to_doc_string())
        } else {
            // If there are already docs, insert a blank line.
            format!(r#"#[doc = ""] #[doc = " {}"]"#, self.to_doc_string())
        };
        let since_docs = proc_macro2::TokenStream::from_str(&since_docs_str)
            .map_err(|e| syn::Error::new(Span::call_site(), format!("failed to parse generated docs: {e}")))?;

        let present_docs: proc_macro2::TokenStream = present_docs.into_iter().collect();
        let last_non_docs: proc_macro2::TokenStream = last_non_doc.into_iter().collect();

        // Other non doc tokens.
        let other: proc_macro2::TokenStream = tokens.into_iter().skip(token_index).collect();

        let tokens = quote! {
            #present_docs
            #since_docs
            #last_non_docs
            #other
        };
        Ok(tokens.into())
    }

    /// Build doc string: `Since: 1.0.0, Date(2021-01-01)` or  `Since: 1.0.0`
    fn to_doc_string(&self) -> String {
        let mut s = String::new();
        s.push_str("Since: ");

        if let Some(version) = &self.version {
            s.push_str(version.as_str());
        }
        if let Some(date) = &self.date {
            s.push_str(&format!(", Date({})", date));
        }

        if !self.change.is_empty() {
            s.push_str(&format!(": {}", self.change.join("; ")));
        }
        s
    }

    pub(crate) fn set_version(&mut self, ver_lit: syn::Expr, span: Span) -> Result<(), syn::Error> {
        if self.version.is_some() {
            return Err(syn::Error::new(span, "`version` set multiple times."));
        }

        let ver = Self::parse_str(ver_lit, "version", span)?;

        semver::Version::parse(&ver)
            .map_err(|_e| syn::Error::new(span, format!("`version`(`{}`) is not valid semver.", ver)))?;

        self.version = Some(ver);

        Ok(())
    }

    pub(crate) fn set_date(&mut self, date_lit: syn::Expr, span: Span) -> Result<(), syn::Error> {
        if self.date.is_some() {
            return Err(syn::Error::new(span, "`date` set multiple times."));
        }

        let date_str = Self::parse_str(date_lit, "date", span)?;

        chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|_e| {
            syn::Error::new(
                span,
                format!("`date`(`{}`) is not valid date string. Expected format: yyyy-mm-dd", date_str),
            )
        })?;

        self.date = Some(date_str);

        Ok(())
    }

    pub(crate) fn set_change(&mut self, change_lit: syn::Expr, span: Span) -> Result<(), syn::Error> {
        let change_str = Self::parse_str(change_lit, "change", span)?;

        self.change.push(change_str);

        Ok(())
    }

    /// Extract string from `foo` or `"foo"`
    fn parse_str(expr: syn::Expr, field: &str, span: Span) -> Result<String, syn::Error> {
        let s = match expr {
            syn::Expr::Lit(s) => match s.lit {
                syn::Lit::Str(s) => s.value(),
                syn::Lit::Verbatim(s) => s.to_string(),
                _ => {
                    return Err(syn::Error::new(span, format!("Failed to parse value of `{}` as string.", field)));
                }
            },
            syn::Expr::Verbatim(s) => s.to_string(),
            _ => {
                return Err(syn::Error::new(span, format!("Failed to parse value of `{}` as string.", field)));
            }
        };
        Ok(s)
    }
}
