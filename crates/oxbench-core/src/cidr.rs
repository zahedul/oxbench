//! IPv4 and IPv6 network maths: what a network covers, splitting it, and merging lists of them.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

pub use ipnet::IpNet;
use ipnet::{Ipv4Subnets, Ipv6Subnets, ip_mask_to_prefix};

use crate::{CoreError, CoreResult};

/// Splits list at most this many subnets; more would only slow the list down.
pub const MAX_LISTED_SUBNETS: usize = 1024;

/// 2^128 has no `u128`, so its digits are written out.
const ALL_IPV6_ADDRESSES: &str = "340,282,366,920,938,463,463,374,607,431,768,211,456";

/// Counts this large also show as a power of two, which is easier to read.
const MIN_BITS_FOR_POWER_NOTATION: u8 = 20;

const IPV4_SPECIAL_RANGES: &[(&str, &str)] = &[
    ("0.0.0.0/8", "This network"),
    ("10.0.0.0/8", "Private (RFC 1918)"),
    (
        "100.64.0.0/10",
        "Shared address space for carrier NAT (RFC 6598)",
    ),
    ("127.0.0.0/8", "Loopback"),
    ("169.254.0.0/16", "Link-local"),
    ("172.16.0.0/12", "Private (RFC 1918)"),
    ("192.0.0.0/24", "IETF protocol assignments"),
    ("192.0.2.0/24", "Documentation (TEST-NET-1)"),
    ("192.168.0.0/16", "Private (RFC 1918)"),
    ("198.18.0.0/15", "Benchmarking (RFC 2544)"),
    ("198.51.100.0/24", "Documentation (TEST-NET-2)"),
    ("203.0.113.0/24", "Documentation (TEST-NET-3)"),
    ("224.0.0.0/4", "Multicast"),
    ("255.255.255.255/32", "Limited broadcast"),
    ("240.0.0.0/4", "Reserved"),
];

const IPV6_SPECIAL_RANGES: &[(&str, &str)] = &[
    ("::/128", "Unspecified"),
    ("::1/128", "Loopback"),
    ("::ffff:0:0/96", "IPv4-mapped"),
    ("64:ff9b::/96", "IPv4/IPv6 translation (NAT64)"),
    ("2001:db8::/32", "Documentation"),
    ("2002::/16", "6to4"),
    ("fc00::/7", "Unique local (private)"),
    ("fe80::/10", "Link-local"),
    ("ff00::/8", "Multicast"),
    ("2000::/3", "Global unicast"),
];

/// An address together with the network it was written in, such as `192.168.1.10/24`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Interface {
    pub address: IpAddr,
    /// With the host bits cleared.
    pub network: IpNet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkDetails {
    pub interface: Interface,
    pub kind: &'static str,
    pub netmask: IpAddr,
    pub wildcard: IpAddr,
    pub first_address: IpAddr,
    /// The broadcast address in IPv4.
    pub last_address: IpAddr,
    /// The addresses hosts can use: all but the network and broadcast addresses in IPv4,
    /// except in /31 and /32 networks.
    pub first_host: IpAddr,
    pub last_host: IpAddr,
    pub total_addresses: String,
    pub usable_hosts: String,
    /// The `in-addr.arpa` or `ip6.arpa` zone that holds the network's reverse DNS.
    pub reverse_zone: String,
    pub integer: String,
    pub hex: String,
    /// The historic class, IPv4 only.
    pub class: Option<char>,
    /// All eight-digit groups, IPv6 only.
    pub expanded: Option<String>,
}

