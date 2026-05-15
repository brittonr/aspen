//! Narrow TAP lifecycle helper for Aspen VM-CI.
//!
//! This binary is intentionally tiny and policy-bound: it only accepts Aspen CI
//! TAP names (`ci-n<N>-vm<M>-tap`) and the fixed bridge `aspen-ci-br0` before it
//! delegates to `ip`. Operators may copy this binary to a mutable path and grant
//! that copy `cap_net_admin+ep`; `aspen-node` itself remains unprivileged.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitCode;
use std::process::Output;

const ALLOWED_BRIDGE: &str = "aspen-ci-br0";
const IFNAMSIZ_MINUS_NUL: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Ensure,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Request {
    action: Action,
    tap_name: String,
    bridge_name: String,
}

fn main() -> ExitCode {
    match parse_request(std::env::args_os().skip(1)).and_then(run_request) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("aspen-tap-helper: {error}");
            ExitCode::from(2)
        }
    }
}

fn parse_request<I>(args: I) -> Result<Request, String>
where I: IntoIterator<Item = OsString> {
    let args: Vec<String> = args
        .into_iter()
        .map(|arg| arg.into_string().map_err(|_| "arguments must be valid UTF-8".to_string()))
        .collect::<Result<_, _>>()?;

    match args.as_slice() {
        [action, tap_name, bridge_name] => {
            let action = match action.as_str() {
                "ensure" => Action::Ensure,
                "delete" => Action::Delete,
                other => return Err(format!("unsupported action {other:?}; expected ensure or delete")),
            };
            validate_tap_name(tap_name)?;
            validate_bridge_name(bridge_name)?;
            Ok(Request {
                action,
                tap_name: tap_name.clone(),
                bridge_name: bridge_name.clone(),
            })
        }
        _ => Err("usage: aspen-tap-helper <ensure|delete> <ci-nN-vmM-tap> <aspen-ci-br0>".to_string()),
    }
}

fn validate_bridge_name(bridge_name: &str) -> Result<(), String> {
    if bridge_name == ALLOWED_BRIDGE {
        Ok(())
    } else {
        Err(format!("bridge {bridge_name:?} is not allowlisted; expected {ALLOWED_BRIDGE}"))
    }
}

fn validate_tap_name(tap_name: &str) -> Result<(), String> {
    if tap_name.len() > IFNAMSIZ_MINUS_NUL {
        return Err(format!("TAP name {tap_name:?} exceeds {IFNAMSIZ_MINUS_NUL} bytes"));
    }

    let Some(rest) = tap_name.strip_prefix("ci-n") else {
        return Err(format!("TAP name {tap_name:?} must start with ci-n"));
    };
    let Some((node, rest)) = rest.split_once("-vm") else {
        return Err(format!("TAP name {tap_name:?} must contain -vm"));
    };
    let Some(vm) = rest.strip_suffix("-tap") else {
        return Err(format!("TAP name {tap_name:?} must end with -tap"));
    };

    if node.is_empty() || !node.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("TAP name {tap_name:?} has invalid node id"));
    }
    if vm.is_empty() || !vm.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("TAP name {tap_name:?} has invalid vm index"));
    }

    Ok(())
}

fn run_request(request: Request) -> Result<(), String> {
    match request.action {
        Action::Ensure => ensure_tap(&request.tap_name, &request.bridge_name),
        Action::Delete => delete_tap(&request.tap_name),
    }
}

fn ensure_tap(tap_name: &str, bridge_name: &str) -> Result<(), String> {
    if !ip_success(&["link", "show", "dev", tap_name])? {
        run_ip(&["tuntap", "add", "dev", tap_name, "mode", "tap"], "create TAP device")?;
    }
    run_ip(&["link", "set", "dev", tap_name, "master", bridge_name], "attach TAP device to bridge")?;
    run_ip(&["link", "set", "dev", tap_name, "up"], "bring TAP device up")?;
    Ok(())
}

fn delete_tap(tap_name: &str) -> Result<(), String> {
    let output = run_ip_raw(&["link", "delete", "dev", tap_name])?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("Cannot find device") || stderr.contains("does not exist") {
        return Ok(());
    }
    Err(format!("delete TAP device: ip link delete dev {tap_name} failed: {stderr}"))
}

fn ip_success(args: &[&str]) -> Result<bool, String> {
    Ok(run_ip_raw(args)?.status.success())
}

fn run_ip(args: &[&str], context: &str) -> Result<(), String> {
    let output = run_ip_raw(args)?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!("{context}: ip {} failed: {stderr}", args.join(" ")))
}

fn run_ip_raw(args: &[&str]) -> Result<Output, String> {
    let ip_path = ip_command_path();
    Command::new(&ip_path)
        .args(args)
        .output()
        .map_err(|source| format!("failed to invoke {}: {source}", ip_path.display()))
}

fn ip_command_path() -> PathBuf {
    std::env::var_os("ASPEN_CI_IP_PATH").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("ip"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Request, String> {
        parse_request(args.iter().map(OsString::from))
    }

    #[test]
    fn accepts_allowlisted_request() {
        let request = parse(&["ensure", "ci-n1-vm0-tap", ALLOWED_BRIDGE]).unwrap();
        assert_eq!(request.action, Action::Ensure);
        assert_eq!(request.tap_name, "ci-n1-vm0-tap");
        assert_eq!(request.bridge_name, ALLOWED_BRIDGE);
    }

    #[test]
    fn accepts_delete_action() {
        let request = parse(&["delete", "ci-n12-vm7-tap", ALLOWED_BRIDGE]).unwrap();
        assert_eq!(request.action, Action::Delete);
    }

    #[test]
    fn rejects_unknown_action() {
        let err = parse(&["flush", "ci-n1-vm0-tap", ALLOWED_BRIDGE]).unwrap_err();
        assert!(err.contains("unsupported action"));
    }

    #[test]
    fn rejects_non_ci_tap_names() {
        for tap in [
            "eth0",
            "ci-n-vm0-tap",
            "ci-n1-vm-tap",
            "ci-n1-vm0",
            "ci-n1-vm0-tap-extra",
        ] {
            assert!(validate_tap_name(tap).is_err(), "expected {tap} to be rejected");
        }
    }

    #[test]
    fn rejects_too_long_tap_names() {
        let err = validate_tap_name("ci-n123456-vm0-tap").unwrap_err();
        assert!(err.contains("exceeds"));
    }

    #[test]
    fn rejects_non_allowlisted_bridge() {
        let err = parse(&["ensure", "ci-n1-vm0-tap", "br0"]).unwrap_err();
        assert!(err.contains("not allowlisted"));
    }
}
