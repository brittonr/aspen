use proc_macro::TokenStream;
use proc_macro2::TokenTree;

/// Check if the next two token is a doc attribute, such as `#[doc = "foo"]`.
///
/// An doc attribute is composed of a `#` token and a `Group` token with a `Bracket` delimiter:
/// ```text
/// Punct { ch: '#', },
/// Group {
///     delimiter: Bracket,
///     stream: TokenStream [
///         Ident { ident: "doc", },
///         Punct { ch: '=', },
///         Literal { kind: Str, symbol: " Doc", },
///     ],
/// },
/// ```
pub(crate) fn is_doc(curr: &TokenTree, next: &TokenTree) -> bool {
    let TokenTree::Punct(punct) = curr else {
        return false;
    };

    let punct_char = punct.as_char();
    if punct_char != '#' {
        return false;
    }
    debug_assert_eq!(punct_char, '#');

    let TokenTree::Group(group) = next else {
        return false;
    };

    if group.delimiter() != proc_macro2::Delimiter::Bracket {
        return false;
    }
    debug_assert_eq!(group.delimiter(), proc_macro2::Delimiter::Bracket);

    let stream = group.stream();
    if stream.is_empty() {
        return false;
    }
    debug_assert!(!stream.is_empty());

    let Some(first_token) = stream.into_iter().next() else {
        return false;
    };
    let TokenTree::Ident(ident) = first_token else {
        return false;
    };

    if ident != "doc" {
        return false;
    }
    debug_assert!(ident == "doc");
    true
}

pub(crate) fn token_stream_with_error(mut item: TokenStream, e: syn::Error) -> TokenStream {
    let compile_error = TokenStream::from(e.into_compile_error());
    debug_assert!(!compile_error.is_empty());
    item.extend(compile_error);
    debug_assert!(!item.is_empty());
    item
}
