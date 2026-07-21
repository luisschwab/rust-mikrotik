//! Shared parsing and address-arithmetic helpers.

use alloc::vec::Vec;
use core::net::IpAddr;
use core::net::Ipv4Addr;
use core::net::Ipv6Addr;

/// Parse a plain IPv4 or IPv6 prefix into its address and prefix length.
#[must_use]
pub fn parse_ip_prefix(value: &str) -> Option<(IpAddr, u8)> {
    let (address, prefix_length) = value.trim().split_once('/')?;
    let address = address.parse::<IpAddr>().ok()?;
    let prefix_length = prefix_length.parse::<u8>().ok()?;
    (prefix_length <= maximum_prefix_length(address)).then_some((address, prefix_length))
}

/// Return the maximum prefix length for an IP address family.
#[must_use]
pub const fn maximum_prefix_length(address: IpAddr) -> u8 {
    match address {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    }
}

/// Mask an IP address to a prefix's canonical network address.
#[must_use]
pub fn network_address(address: IpAddr, prefix_length: u8) -> Option<IpAddr> {
    match address {
        IpAddr::V4(address) => {
            if prefix_length > 32 {
                return None;
            }
            let mask = if prefix_length == 0 {
                0
            } else {
                u32::MAX << (32 - u32::from(prefix_length))
            };
            Some(IpAddr::V4(Ipv4Addr::from(u32::from(address) & mask)))
        }
        IpAddr::V6(address) => {
            if prefix_length > 128 {
                return None;
            }
            let mask = if prefix_length == 0 {
                0
            } else {
                u128::MAX << (128 - u128::from(prefix_length))
            };
            Some(IpAddr::V6(Ipv6Addr::from(u128::from(address) & mask)))
        }
    }
}

/// Return whether a prefix contains an IP address.
#[must_use]
pub fn prefix_contains(network: IpAddr, prefix_length: u8, address: IpAddr) -> bool {
    network.is_ipv4() == address.is_ipv4()
        && network_address(address, prefix_length).is_some_and(|candidate| candidate == network)
}

/// Return whether two canonical or non-canonical prefixes overlap.
#[must_use]
pub fn prefixes_overlap(left: (IpAddr, u8), right: (IpAddr, u8)) -> bool {
    if left.0.is_ipv4() != right.0.is_ipv4() {
        return false;
    }
    let Some(left_network) = network_address(left.0, left.1) else {
        return false;
    };
    let Some(right_network) = network_address(right.0, right.1) else {
        return false;
    };
    prefix_contains(left_network, left.1, right_network) || prefix_contains(right_network, right.1, left_network)
}

/// Convert one inclusive, same-family IP address range into a minimal prefix cover.
#[must_use]
pub fn range_to_prefixes(start: IpAddr, end: IpAddr) -> Vec<(IpAddr, u8)> {
    if start.is_ipv4() != end.is_ipv4() {
        return Vec::new();
    }
    let bits = u32::from(maximum_prefix_length(start));
    let mut current = ip_number(start);
    let end = ip_number(end);
    if current > end {
        return Vec::new();
    }

    let mut prefixes = Vec::new();
    loop {
        let alignment = if current == 0 {
            bits
        } else {
            current.trailing_zeros().min(bits)
        };
        let remaining = end - current;
        let remaining_bits = if remaining == u128::MAX {
            128
        } else {
            127 - (remaining + 1).leading_zeros()
        };
        let host_bits = alignment.min(remaining_bits);
        // Both address families have at most 128 bits, so this difference always fits in `u8`.
        let prefix_length = (bits - host_bits) as u8;
        prefixes.push((number_ip(current, start), prefix_length));
        if host_bits == 128 {
            break;
        }
        let block_size = 1_u128 << host_bits;
        let Some(next) = current.checked_add(block_size) else {
            break;
        };
        if next > end {
            break;
        }
        current = next;
    }
    prefixes
}

/// Convert an IP address into its family-local numeric value.
fn ip_number(address: IpAddr) -> u128 {
    match address {
        IpAddr::V4(address) => u128::from(u32::from(address)),
        IpAddr::V6(address) => u128::from(address),
    }
}

/// Convert a family-local numeric address back into the selected family.
fn number_ip(value: u128, family: IpAddr) -> IpAddr {
    match family {
        IpAddr::V4(_) => {
            // IPv4 ranges originate as `u32` values and cannot cross the IPv4 maximum.
            let bytes = value.to_be_bytes();
            IpAddr::V4(Ipv4Addr::new(bytes[12], bytes[13], bytes[14], bytes[15]))
        }
        IpAddr::V6(_) => IpAddr::V6(value.into()),
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn parses_and_masks_ipv4_and_ipv6_prefixes() {
        assert_eq!(
            parse_ip_prefix("192.0.2.9/29"),
            Some(("192.0.2.9".parse().unwrap(), 29))
        );
        assert_eq!(
            network_address("192.0.2.9".parse().unwrap(), 29),
            Some("192.0.2.8".parse().unwrap())
        );
        assert_eq!(
            network_address("2001:db8::3".parse().unwrap(), 126),
            Some("2001:db8::".parse().unwrap())
        );
        assert!(parse_ip_prefix("192.0.2.1/33").is_none());
        assert!(parse_ip_prefix("2001:db8::1/129").is_none());
    }

    #[test]
    fn detects_prefix_containment_and_overlap() {
        let network = "192.0.2.0".parse().unwrap();
        assert!(prefix_contains(network, 24, "192.0.2.200".parse().unwrap()));
        assert!(!prefix_contains(network, 24, "192.0.3.1".parse().unwrap()));
        assert!(prefixes_overlap((network, 24), ("192.0.2.128".parse().unwrap(), 25)));
        assert!(!prefixes_overlap((network, 24), ("2001:db8::".parse().unwrap(), 64)));
    }

    #[test]
    fn converts_inclusive_ranges_to_minimal_prefixes() {
        assert_eq!(
            range_to_prefixes("192.0.2.0".parse().unwrap(), "192.0.2.7".parse().unwrap()),
            vec![("192.0.2.0".parse().unwrap(), 29)]
        );
        assert_eq!(
            range_to_prefixes("192.0.2.1".parse().unwrap(), "192.0.2.6".parse().unwrap()),
            vec![
                ("192.0.2.1".parse().unwrap(), 32),
                ("192.0.2.2".parse().unwrap(), 31),
                ("192.0.2.4".parse().unwrap(), 31),
                ("192.0.2.6".parse().unwrap(), 32),
            ]
        );
        assert!(range_to_prefixes("192.0.2.1".parse().unwrap(), "2001:db8::1".parse().unwrap()).is_empty());
    }
}