/// Whether a checked address or network falls inside another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Containment {
    pub checked: Interface,
    pub is_inside: bool,
    /// Which address of the network it is, counting the network address as 0.
    pub offset: Option<u128>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitPlan {
    /// The first `MAX_LISTED_SUBNETS` at most.
    pub subnets: Vec<IpNet>,
    pub subnet_count: String,
    pub addresses_each: String,
    pub is_truncated: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MergeResult {
    pub networks: Vec<IpNet>,
    /// How many entries were read; a range counts as one.
    pub input_count: usize,
    /// One message per line that couldn't be read, such as "Line 3: …".
    pub problems: Vec<String>,
}

/// Reads `192.168.1.10/24`, `192.168.1.10/255.255.255.0`, `192.168.1.10 255.255.255.0`,
/// `2001:db8::1/64`, or a bare address, which counts as a single-address network.
pub fn parse_interface(input: &str) -> CoreResult<Interface> {
    let text = input.trim();
    if text.is_empty() {
        return Err(invalid("Enter an address, such as 192.168.1.10/24."));
    }
    let (address_text, suffix) = match text.split_once('/') {
        Some((address, suffix)) => (address.trim(), Some(suffix.trim())),
        None => match text.split_once(char::is_whitespace) {
            Some((address, mask)) => (address, Some(mask.trim())),
            None => (text, None),
        },
    };
    let address = IpAddr::from_str(address_text)
        .map_err(|_| invalid(format!("\"{address_text}\" isn't an IP address.")))?;
    let prefix_len = match suffix {
        Some(suffix) => parse_prefix_or_mask(address, suffix)?,
        None => max_prefix_len(address),
    };
    let network = IpNet::new(address, prefix_len)
        .map_err(|_| invalid(format!("/{prefix_len} is too long for {address}.")))?
        .trunc();
    Ok(Interface { address, network })
}

pub fn describe_network(interface: Interface) -> NetworkDetails {
    let network = interface.network;
    let host_bits = network.max_prefix_len() - network.prefix_len();
    let (first_host, last_host, usable_hosts) = find_host_range(network, interface.address);
    let (integer, hex) = match interface.address {
        IpAddr::V4(address) => {
            let value = u32::from(address);
            (value.to_string(), format!("0x{value:08X}"))
        }
        IpAddr::V6(address) => {
            let value = u128::from(address);
            (value.to_string(), format!("0x{value:032X}"))
        }
    };
    NetworkDetails {
        interface,
        kind: classify_network(network),
        netmask: network.netmask(),
        wildcard: network.hostmask(),
        first_address: network.network(),
        last_address: network.broadcast(),
        first_host,
        last_host,
        total_addresses: format_address_count(host_bits, 0),
        usable_hosts,
        reverse_zone: format_reverse_zone(network),
        integer,
        hex,
        class: match interface.address {
            IpAddr::V4(address) => Some(find_class(address)),
            IpAddr::V6(_) => None,
        },
        expanded: match interface.address {
            IpAddr::V4(_) => None,
            IpAddr::V6(address) => Some(expand_ipv6(address)),
        },
    }
}

/// Checks whether `input`, an address or a network, falls inside `network`.
pub fn check_containment(network: IpNet, input: &str) -> CoreResult<Containment> {
    let checked = parse_interface(input)?;
    let is_inside = network.contains(&checked.network);
    let offset =
        is_inside.then(|| address_value(checked.address) - address_value(network.network()));
    Ok(Containment {
        checked,
        is_inside,
        offset,
    })
}

/// Splits `network` into equal subnets with the longer prefix `new_prefix_len`.
pub fn split_network(network: IpNet, new_prefix_len: u8) -> CoreResult<SplitPlan> {
    let subnets = network.subnets(new_prefix_len).map_err(|_| {
        invalid(format!(
            "Subnets of {network} need a prefix from /{} to /{}.",
            network.prefix_len(),
            network.max_prefix_len()
        ))
    })?;
    let split_bits = new_prefix_len - network.prefix_len();
    let host_bits = network.max_prefix_len() - new_prefix_len;
    let is_truncated = 1_usize
        .checked_shl(u32::from(split_bits))
        .is_none_or(|count| count > MAX_LISTED_SUBNETS);
    Ok(SplitPlan {
        subnets: subnets.take(MAX_LISTED_SUBNETS).collect(),
        subnet_count: format_address_count(split_bits, 0),
        addresses_each: format_address_count(host_bits, 0),
        is_truncated,
    })
}

/// Merges networks, addresses and `start - end` ranges, one or more per line separated by
/// commas, into the fewest networks that cover exactly the same addresses.
pub fn merge_networks(input: &str) -> MergeResult {
    let mut result = MergeResult::default();
    let mut networks = Vec::new();
    for (index, line) in input.lines().enumerate() {
        let items = line
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty() && !item.starts_with('#'));
        for item in items {
            match parse_merge_item(item) {
                Ok(parsed) => {
                    result.input_count += 1;
                    networks.extend(parsed);
                }
                Err(error) => result.problems.push(format!("Line {}: {error}", index + 1)),
            }
        }
    }
    result.networks = IpNet::aggregate(&networks);
    result
}

