
fn validate_trust_state(trust_state: &str) -> Result<()> {
    if matches!(
        trust_state,
        TRUST_STATE_UNKNOWN
            | TRUST_STATE_SOURCE_KNOWN
            | TRUST_STATE_BUILDER_ATTESTED
            | TRUST_STATE_REVIEWED
            | TRUST_STATE_REPRODUCIBLE_VERIFIED
            | TRUST_STATE_SANDBOX_ONLY
            | TRUST_STATE_POLICY_TRUSTED
            | TRUST_STATE_DENIED
    ) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("invalid provenance trust state `{trust_state}`")))
    }
}

fn validate_profile(profile: &str) -> Result<()> {
    if matches!(profile, PROFILE_NODE_CONTROL | PROFILE_LOCAL_TEST) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("invalid provenance evaluation profile `{profile}`")))
    }
}

fn validate_build_params(params: &[BuildParam]) -> Result<()> {
    ensure_ref_bound(params.len(), MAX_BUILD_PARAMS, "provenance build params")?;
    for param in params {
        validate_build_param(param)?;
    }
    Ok(())
}

fn validate_build_param(param: &BuildParam) -> Result<()> {
    validate_build_param_token(&param.key, "provenance build param key")?;
    validate_build_param_token(&param.value, "provenance build param value")
}

fn validate_build_param_token(value: &str, context: &str) -> Result<()> {
    if value.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{context} must not be empty")));
    }
    if value.len() > MAX_BUILD_PARAM_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "{context} is too long: {} > {MAX_BUILD_PARAM_BYTES}",
            value.len()
        )));
    }
    if value.contains('\n') || value.contains('\r') {
        return Err(MoltenError::invalid_harness(format!("{context} must not contain newlines")));
    }
    Ok(())
}

fn build_params_sequence(params: &[BuildParam]) -> IoValue {
    let mut sorted = params.to_vec();
    sorted.sort();
    sequence(
        sorted
            .iter()
            .map(|param| record("build-param", vec![string(&param.key), string(&param.value)]))
            .collect(),
    )
}

fn record_build_params_sequence(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<Vec<BuildParam>> {
    let record_value = value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} sequence>")))?;
    let Some(items) = fields[0].collect_sequence() else {
        return Err(MoltenError::invalid_harness(format!("{tag} must contain a sequence")));
    };
    ensure_ref_bound(items.len(), MAX_BUILD_PARAMS, tag)?;
    let mut params = Vec::with_capacity(items.len());
    for item in items.iter() {
        params.push(required_build_param(item, tag)?);
    }
    validate_build_params(&params)?;
    Ok(params)
}

fn required_build_param(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<BuildParam> {
    let item_value = value_to_iovalue(value);
    let fields = item_value
        .collect_simple_record("build-param", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} item must be <build-param key value>")))?;
    let key = fields[0]
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} build param key must be a string")))?;
    let value = fields[1]
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} build param value must be a string")))?;
    Ok(BuildParam { key, value })
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}
