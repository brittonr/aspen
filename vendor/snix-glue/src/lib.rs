#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod builder;
pub mod builtins;
pub mod fetchers;
pub mod known_paths;
pub mod snix_io;
pub mod snix_store_io;

mod fetchurl;

// Used as user agent in various HTTP Clients
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[cfg(test)]
mod tests;

/// Tell the Evaluator to resolve `<nix>` to the path `/__corepkgs__`,
/// which has special handling in [snix_io::SnixIO].
/// This is used in nixpkgs to import `fetchurl.nix` from `<nix>`.
pub fn configure_nix_path<'co, 'ro, 'env, IO>(
    eval_builder: snix_eval::EvaluationBuilder<'co, 'ro, 'env, IO>,
    nix_search_path: &Option<String>,
) -> snix_eval::EvaluationBuilder<'co, 'ro, 'env, IO> {
    eval_builder.nix_path(
        nix_search_path
            .as_ref()
            .map(|p| format!("nix=/__corepkgs__:{p}"))
            .or_else(|| Some("nix=/__corepkgs__".to_string())),
    )
}