fn parse_merge_item(item: &str) -> CoreResult<Vec<IpNet>> {
    let Some((start, end)) = item.split_once('-') else {
        return Ok(vec![parse_interface(item)?.network]);
    };
    let parse_address = |text: &str| {
        IpAddr::from_str(text.trim())
            .map_err(|_| invalid(format!("\"{}\" isn't an IP address.", text.trim())))
    };
    let (start, end) = (parse_address(start)?, parse_address(end)?);
    if start.is_ipv4() != end.is_ipv4() {
        return Err(invalid(format!(
            "The range {start} - {end} mixes IPv4 and IPv6."
        )));
    }
    if address_value(start) > address_value(end) {
        return Err(invalid(format!(
            "The range {start} - {end} runs backwards."
        )));
    }
    match (start, end) {
        (IpAddr::V4(start), IpAddr::V4(end)) => {
            Ok(Ipv4Subnets::new(start, end, 0).map(IpNet::V4).collect())
        }
        (IpAddr::V6(start), IpAddr::V6(end)) => {
            Ok(Ipv6Subnets::new(start, end, 0).map(IpNet::V6).collect())
        }
        // Ruled out above; kept so the match needs no panic.
        _ => Ok(Vec::new()),
    }
}

fn parse_prefix_or_mask(address: IpAddr, suffix: &str) -> CoreResult<u8> {
    let max = max_prefix_len(address);
    if let Ok(prefix_len) = suffix.parse::<u8>() {
        if prefix_len > max {
            return Err(invalid(format!(
                "/{prefix_len} is too long: {} prefixes go up to /{max}.",
                family_name(address)
            )));
        }
        return Ok(prefix_len);
    }
    let mask = IpAddr::from_str(suffix)
        .map_err(|_| invalid(format!("\"{suffix}\" isn't a prefix length or a netmask.")))?;
    if mask.is_ipv4() != address.is_ipv4() {
        return Err(invalid(format!(
            "The netmask {mask} isn't {}, like the address.",
            family_name(address)
        )));
    }
    ip_mask_to_prefix(mask).map_err(|_| {
        invalid(format!(
            "{mask} isn't a valid netmask: its one bits must all come first."
        ))
    })
}

const fn max_prefix_len(address: IpAddr) -> u8 {
    match address {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    }
}

const fn family_name(address: IpAddr) -> &'static str {
    match address {
        IpAddr::V4(_) => "IPv4",
        IpAddr::V6(_) => "IPv6",
    }
}

fn address_value(address: IpAddr) -> u128 {
    match address {
        IpAddr::V4(address) => u128::from(u32::from(address)),
        IpAddr::V6(address) => u128::from(address),
    }
}

fn find_host_range(network: IpNet, address: IpAddr) -> (IpAddr, IpAddr, String) {
    let IpNet::V4(network_v4) = network else {
        let host_bits = network.max_prefix_len() - network.prefix_len();
        return (
            network.network(),
            network.broadcast(),
            format_address_count(host_bits, 0),
        );
    };
    match network_v4.prefix_len() {
        32 => (address, address, "1".to_owned()),
        // Point-to-point links use both addresses (RFC 3021).
        31 => (network.network(), network.broadcast(), "2".to_owned()),
        prefix_len => {
            let first = Ipv4Addr::from(u32::from(network_v4.network()) + 1);
            let last = Ipv4Addr::from(u32::from(network_v4.broadcast()) - 1);
            let usable = format_address_count(32 - prefix_len, 2);
            (IpAddr::V4(first), IpAddr::V4(last), usable)
        }
    }
}

