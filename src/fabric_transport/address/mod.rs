//! Admission of IP connection hints exchanged between hosts.
//!
//! An exchanged hint (node live tickets, cross-process endpoint locators) is a dialing hint, not
//! local bind configuration and not authority. The textual `SocketAddrV6` form omits `flowinfo`,
//! and a numeric `scope_id` names an interface only on the host that produced it, so exchanged
//! hints admit neither. Local bind configuration keeps its own policy in the direct-stream
//! endpoint.

/// Why an IP address is outside the exchanged-hint domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpHintIssue {
    Unspecified,
    Multicast,
    /// Link-local IPv6 needs a scope, which only the producing host can interpret.
    LinkLocal,
    /// The textual hint cannot carry a nonzero IPv6 flow label.
    NonzeroFlowinfo,
    /// A sender's interface index does not name an interface on the receiver.
    NonzeroScope,
}

impl IpHintIssue {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unspecified => "unspecified address",
            Self::Multicast => "multicast address",
            Self::LinkLocal => "link-local IPv6 address",
            Self::NonzeroFlowinfo => "nonzero IPv6 flowinfo",
            Self::NonzeroScope => "nonzero IPv6 scope id",
        }
    }
}

/// Admits unicast IPv4 and unscoped, flow-label-free unicast IPv6 (global, unique-local or
/// loopback).
// r[impl molten.fabric_transport.ip_hint_domain]
pub fn admit_ip_hint(address: &std::net::SocketAddr) -> Result<(), IpHintIssue> {
    if address.ip().is_unspecified() {
        return Err(IpHintIssue::Unspecified);
    }
    if address.ip().is_multicast() {
        return Err(IpHintIssue::Multicast);
    }
    if let std::net::SocketAddr::V6(address) = address {
        if address.flowinfo() != 0 {
            return Err(IpHintIssue::NonzeroFlowinfo);
        }
        if address.scope_id() != 0 {
            return Err(IpHintIssue::NonzeroScope);
        }
        if address.ip().is_unicast_link_local() {
            return Err(IpHintIssue::LinkLocal);
        }
    }
    Ok(())
}

/// Producers export only admitted hints: a local address outside the domain is not a usable
/// cross-host hint, so it is omitted rather than published in a lossy or host-local form.
pub fn is_exportable_hint(address: &iroh::TransportAddr) -> bool {
    match address {
        iroh::TransportAddr::Ip(address) => admit_ip_hint(address).is_ok(),
        _ => true,
    }
}

/// Parses the text after an `ip:` prefix and admits it; the error names the rejected field.
pub fn parse_ip_hint(text: &str) -> crate::error::Result<std::net::SocketAddr> {
    let address = text
        .parse::<std::net::SocketAddr>()
        .map_err(|error| crate::error::Failure::invalid_harness(format!("IP hint {text} does not parse: {error}")))?;
    admit_ip_hint(&address).map_err(|issue| {
        crate::error::Failure::invalid_harness(format!(
            "IP hint {text} is outside the exchanged-hint domain: {}",
            issue.as_str()
        ))
    })?;
    Ok(address)
}

#[cfg(test)]
mod tests {

    use super::*;

    fn v6(ip: &str, flowinfo: u32, scope_id: u32) -> std::net::SocketAddr {
        std::net::SocketAddr::V6(std::net::SocketAddrV6::new(ip.parse().expect("ipv6"), 7000, flowinfo, scope_id))
    }

