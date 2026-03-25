//! Contains builtins that fetch paths from the Internet, or local filesystem.

use super::utils::select_string;
use crate::{
    fetchers::{Fetch, url_basename},
    snix_store_io::SnixStoreIO,
};
use nix_compat::nixhash::{HashAlgo, NixHash};
use snix_eval::builtin_macros::builtins;
use snix_eval::generators::Gen;
use snix_eval::generators::GenCo;
use snix_eval::{CatchableErrorKind, ErrorKind, Value, try_cek};
use std::{rc::Rc, sync::Arc};
use url::Url;

// Used as a return type for extract_fetch_args, which is sharing some
// parsing code between the fetchurl and fetchTarball builtins.
struct NixFetchArgs {
    url: Url,
    name: Option<String>,
    sha256: Option<[u8; 32]>,
}

// `fetchurl` and `fetchTarball` accept a single argument, which can either be the URL (as string),
// or an attrset, where `url`, `sha256` and `name` keys are allowed.
async fn extract_fetch_args(
    co: &GenCo,
    args: Value,
) -> Result<Result<NixFetchArgs, CatchableErrorKind>, ErrorKind> {
    if let Ok(url_str) = args.to_str() {
        // Get the raw bytes, not the ToString repr.
        let url_str =
            String::from_utf8(url_str.as_bytes().to_vec()).map_err(|_| ErrorKind::Utf8)?;

        // Parse the URL.
        let url = Url::parse(&url_str).map_err(|e| ErrorKind::SnixError(Arc::from(e)))?;

        return Ok(Ok(NixFetchArgs {
            url,
            name: None,
            sha256: None,
        }));
    }

    let attrs = args.to_attrs().map_err(|_| ErrorKind::TypeError {
        expected: "attribute set or contextless string",
        actual: args.type_of(),
    })?;

    // Reject disallowed attrset keys, to match Nix' behaviour.
    // We complain about the first unexpected key we find in the list.
    const VALID_KEYS: [&[u8]; 3] = [b"url", b"name", b"sha256"];
    if let Some(first_invalid_key) = attrs.keys().find(|k| !&VALID_KEYS.contains(&k.as_bytes())) {
        return Err(ErrorKind::UnexpectedArgumentBuiltin(
            first_invalid_key.clone(),
        ));
    }

    let url_str = try_cek!(select_string(co, &attrs, "url").await?)
        .ok_or_else(|| ErrorKind::AttributeNotFound { name: "url".into() })?;
    let name = try_cek!(select_string(co, &attrs, "name").await?);
    let sha256_str = try_cek!(select_string(co, &attrs, "sha256").await?);

    Ok(Ok(NixFetchArgs {
        url: Url::parse(&url_str).map_err(|e| ErrorKind::SnixError(Arc::from(e)))?,
        name,
        // parse the sha256 string into a digest, and bail out if it's not sha256.
        sha256: sha256_str
            .map(
                |sha256_str| match NixHash::from_str(&sha256_str, Some(HashAlgo::Sha256)) {
                    Ok(NixHash::Sha256(digest)) => Ok(digest),
                    _ => Err(ErrorKind::InvalidHash(sha256_str)),
                },
            )
            .transpose()?,
    }))
}