fn classify_network(network: IpNet) -> &'static str {
    let ranges = match network {
        IpNet::V4(_) => IPV4_SPECIAL_RANGES,
        IpNet::V6(_) => IPV6_SPECIAL_RANGES,
    };
    let parsed = ranges
        .iter()
        .filter_map(|(range, label)| Some((IpNet::from_str(range).ok()?, *label)));
    let mut overlaps_special_range = false;
    for (range, label) in parsed {
        if range.contains(&network) {
            return label;
        }
        overlaps_special_range |= network.contains(&range);
    }
    match (network, overlaps_special_range) {
        (_, true) => "Mixed: includes special-purpose ranges",
        (IpNet::V4(_), false) => "Public",
        (IpNet::V6(_), false) => "Reserved",
    }
}

fn find_class(address: Ipv4Addr) -> char {
    match address.octets()[0] {
        0..=127 => 'A',
        128..=191 => 'B',
        192..=223 => 'C',
        224..=239 => 'D',
        240..=255 => 'E',
    }
}

/// The zone covering the network, rounded out to whole octets or hex digits as DNS delegates them.
fn format_reverse_zone(network: IpNet) -> String {
    let mut labels: Vec<String> = match network {
        IpNet::V4(network) => {
            let octet_count = usize::from(network.prefix_len() / 8);
            network.network().octets()[..octet_count]
                .iter()
                .map(u8::to_string)
                .collect()
        }
        IpNet::V6(network) => {
            let nibble_count = usize::from(network.prefix_len() / 4);
            format!("{:032x}", u128::from(network.network()))
                .chars()
                .take(nibble_count)
                .map(String::from)
                .collect()
        }
    };
    labels.reverse();
    labels.push(
        match network {
            IpNet::V4(_) => "in-addr.arpa",
            IpNet::V6(_) => "ip6.arpa",
        }
        .to_owned(),
    );
    labels.join(".")
}

