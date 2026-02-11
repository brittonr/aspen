//! Function parsing for production and Verus code.
//!
//! This module provides AST-based parsing using `syn` for accurate function extraction.

pub mod production;
pub mod verus;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::FnArg;
use syn::GenericParam;
use syn::ReturnType;
use syn::Signature;
use syn::Type;

use crate::FunctionParam;
use crate::FunctionSignature;

/// Extract signature information from a syn Signature.
pub fn extract_signature(sig: &Signature) -> FunctionSignature {
    let params = sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Receiver(_) => None, // Skip self
            FnArg::Typed(pat_type) => {
                let name = pat_type.pat.to_token_stream().to_string();
                let ty = normalize_type_tokens(&pat_type.ty);
                Some(FunctionParam { name, ty })
            }
        })
        .collect();

    let return_type = match &sig.output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => Some(normalize_type_tokens(ty)),
    };

    let generics = sig
        .generics
        .params
        .iter()
        .map(|param| match param {
            GenericParam::Type(ty) => ty.ident.to_string(),
            GenericParam::Lifetime(lt) => format!("'{}", lt.lifetime.ident),
            GenericParam::Const(c) => format!("const {}", c.ident),
        })
        .collect();

    FunctionSignature {
        params,
        return_type,
        is_const: sig.constness.is_some(),
        is_async: sig.asyncness.is_some(),
        generics,
    }
}

/// Normalize type tokens to a canonical string form.
fn normalize_type_tokens(ty: &Type) -> String {
    let tokens = ty.to_token_stream();
    normalize_tokens(&tokens)
}

/// Normalize a token stream to a canonical string.
pub fn normalize_tokens(tokens: &TokenStream) -> String {
    // Convert to string and normalize whitespace
    let s = tokens.to_string();

    // Normalize spacing around punctuation
    let s = s
        .replace(" ::", "::")
        .replace(":: ", "::")
        .replace(" <", "<")
        .replace("< ", "<")
        .replace(" >", ">")
        .replace("> ", ">")
        .replace(" ,", ",");

    // Collapse multiple spaces
    let mut result = String::new();
    let mut prev_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(c);
            prev_space = false;
        }
    }

    result.trim().to_string()
}

/// Extract the body of a function as a normalized string.
pub fn extract_body_string(block: &syn::Block) -> String {
    let tokens = block.to_token_stream();
    let s = tokens.to_string();

    // Remove outer braces
    let s = s.trim();
    let s = if s.starts_with('{') && s.ends_with('}') {
        &s[1..s.len() - 1]
    } else {
        s
    };

    s.trim().to_string()
}