/// Format a Unix timestamp as `%Y%m%d%H%M%S` without pulling in chrono.
/// Matches Nix's `formatSecondsSinceEpoch` output.
fn format_seconds_since_epoch(timestamp: i64) -> String {
    if timestamp <= 0 {
        return "19700101000000".to_string();
    }
    let t = timestamp as u64;
    let secs_per_minute = 60u64;
    let secs_per_hour = 3600u64;
    let secs_per_day = 86400u64;

    let seconds = t % secs_per_minute;
    let minutes = (t % secs_per_hour) / secs_per_minute;
    let hours = (t % secs_per_day) / secs_per_hour;

    // Days since epoch
    let mut days = t / secs_per_day;

    // Compute year/month/day from days since 1970-01-01
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    let days_in_months: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 0u64;
    for (i, &dim) in days_in_months.iter().enumerate() {
        if days < dim {
            month = i as u64 + 1;
            break;
        }
        days -= dim;
    }
    let day = days + 1;

    format!(
        "{:04}{:02}{:02}{:02}{:02}{:02}",
        year, month, day, hours, minutes, seconds
    )
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[allow(unused_variables)] // for the `state` arg, for now
#[builtins(state = "Rc<SnixStoreIO>")]
pub(crate) mod fetcher_builtins {
    use bstr::ByteSlice;
    use nix_compat::{flakeref, nixhash::NixHash};
    use snix_eval::{NixContext, NixString, try_cek_to_value};
    use std::collections::BTreeMap;

    use super::*;

    /// Consumes a fetch.
    /// If there is enough info to calculate the store path without fetching,
    /// queue the fetch to be fetched lazily, and return the store path.
    /// If there's not enough info to calculate it, do the fetch now, and then
    /// return the store path.
    /// Note the builtins.typeof of fetchurl and fetchTarball are *not* "path", but "string",
    /// to stay bug-compatible with Nix.
    fn fetch_lazy(state: Rc<SnixStoreIO>, name: String, fetch: Fetch) -> Result<Value, ErrorKind> {
        let store_path = match fetch
            .store_path(&name)
            .map_err(|e| ErrorKind::SnixError(Arc::from(e)))?
        {
            Some(store_path) => {
                // Move the fetch to KnownPaths, so it can be actually fetched later.
                let sp = state
                    .known_paths
                    .borrow_mut()
                    .add_fetch(fetch, &name)
                    .expect("Snix bug: should only fail if the store path cannot be calculated");

                debug_assert_eq!(
                    sp, store_path,
                    "calculated store path by KnownPaths should match"
                );
                sp
            }
            None => {
                // If we don't have enough info, do the fetch now.
                let (store_path, _path_info) = state
                    .tokio_handle
                    .block_on(async { state.fetcher.ingest_and_persist(&name, fetch).await })
                    .map_err(|e| ErrorKind::SnixError(Arc::from(e)))?;

                store_path
            }
        };

        let s = store_path.to_absolute_path();

        // Emit the calculated Store Path, which needs to have context.
        let context = NixContext::new().append(snix_eval::NixContextElement::Plain(s.clone()));
        Ok(Value::String(NixString::new_context_from(context, s)))
    }

    #[builtin("fetchurl")]
    async fn builtin_fetchurl(
        state: Rc<SnixStoreIO>,
        co: GenCo,
        args: Value,
    ) -> Result<Value, ErrorKind> {
        let args = try_cek_to_value!(extract_fetch_args(&co, args).await?);

        // Derive the name from the URL basename if not set explicitly.
        let name = args
            .name
            .unwrap_or_else(|| url_basename(&args.url).to_owned());

        fetch_lazy(
            state,
            name,
            Fetch::URL {
                url: args.url,
                exp_hash: args.sha256.map(NixHash::Sha256),
            },
        )
    }

    #[builtin("fetchTarball")]
    async fn builtin_fetch_tarball(
        state: Rc<SnixStoreIO>,
        co: GenCo,
        args: Value,
    ) -> Result<Value, ErrorKind> {
        let args = try_cek_to_value!(extract_fetch_args(&co, args).await?);

        // Name defaults to "source" if not set explicitly.
        const DEFAULT_NAME_FETCH_TARBALL: &str = "source";
        let name = args
            .name
            .unwrap_or_else(|| DEFAULT_NAME_FETCH_TARBALL.to_owned());

        fetch_lazy(
            state,
            name,
            Fetch::Tarball {
                url: args.url,
                exp_nar_sha256: args.sha256,
            },
        )
    }

    #[builtin("fetchGit")]
    async fn builtin_fetch_git(
        state: Rc<SnixStoreIO>,
        co: GenCo,
        args: Value,
    ) -> Result<Value, ErrorKind> {
        // --- 4.1: Argument parsing ---
        // fetchGit accepts a string (URL) or an attrset.
        let (url, rev, git_ref, submodules, shallow, all_refs, nar_hash, name) =
            if let Ok(url_str) = args.to_str() {
                let url_str = String::from_utf8(url_str.as_bytes().to_vec())
                    .map_err(|_| ErrorKind::Utf8)?;
                let url =
                    Url::parse(&url_str).map_err(|e| ErrorKind::SnixError(Arc::from(e)))?;
                (url, None, None, false, false, false, None, None)
            } else {
                let attrs = args.to_attrs().map_err(|_| ErrorKind::TypeError {
                    expected: "attribute set or contextless string",
                    actual: args.type_of(),
                })?;

                // Reject unknown keys
                const VALID_KEYS: [&[u8]; 8] = [
                    b"url",
                    b"rev",
                    b"ref",
                    b"submodules",
                    b"shallow",
                    b"allRefs",
                    b"narHash",
                    b"name",
                ];
                if let Some(first_invalid_key) =
                    attrs.keys().find(|k| !VALID_KEYS.contains(&k.as_bytes()))
                {
                    return Err(ErrorKind::UnexpectedArgumentBuiltin(
                        first_invalid_key.clone(),
                    ));
                }

                let url_str = try_cek_to_value!(select_string(&co, &attrs, "url").await?)
                    .ok_or_else(|| ErrorKind::AttributeNotFound { name: "url".into() })?;
                let url = Url::parse(&url_str)
                    .map_err(|e| ErrorKind::SnixError(Arc::from(e)))?;

                let rev = try_cek_to_value!(select_string(&co, &attrs, "rev").await?);
                let git_ref = try_cek_to_value!(select_string(&co, &attrs, "ref").await?);
                let nar_hash_str =
                    try_cek_to_value!(select_string(&co, &attrs, "narHash").await?);
                let name = try_cek_to_value!(select_string(&co, &attrs, "name").await?);

                // Parse boolean attributes with defaults
                let submodules = match attrs.select("submodules") {
                    Some(Value::Bool(b)) => *b,
                    Some(v) => {
                        let forced =
                            snix_eval::generators::request_force(&co, v.clone()).await;
                        match forced {
                            Value::Bool(b) => b,
                            _ => {
                                return Err(ErrorKind::TypeError {
                                    expected: "bool",
                                    actual: forced.type_of(),
                                })
                            }
                        }
                    }
                    None => false,
                };

                let shallow = match attrs.select("shallow") {
                    Some(Value::Bool(b)) => *b,
                    Some(v) => {
                        let forced =
                            snix_eval::generators::request_force(&co, v.clone()).await;
                        match forced {
                            Value::Bool(b) => b,
                            _ => {
                                return Err(ErrorKind::TypeError {
                                    expected: "bool",
                                    actual: forced.type_of(),
                                })
                            }
                        }
                    }
                    None => false,
                };

                let all_refs = match attrs.select("allRefs") {
                    Some(Value::Bool(b)) => *b,
                    Some(v) => {
                        let forced =
                            snix_eval::generators::request_force(&co, v.clone()).await;
                        match forced {
                            Value::Bool(b) => b,
                            _ => {
                                return Err(ErrorKind::TypeError {
                                    expected: "bool",
                                    actual: forced.type_of(),
                                })
                            }
                        }
                    }
                    None => false,
                };

                // Parse narHash as SRI
                let nar_hash: Option<[u8; 32]> = nar_hash_str
                    .map(|s| match NixHash::from_sri(&s) {
                        Ok(NixHash::Sha256(digest)) => Ok(digest),
                        Ok(_) => Err(ErrorKind::InvalidHash(s)),
                        Err(e) => Err(ErrorKind::SnixError(Arc::from(e))),
                    })
                    .transpose()?;

                (url, rev, git_ref, submodules, shallow, all_refs, nar_hash, name)
            };

        let name = name.unwrap_or_else(|| "source".to_string());

        let fetch = Fetch::Git {
            url: url.clone(),
            rev: rev.clone(),
            r#ref: git_ref.clone(),
            shallow,
            submodules,
            all_refs,
            exp_nar_sha256: nar_hash,
        };

        // --- 4.2: Fetch-or-cache logic ---
        // If narHash is provided, check if we already have it in the store.
        if let Some(store_path) = fetch
            .store_path(&name)
            .map_err(|e| ErrorKind::SnixError(Arc::from(e)))?
        {
            let existing = state
                .tokio_handle
                .block_on(async {
                    state
                        .path_info_service
                        .get(*store_path.digest())
                        .await
                })
                .map_err(|e| ErrorKind::SnixError(Arc::from(e)))?;

            if existing.is_some() {
                // Already cached — build return attrset from store path.
                // We don't have git metadata cached, so we use the rev from args.
                let out_path = store_path.to_absolute_path();
                let context = NixContext::new()
                    .append(snix_eval::NixContextElement::Plain(out_path.clone()));

                let nar_hash_sri = nar_hash
                    .map(|h| NixHash::Sha256(h).to_sri_string())
                    .unwrap_or_default();

                let resolved_rev =
                    rev.clone().unwrap_or_else(|| "0".repeat(40));
                let short_rev = resolved_rev.chars().take(7).collect::<String>();

                let mut attrs = BTreeMap::new();
                attrs.insert(
                    "outPath".into(),
                    Value::String(NixString::new_context_from(context, out_path)),
                );
                attrs.insert("rev".into(), Value::from(resolved_rev.as_str()));
                attrs.insert("shortRev".into(), Value::from(short_rev.as_str()));
                attrs.insert("lastModified".into(), Value::Integer(0));
                attrs.insert("lastModifiedDate".into(), Value::from("19700101000000"));
                attrs.insert("revCount".into(), Value::Integer(0));
                attrs.insert("narHash".into(), Value::from(nar_hash_sri.as_str()));
                attrs.insert("submodules".into(), Value::Bool(submodules));

                return Ok(Value::Attrs(attrs.into()));
            }
        }

        // Not cached — clone, ingest, and persist.
        let (store_path, _path_info) = state
            .tokio_handle
            .block_on(async { state.fetcher.ingest_and_persist(&name, fetch).await })
            .map_err(|e| ErrorKind::SnixError(Arc::from(e)))?;

        // Get git metadata from the cloned repo.
        // We do a second clone to extract metadata — this is acceptable since
        // the first clone is cached by the OS page cache.
        let clone_url = url.clone();
        let clone_rev = rev.clone();
        let clone_ref = git_ref.clone();
        let metadata = state
            .tokio_handle
            .block_on(async {
                tokio::task::spawn_blocking(move || {
                    let temp_dir = tempfile::tempdir()?;
                    let bare_dir = temp_dir.path().join("bare");

                    // Quick bare clone just for metadata
                    let mut cmd = std::process::Command::new("git");
                    cmd.args(["clone", "--bare", "--single-branch"]);
                    if shallow {
                        cmd.arg("--depth=1");
                    }
                    if let Some(ref r) = clone_ref {
                        cmd.args(["--branch", r]);
                    }
                    cmd.arg(clone_url.as_str()).arg(&bare_dir);
                    let output = cmd.output()?;
                    if !output.status.success() {
                        return Ok::<_, std::io::Error>((
                            clone_rev.unwrap_or_else(|| "0".repeat(40)),
                            0i64,
                            0i64,
                        ));
                    }

                    // Resolve rev
                    let resolved_rev = if let Some(ref rev) = clone_rev {
                        rev.clone()
                    } else {
                        let target = clone_ref.as_deref().unwrap_or("HEAD");
                        let output = std::process::Command::new("git")
                            .args(["rev-parse", target])
                            .current_dir(&bare_dir)
                            .output()?;
                        String::from_utf8_lossy(&output.stdout).trim().to_string()
                    };

                    // Get commit timestamp
                    let log_output = std::process::Command::new("git")
                        .args(["log", "-1", "--format=%ct", &resolved_rev])
                        .current_dir(&bare_dir)
                        .output()?;
                    let last_modified: i64 = String::from_utf8_lossy(&log_output.stdout)
                        .trim()
                        .parse()
                        .unwrap_or(0);

                    // Get rev count (0 for shallow clones)
                    let count_output = std::process::Command::new("git")
                        .args(["rev-list", "--count", &resolved_rev])
                        .current_dir(&bare_dir)
                        .output()?;
                    let rev_count: i64 = String::from_utf8_lossy(&count_output.stdout)
                        .trim()
                        .parse()
                        .unwrap_or(0);

                    Ok((resolved_rev, last_modified, rev_count))
                })
                .await
                .map_err(|e| std::io::Error::other(e))?
            })
            .map_err(|e| ErrorKind::SnixError(Arc::from(e)))?;

        let (resolved_rev, last_modified, rev_count) = metadata;
        let short_rev = resolved_rev.chars().take(7).collect::<String>();

        // --- 4.3: Build return attrset ---
        let out_path = store_path.to_absolute_path();
        let context =
            NixContext::new().append(snix_eval::NixContextElement::Plain(out_path.clone()));

        // Compute actual NAR hash from the persisted path info
        let actual_nar_hash_sri = {
            let pi = state
                .tokio_handle
                .block_on(async {
                    state
                        .path_info_service
                        .get(*store_path.digest())
                        .await
                })
                .map_err(|e| ErrorKind::SnixError(Arc::from(e)))?;
            match pi {
                Some(pi) => NixHash::Sha256(pi.nar_sha256).to_sri_string(),
                None => String::new(),
            }
        };

        // --- 4.4: Format lastModifiedDate ---
        let last_modified_date = format_seconds_since_epoch(last_modified);

        let mut attrs = BTreeMap::new();
        attrs.insert(
            "outPath".into(),
            Value::String(NixString::new_context_from(context, out_path)),
        );
        attrs.insert("rev".into(), Value::from(resolved_rev.as_str()));
        attrs.insert("shortRev".into(), Value::from(short_rev.as_str()));
        attrs.insert("lastModified".into(), Value::Integer(last_modified));
        attrs.insert(
            "lastModifiedDate".into(),
            Value::from(last_modified_date.as_str()),
        );
        attrs.insert("revCount".into(), Value::Integer(rev_count));
        attrs.insert(
            "narHash".into(),
            Value::from(actual_nar_hash_sri.as_str()),
        );
        attrs.insert("submodules".into(), Value::Bool(submodules));

        Ok(Value::Attrs(attrs.into()))
    }

    // FUTUREWORK: make it a feature flag once #64 is implemented
    #[builtin("parseFlakeRef")]
    async fn builtin_parse_flake_ref(
        state: Rc<SnixStoreIO>,
        co: GenCo,
        value: Value,
    ) -> Result<Value, ErrorKind> {
        let flake_ref = value.to_str()?;
        let flake_ref_str = flake_ref.to_str()?;

        let fetch_args = flake_ref_str
            .parse()
            .map_err(|err| ErrorKind::SnixError(Arc::new(err)))?;

        // Convert the FlakeRef to our Value format
        let mut attrs = BTreeMap::new();

        // Extract type and url based on the variant
        match fetch_args {
            flakeref::FlakeRef::Git { url, .. } => {
                attrs.insert("type".into(), Value::from("git"));
                attrs.insert("url".into(), Value::from(url.to_string()));
            }
            flakeref::FlakeRef::GitHub {
                owner, repo, r#ref, ..
            } => {
                attrs.insert("type".into(), Value::from("github"));
                attrs.insert("owner".into(), Value::from(owner));
                attrs.insert("repo".into(), Value::from(repo));
                if let Some(ref_name) = r#ref {
                    attrs.insert("ref".into(), Value::from(ref_name));
                }
            }
            flakeref::FlakeRef::GitLab { owner, repo, .. } => {
                attrs.insert("type".into(), Value::from("gitlab"));
                attrs.insert("owner".into(), Value::from(owner));
                attrs.insert("repo".into(), Value::from(repo));
            }
            flakeref::FlakeRef::File { url, .. } => {
                attrs.insert("type".into(), Value::from("file"));
                attrs.insert("url".into(), Value::from(url.to_string()));
            }
            flakeref::FlakeRef::Tarball { url, .. } => {
                attrs.insert("type".into(), Value::from("tarball"));
                attrs.insert("url".into(), Value::from(url.to_string()));
            }
            flakeref::FlakeRef::Path { path, .. } => {
                attrs.insert("type".into(), Value::from("path"));
                attrs.insert(
                    "path".into(),
                    Value::from(path.to_string_lossy().into_owned()),
                );
            }
            _ => {
                // For all other ref types, return a simple type/url attributes
                attrs.insert("type".into(), Value::from("indirect"));
                attrs.insert("url".into(), Value::from(flake_ref_str));
            }
        }

        Ok(Value::Attrs(attrs.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_seconds_since_epoch_zero() {
        assert_eq!(format_seconds_since_epoch(0), "19700101000000");
    }

    #[test]
    fn test_format_seconds_since_epoch_negative() {
        assert_eq!(format_seconds_since_epoch(-1), "19700101000000");
    }

    #[test]
    fn test_format_seconds_since_epoch_known_date() {
        // 2023-11-14 22:13:20 UTC
        assert_eq!(format_seconds_since_epoch(1700000000), "20231114221320");
    }

    #[test]
    fn test_format_seconds_since_epoch_leap_year() {
        // 2024-02-29 12:00:00 UTC (leap day)
        assert_eq!(format_seconds_since_epoch(1709208000), "20240229120000");
    }

    #[test]
    fn test_format_seconds_since_epoch_epoch() {
        // 1970-01-01 00:00:01 UTC
        assert_eq!(format_seconds_since_epoch(1), "19700101000001");
    }
}
