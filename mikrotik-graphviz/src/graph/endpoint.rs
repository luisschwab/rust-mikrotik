use core::net::IpAddr;
use core::net::Ipv6Addr;
use std::collections::BTreeSet;

use mikrotik_types::abstractions::LinkKind;
use mikrotik_types::abstractions::Subnet;
use mikrotik_types::device::DeviceRole;
use mikrotik_types::device::TopologyNodeKey;
use mikrotik_types::primitives::interface::InterfaceName;

use super::address_index::GraphAddressIndex;
use super::edge::GraphvizEdge;
use super::model::NetworkGraph;
use super::node::graphviz_node_label;
use super::node::graphviz_node_role;

/// One endpoint row in a Graphviz link table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EdgeEndpointLabel {
    /// Device name.
    pub(super) device: String,
    /// Interface name.
    pub(super) interface: String,
    /// Interface address prefixes.
    pub(super) addresses: Vec<String>,
}

/// Return endpoint labels for one visual edge using only addresses relevant to that link.
pub(super) fn edge_endpoint_labels(
    edge: &GraphvizEdge,
    address_index: &GraphAddressIndex,
    graph: &NetworkGraph,
) -> (Option<EdgeEndpointLabel>, Option<EdgeEndpointLabel>) {
    let local_addresses = relevant_endpoint_addresses(
        &edge.local_node,
        edge.local_interface.as_ref(),
        &edge.remote_node,
        edge.remote_interface.as_ref(),
        edge.link_kind,
        address_index,
        graph,
    );
    let remote_addresses = relevant_endpoint_addresses(
        &edge.remote_node,
        edge.remote_interface.as_ref(),
        &edge.local_node,
        edge.local_interface.as_ref(),
        edge.link_kind,
        address_index,
        graph,
    );

    (
        edge_endpoint_label(&edge.local_node, edge.local_interface.as_ref(), graph, local_addresses),
        edge_endpoint_label(
            &edge.remote_node,
            edge.remote_interface.as_ref(),
            graph,
            remote_addresses,
        ),
    )
}

/// Return one endpoint label row including interface and assigned IP prefixes.
fn edge_endpoint_label(
    node: &TopologyNodeKey,
    interface: Option<&InterfaceName>,
    graph: &NetworkGraph,
    relevant_addresses: Vec<String>,
) -> Option<EdgeEndpointLabel> {
    if interface.is_none() && node.as_str().starts_with("bgp:") {
        return Some(EdgeEndpointLabel {
            device: graphviz_node_label(node, graph).unwrap_or_else(|| node.to_string()),
            interface: String::new(),
            addresses: vec![node.as_str().trim_start_matches("bgp:").to_owned()],
        });
    }
    if interface.is_none() {
        if let Some(address) = graph
            .nodes
            .iter()
            .find(|candidate| candidate.key == *node)
            .and_then(|candidate| candidate.inferred.as_ref())
            .and_then(|inferred| inferred.management_address)
        {
            return Some(EdgeEndpointLabel {
                device: graphviz_node_label(node, graph).unwrap_or_else(|| node.to_string()),
                interface: String::new(),
                addresses: vec![address.to_string()],
            });
        }
    }
    let interface = interface?;
    let interface = interface.to_string();
    let mut addresses = relevant_addresses;
    if addresses.is_empty() {
        if let Some(address) = graph
            .nodes
            .iter()
            .find(|candidate| candidate.key == *node)
            .and_then(|candidate| candidate.inferred.as_ref())
            .and_then(|inferred| inferred.management_address)
        {
            addresses.push(address.to_string());
        }
    }
    Some(EdgeEndpointLabel {
        device: graphviz_node_label(node, graph).unwrap_or_else(|| node.to_string()),
        interface,
        addresses,
    })
}

/// Return addresses worth displaying for one endpoint of a link.
fn relevant_endpoint_addresses(
    node: &TopologyNodeKey,
    interface: Option<&InterfaceName>,
    peer_node: &TopologyNodeKey,
    peer_interface: Option<&InterfaceName>,
    link_kind: LinkKind,
    address_index: &GraphAddressIndex,
    graph: &NetworkGraph,
) -> Vec<String> {
    let mut addresses = Vec::new();
    let management_address = node_management_address(node, graph);

    let Some(interface) = interface else {
        if let Some(address) = management_address {
            push_unique_address(&mut addresses, address);
        }
        return addresses.into_iter().collect();
    };
    let interface = interface.to_string();
    let endpoint_addresses = address_index.addresses(node, &interface);
    let peer_addresses = peer_interface.map_or([].as_slice(), |interface| {
        let interface = interface.to_string();
        address_index.addresses(peer_node, &interface)
    });
    let shared_networks = shared_link_networks(endpoint_addresses, peer_addresses);
    let include_public_allocations = link_kind == LinkKind::Customer
        || graphviz_node_role(node, graph) == Some(DeviceRole::CustomerRouter)
        || graphviz_node_role(peer_node, graph) == Some(DeviceRole::CustomerRouter);

    for address in endpoint_addresses {
        let shared_link_address =
            Subnet::from_prefix(address.as_str()).is_some_and(|network| shared_networks.contains(&network));
        if shared_link_address || (include_public_allocations && is_public_prefix(address)) {
            push_unique_address(&mut addresses, address.clone());
        }
    }

    if let Some(address) = management_address {
        if !addresses.iter().any(|prefix| prefix_host_equals(prefix, &address)) {
            addresses.insert(0, address);
        }
    }

    addresses
}