fn expand_ipv6(address: Ipv6Addr) -> String {
    address
        .segments()
        .iter()
        .map(|segment| format!("{segment:04x}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// `2^bits - minus`, with digit grouping, plus the power of two for large counts.
fn format_address_count(bits: u8, minus: u128) -> String {
    let Some(total) = 1_u128.checked_shl(u32::from(bits)) else {
        return format!("{ALL_IPV6_ADDRESSES} (2^{bits})");
    };
    let grouped = group_digits(total.saturating_sub(minus));
    if bits < MIN_BITS_FOR_POWER_NOTATION || minus > 0 {
        return grouped;
    }
    format!("{grouped} (2^{bits})")
}

fn group_digits(value: u128) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        let is_group_start = index > 0 && (digits.len() - index).is_multiple_of(3);
        if is_group_start {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details(input: &str) -> NetworkDetails {
        describe_network(parse_interface(input).unwrap())
    }

    fn net(text: &str) -> IpNet {
        text.parse().unwrap()
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    #[test]
    fn reads_prefixes_masks_and_bare_addresses() {
        let expected = Interface {
            address: ip("192.168.1.10"),
            network: net("192.168.1.0/24"),
        };
        assert_eq!(parse_interface("192.168.1.10/24").unwrap(), expected);
        assert_eq!(
            parse_interface(" 192.168.1.10/255.255.255.0 ").unwrap(),
            expected
        );
        assert_eq!(
            parse_interface("192.168.1.10 255.255.255.0").unwrap(),
            expected
        );
        assert_eq!(
            parse_interface("10.0.0.1").unwrap().network,
            net("10.0.0.1/32")
        );
        assert_eq!(
            parse_interface("2001:db8::1/64").unwrap().network,
            net("2001:db8::/64")
        );
    }

    #[test]
    fn explains_bad_input() {
        let message = |input| parse_interface(input).unwrap_err().to_string();
        assert_eq!(message(""), "Enter an address, such as 192.168.1.10/24.");
        assert_eq!(message("300.1.1.1/8"), "\"300.1.1.1\" isn't an IP address.");
        assert_eq!(
            message("10.0.0.0/33"),
            "/33 is too long: IPv4 prefixes go up to /32."
        );
        assert_eq!(
            message("10.0.0.0/255.0.255.0"),
            "255.0.255.0 isn't a valid netmask: its one bits must all come first."
        );
        assert_eq!(
            message("10.0.0.0/ffff::"),
            "The netmask ffff:: isn't IPv4, like the address."
        );
        assert_eq!(
            message("10.0.0.0/abc"),
            "\"abc\" isn't a prefix length or a netmask."
        );
    }

    #[test]
    fn describes_an_ipv4_network() {
        let details = details("192.168.1.10/24");
        assert_eq!(details.kind, "Private (RFC 1918)");
        assert_eq!(details.netmask, ip("255.255.255.0"));
        assert_eq!(details.wildcard, ip("0.0.0.255"));
        assert_eq!(details.first_address, ip("192.168.1.0"));
        assert_eq!(details.last_address, ip("192.168.1.255"));
        assert_eq!(details.first_host, ip("192.168.1.1"));
        assert_eq!(details.last_host, ip("192.168.1.254"));
        assert_eq!(details.total_addresses, "256");
        assert_eq!(details.usable_hosts, "254");
        assert_eq!(details.reverse_zone, "1.168.192.in-addr.arpa");
        assert_eq!(details.integer, "3232235786");
        assert_eq!(details.hex, "0xC0A8010A");
        assert_eq!(details.class, Some('C'));
        assert_eq!(details.expanded, None);
    }

    #[test]
    fn point_to_point_and_single_host_networks_use_every_address() {
        let link = details("10.0.0.0/31");
        assert_eq!(
            (link.first_host, link.last_host),
            (ip("10.0.0.0"), ip("10.0.0.1"))
        );
        assert_eq!(link.usable_hosts, "2");
        let host = details("10.0.0.7");
        assert_eq!(
            (host.first_host, host.last_host),
            (ip("10.0.0.7"), ip("10.0.0.7"))
        );
        assert_eq!(host.usable_hosts, "1");
    }

    #[test]
    fn large_counts_are_grouped_and_show_their_power_of_two() {
        let details = details("10.0.0.0/8");
        assert_eq!(details.total_addresses, "16,777,216 (2^24)");
        assert_eq!(details.usable_hosts, "16,777,214");
        assert_eq!(
            describe_network(parse_interface("::/0").unwrap()).total_addresses,
            "340,282,366,920,938,463,463,374,607,431,768,211,456 (2^128)"
        );
    }

    #[test]
    fn describes_an_ipv6_network() {
        let details = details("2001:db8:abcd:12::1/64");
        assert_eq!(details.kind, "Documentation");
        assert_eq!(details.first_address, ip("2001:db8:abcd:12::"));
        assert_eq!(
            details.last_address,
            ip("2001:db8:abcd:12:ffff:ffff:ffff:ffff")
        );
        assert_eq!(details.first_host, details.first_address);
        assert_eq!(details.total_addresses, "18,446,744,073,709,551,616 (2^64)");
        assert_eq!(
            details.reverse_zone,
            "2.1.0.0.d.c.b.a.8.b.d.0.1.0.0.2.ip6.arpa"
        );
        assert_eq!(
            details.expanded.as_deref(),
            Some("2001:0db8:abcd:0012:0000:0000:0000:0001")
        );
        assert_eq!(details.class, None);
    }

    #[test]
    fn classifies_special_and_public_ranges() {
        let kind = |input| details(input).kind;
        assert_eq!(kind("8.8.8.8"), "Public");
        assert_eq!(kind("127.0.0.1"), "Loopback");
        assert_eq!(
            kind("100.64.1.1"),
            "Shared address space for carrier NAT (RFC 6598)"
        );
        assert_eq!(kind("255.255.255.255"), "Limited broadcast");
        assert_eq!(kind("0.0.0.0/0"), "Mixed: includes special-purpose ranges");
        assert_eq!(kind("fe80::1"), "Link-local");
        assert_eq!(kind("fd12::/48"), "Unique local (private)");
        assert_eq!(kind("2606:4700::1111"), "Global unicast");
    }

    #[test]
    fn reverse_zones_round_out_to_whole_octets() {
        assert_eq!(details("10.20.0.0/12").reverse_zone, "10.in-addr.arpa");
        assert_eq!(details("10.0.0.0/7").reverse_zone, "in-addr.arpa");
        assert_eq!(details("192.0.2.5").reverse_zone, "5.2.0.192.in-addr.arpa");
    }

    #[test]
    fn checks_whether_addresses_and_networks_are_inside() {
        let network = net("192.168.1.0/24");
        let inside = check_containment(network, "192.168.1.77").unwrap();
        assert!(inside.is_inside);
        assert_eq!(inside.offset, Some(77));
        let subnet = check_containment(network, "192.168.1.128/25").unwrap();
        assert!(subnet.is_inside);
        let wider = check_containment(network, "192.168.0.0/16").unwrap();
        assert!(!wider.is_inside);
        assert_eq!(wider.offset, None);
        assert!(!check_containment(network, "::1").unwrap().is_inside);
        assert!(check_containment(network, "nope").is_err());
    }

    #[test]
    fn splits_into_equal_subnets() {
        let plan = split_network(net("192.168.1.0/24"), 26).unwrap();
        assert_eq!(
            plan.subnets,
            [
                net("192.168.1.0/26"),
                net("192.168.1.64/26"),
                net("192.168.1.128/26"),
                net("192.168.1.192/26"),
            ]
        );
        assert_eq!(plan.subnet_count, "4");
        assert_eq!(plan.addresses_each, "64");
        assert!(!plan.is_truncated);
    }

    #[test]
    fn long_splits_list_only_the_first_subnets() {
        let plan = split_network(net("10.0.0.0/8"), 24).unwrap();
        assert_eq!(plan.subnets.len(), MAX_LISTED_SUBNETS);
        assert_eq!(plan.subnet_count, "65,536");
        assert!(plan.is_truncated);
        assert!(!split_network(net("10.0.0.0/8"), 18).unwrap().is_truncated);
        let whole = split_network(net("::/0"), 128).unwrap();
        assert!(whole.is_truncated);
        assert!(split_network(net("10.0.0.0/8"), 4).is_err());
    }

    #[test]
    fn merges_neighbours_and_drops_covered_networks() {
        let result = merge_networks(
            "192.168.0.0/24\n192.168.1.0/24, 192.168.1.128/25\n# a comment\n\n10.0.0.1\n10.0.0.0/32",
        );
        assert_eq!(result.networks, [net("10.0.0.0/31"), net("192.168.0.0/23")]);
        assert_eq!(result.input_count, 5);
        assert_eq!(result.problems, Vec::<String>::new());
    }

    #[test]
    fn merges_ranges_into_the_networks_that_cover_them() {
        let result = merge_networks("10.0.0.5 - 10.0.0.12");
        assert_eq!(
            result.networks,
            [
                net("10.0.0.5/32"),
                net("10.0.0.6/31"),
                net("10.0.0.8/30"),
                net("10.0.0.12/32"),
            ]
        );
    }

    #[test]
    fn reports_each_unreadable_line() {
        let result = merge_networks("10.0.0.0/8\nbanana\n10.0.0.9 - 10.0.0.1\n10.0.0.1 - ::1");
        assert_eq!(result.networks, [net("10.0.0.0/8")]);
        assert_eq!(
            result.problems,
            [
                "Line 2: \"banana\" isn't an IP address.",
                "Line 3: The range 10.0.0.9 - 10.0.0.1 runs backwards.",
                "Line 4: The range 10.0.0.1 - ::1 mixes IPv4 and IPv6.",
            ]
        );
    }
}