    // r[verify molten.fabric_transport.ip_hint_domain]
    #[test]
    fn exchanged_hints_admit_only_unscoped_unicast_addresses() {
        let cases = [
            (
                std::net::SocketAddr::V4(std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(192, 0, 2, 7), 7000)),
                Ok(()),
            ),
            (std::net::SocketAddr::V4(std::net::SocketAddrV4::new(std::net::Ipv4Addr::LOCALHOST, 7000)), Ok(())),
            (
                std::net::SocketAddr::V4(std::net::SocketAddrV4::new(std::net::Ipv4Addr::UNSPECIFIED, 7000)),
                Err(IpHintIssue::Unspecified),
            ),
            (
                std::net::SocketAddr::V4(std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(224, 0, 0, 1), 7000)),
                Err(IpHintIssue::Multicast),
            ),
            (v6("2001:db8::7", 0, 0), Ok(())),
            (v6("fd00::7", 0, 0), Ok(())),
            (v6("::1", 0, 0), Ok(())),
            (v6("::", 0, 0), Err(IpHintIssue::Unspecified)),
            (v6("ff02::1", 0, 0), Err(IpHintIssue::Multicast)),
            (v6("fe80::7", 0, 0), Err(IpHintIssue::LinkLocal)),
            (v6("fe80::7", 0, 2), Err(IpHintIssue::NonzeroScope)),
            (v6("2001:db8::7", 0, 2), Err(IpHintIssue::NonzeroScope)),
            (v6("2001:db8::7", 9, 0), Err(IpHintIssue::NonzeroFlowinfo)),
        ];
        for (address, expected) in cases {
            assert_eq!(admit_ip_hint(&address), expected, "{address}");
        }
    }

    #[test]
    fn malformed_and_scoped_hint_text_is_rejected() {
        assert!(parse_ip_hint("[fe80::7%eth0]:7000").is_err(), "interface names are not numeric scopes");
        assert!(parse_ip_hint("[fe80::7%2]:7000").is_err());
        assert!(parse_ip_hint("[2001:db8::7]:7000").is_ok());
        assert!(parse_ip_hint("127.0.0.1:7000").is_ok());
        assert!(parse_ip_hint("not-an-address").is_err());
    }

    /// Host A's interface 2 is `eth0`; host B's interface 2 is `wlan0`. A hint carrying A's scope
    /// must not make B dial through `wlan0`, so the receiver rejects it instead of
    /// reinterpreting the index.
    // r[verify molten.fabric_transport.ip_hint_domain]
    #[test]
    fn a_senders_interface_index_never_selects_a_receiver_interface() {
        let sender_interfaces = [(2_u32, "eth0")];
        let receiver_interfaces = [(2_u32, "wlan0")];
        let hint = std::net::SocketAddr::V6(std::net::SocketAddrV6::new(
            "fe80::7".parse().expect("ipv6"),
            7000,
            0,
            sender_interfaces[0].0,
        ));
        let exported = iroh::TransportAddr::Ip(hint);
        assert!(!is_exportable_hint(&exported), "the sender omits it");
        let text = hint.to_string();
        assert!(text.ends_with("%2]:7000"), "{text}");
        let received = parse_ip_hint(&text);
        assert!(received.is_err(), "receiver would otherwise use {}", receiver_interfaces[0].1);
    }

    // r[verify molten.fabric_transport.ip_hint_domain]
    #[hegel::test(test_cases = 200)]
    fn admitted_hints_roundtrip_every_semantic_field(tc: hegel::TestCase) {
        let is_v6 = tc.draw(hegel::generators::booleans());
        let port = tc.draw(hegel::generators::integers::<u16>());
        let address = if is_v6 {
            let low = u128::from(tc.draw(hegel::generators::integers::<u64>()));
            let high = u128::from(tc.draw(hegel::generators::integers::<u64>()));
            // Bias toward the classes the domain treats specially, not only random global addresses.
            let bits = match tc.draw(hegel::generators::integers::<u8>().max_value(4)) {
                0 => (high << 64) | low,
                1 => (0xfe80_u128 << 112) | low,
                2 => (0xff02_u128 << 112) | low,
                3 => u128::from(std::net::Ipv6Addr::LOCALHOST),
                _ => 0,
            };
            let flowinfo = if tc.draw(hegel::generators::booleans()) {
                0
            } else {
                tc.draw(hegel::generators::integers::<u32>())
            };
            let scope_id = if tc.draw(hegel::generators::booleans()) {
                0
            } else {
                tc.draw(hegel::generators::integers::<u32>())
            };
            std::net::SocketAddr::V6(std::net::SocketAddrV6::new(
                std::net::Ipv6Addr::from(bits),
                port,
                flowinfo,
                scope_id,
            ))
        } else {
            std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::from(tc.draw(hegel::generators::integers::<u32>())),
                port,
            ))
        };
        let exported = iroh::TransportAddr::Ip(address);
        match admit_ip_hint(&address) {
            Ok(()) => {
                assert!(is_exportable_hint(&exported));
                let text = exported.to_string();
                let parsed = parse_ip_hint(text.strip_prefix("ip:").expect("ip prefix")).expect("admitted hint parses");
                assert_eq!(parsed.ip(), address.ip());
                assert_eq!(parsed.port(), address.port());
                if let (std::net::SocketAddr::V6(parsed), std::net::SocketAddr::V6(original)) = (parsed, address) {
                    assert_eq!(parsed.flowinfo(), original.flowinfo());
                    assert_eq!(parsed.scope_id(), original.scope_id());
                }
            }
            Err(issue) => {
                assert!(!is_exportable_hint(&exported));
                let ip = address.ip();
                let is_valid = match (issue, address) {
                    (IpHintIssue::Unspecified, _) => ip.is_unspecified(),
                    (IpHintIssue::Multicast, _) => ip.is_multicast(),
                    (IpHintIssue::NonzeroFlowinfo, std::net::SocketAddr::V6(v6)) => v6.flowinfo() != 0,
                    (IpHintIssue::NonzeroScope, std::net::SocketAddr::V6(v6)) => v6.scope_id() != 0,
                    (IpHintIssue::LinkLocal, std::net::SocketAddr::V6(v6)) => v6.ip().is_unicast_link_local(),
                    _ => false,
                };
                assert!(is_valid, "{issue:?} does not describe {address}");
            }
        }
    }
}