/// Add an address if it has not already been included.
fn push_unique_address(addresses: &mut Vec<String>, address: String) {
    if !addresses.iter().any(|existing| existing == &address) {
        addresses.push(address);
    }
}

/// Return true when a prefix host is equal to an address string.
fn prefix_host_equals(prefix: &str, address: &str) -> bool {
    prefix.split_once('/').map_or(prefix, |(host, _length)| host) == address
}

/// Return link networks that appear on both endpoints.
fn shared_link_networks(left: &[String], right: &[String]) -> BTreeSet<Subnet> {
    let right_networks = right
        .iter()
        .filter_map(|address| Subnet::from_prefix(address))
        .collect::<BTreeSet<_>>();
    left.iter()
        .filter_map(|address| Subnet::from_prefix(address))
        .filter(|network| network.is_link_network())
        .filter(|network| right_networks.contains(network))
        .collect()
}

/// Return the management address known for one graph node.
fn node_management_address(node: &TopologyNodeKey, graph: &NetworkGraph) -> Option<String> {
    graph
        .nodes
        .iter()
        .find(|candidate| candidate.key == *node)
        .and_then(|candidate| {
            candidate
                .target_address
                .map(|address| address.ip().to_string())
                .or_else(|| {
                    candidate
                        .inferred
                        .as_ref()
                        .and_then(|inferred| inferred.management_address)
                        .map(|address| address.to_string())
                })
        })
}

/// Return true when a prefix belongs to a public address block.
fn is_public_prefix(prefix: &str) -> bool {
    Subnet::from_prefix(prefix).is_some_and(|network| is_public_ip(network.address))
}

/// Return true when an address is globally routable enough to display as a customer allocation.
fn is_public_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            !address.is_private()
                && !address.is_loopback()
                && !address.is_link_local()
                && !address.is_multicast()
                && !address.is_broadcast()
                && !address.is_documentation()
                && !address.is_unspecified()
        }
        IpAddr::V6(address) => {
            !address.is_loopback()
                && !address.is_unspecified()
                && !address.is_multicast()
                && !address.is_unique_local()
                && !address.is_unicast_link_local()
                && !is_ipv6_documentation(address)
        }
    }
}

/// Return true for the IPv6 documentation prefix `2001:db8::/32`.
fn is_ipv6_documentation(address: Ipv6Addr) -> bool {
    let segments = address.segments();
    segments[0] == 0x2001 && segments[1] == 0x0db8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_address_detection_covers_ipv4_ipv6_and_prefix_parse_failures() {
        assert!(is_public_ip("8.8.8.8".parse().unwrap()));
        assert!(is_public_ip("2606:4700:4700::1111".parse().unwrap()));
        for address in [
            "10.0.0.1",
            "127.0.0.1",
            "169.254.1.1",
            "224.0.0.1",
            "255.255.255.255",
            "192.0.2.1",
            "0.0.0.0",
            "::1",
            "::",
            "ff02::1",
            "fc00::1",
            "fe80::1",
            "2001:db8::1",
        ] {
            assert!(
                !is_public_ip(address.parse().unwrap()),
                "{address} should not be public"
            );
        }
        assert!(is_public_prefix("8.8.8.8/32"));
        assert!(!is_public_prefix("invalid"));
    }

    #[test]
    fn endpoint_address_helpers_deduplicate_and_compare_prefix_hosts() {
        let mut addresses = vec!["192.0.2.1".to_owned()];
        push_unique_address(&mut addresses, "192.0.2.1".to_owned());
        push_unique_address(&mut addresses, "192.0.2.2".to_owned());
        assert_eq!(addresses, ["192.0.2.1", "192.0.2.2"]);
        assert!(prefix_host_equals("192.0.2.1/24", "192.0.2.1"));
        assert!(prefix_host_equals("192.0.2.1", "192.0.2.1"));
        assert!(!prefix_host_equals("192.0.2.1/24", "192.0.2.2"));
        assert!(is_ipv6_documentation("2001:db8::1".parse().unwrap()));
        assert!(!is_ipv6_documentation("2001:db9::1".parse().unwrap()));
    }
}
