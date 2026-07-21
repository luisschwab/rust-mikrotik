use core::net::IpAddr;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;

use mikrotik_types::abstractions::Subnet;
use mikrotik_types::abstractions::SubnetEndpoint;
use mikrotik_types::api::ip::Route;
use mikrotik_types::device::TopologyNodeKey;
use mikrotik_types::primitives::RouteGateway;
use mikrotik_types::primitives::interface::InterfaceName;
use mikrotik_types::primitives::interface::InterfaceType;
use mikrotik_types::primitives::ip::DiscoveryProtocol;
use mikrotik_types::topology::TopologyLink;

use crate::snapshot::GraphSnapshot;

/// Build graph edges from shared configured L3 interface networks.
pub(super) fn l3_link_edges(snapshots: &[GraphSnapshot]) -> Vec<TopologyLink> {
    let mut endpoints_by_network = BTreeMap::<Subnet, BTreeSet<SubnetEndpoint>>::new();
    for snapshot in snapshots {
        let node = snapshot.topology_node_key();
        for endpoint in l3_endpoints(snapshot) {
            if endpoint.is_bridge {
                continue;
            }
            endpoints_by_network
                .entry(endpoint.network)
                .or_default()
                .insert(SubnetEndpoint {
                    node: node.clone(),
                    interface: endpoint.interface,
                });
        }
    }

    let mut edges = Vec::new();
    for endpoints in endpoints_by_network.values() {
        let endpoints = endpoints.iter().collect::<Vec<_>>();
        for (left_index, left) in endpoints.iter().enumerate() {
            for right in endpoints.iter().skip(left_index + 1) {
                if left.node == right.node {
                    continue;
                }
                edges.push(TopologyLink {
                    local_node: left.node.clone(),
                    local_interface: Some(left.interface.clone()),
                    remote_node: right.node.clone(),
                    remote_interface: Some(right.interface.clone()),
                    discovered_by: vec![DiscoveryProtocol::Unknown("l3".to_owned())],
                    confidence: 90,
                });
            }
        }
    }
    edges
}

/// Build graph edges from active route next-hop evidence.
pub(super) fn route_next_hop_edges(
    snapshots: &[GraphSnapshot],
    target_keys: &HashMap<String, TopologyNodeKey>,
    address_interfaces: &HashMap<String, (TopologyNodeKey, InterfaceName)>,
) -> Vec<TopologyLink> {
    let mut edges = Vec::new();
    for snapshot in snapshots {
        let local_node = snapshot.topology_node_key();
        for route in &snapshot.ip.routes.data {
            let Some((remote_address, scoped_interface)) = route_next_hop(route) else {
                continue;
            };
            let Some(remote_node) = target_keys.get(&remote_address.to_string()) else {
                continue;
            };
            if *remote_node == local_node {
                continue;
            }

            let preferred_interface = route.vrf_interface.as_ref().or(scoped_interface.as_ref());
            let local_interface = preferred_interface
                .cloned()
                .or_else(|| interface_for_reachable_address(snapshot, remote_address, None));

            edges.push(TopologyLink {
                local_node: local_node.clone(),
                local_interface,
                remote_node: remote_node.clone(),
                remote_interface: interface_for_address(remote_node, remote_address, address_interfaces),
                discovered_by: vec![DiscoveryProtocol::Unknown("route".to_owned())],
                confidence: route_edge_confidence(route),
            });
        }
    }
    edges
}

/// Return the local interface whose configured prefix contains the BGP peer address.
pub(super) fn interface_for_remote_address(snapshot: &GraphSnapshot, remote_address: IpAddr) -> Option<InterfaceName> {
    interface_for_reachable_address(snapshot, remote_address, None)
}

/// Return the local interface whose configured link prefix contains an address.
pub(super) fn interface_for_l3_link_address(
    snapshot: &GraphSnapshot,
    remote_address: IpAddr,
    preferred_interface: Option<&InterfaceName>,
) -> Option<InterfaceName> {
    interface_for_reachable_address_with(snapshot, remote_address, preferred_interface, |network| {
        network.is_link_network()
    })
}

/// Return the interface whose configured address matches the requested node and address.
pub(super) fn interface_for_address(
    remote_key: &TopologyNodeKey,
    remote_address: IpAddr,
    address_interfaces: &HashMap<String, (TopologyNodeKey, InterfaceName)>,
) -> Option<InterfaceName> {
    address_interfaces
        .get(&remote_address.to_string())
        .filter(|(key, _interface)| key == remote_key)
        .map(|(_key, interface)| interface.clone())
}

/// Return the best next-hop address and optional scoped interface for a route.
fn route_next_hop(route: &Route) -> Option<(IpAddr, Option<InterfaceName>)> {
    if route.active != Some(true)
        || route.disabled == Some(true)
        || route.inactive == Some(true)
        || route.connect == Some(true)
    {
        return None;
    }
    route
        .immediate_gw
        .as_ref()
        .and_then(RouteGateway::next_hop)
        .or_else(|| route.gateway.as_ref().and_then(RouteGateway::next_hop))
}

/// Return confidence for an edge discovered from route next-hop evidence.
fn route_edge_confidence(route: &Route) -> u8 {
    debug_assert_eq!(route.active, Some(true));
    80
}

/// Return configured L3 endpoints for one snapshot.
fn l3_endpoints(snapshot: &GraphSnapshot) -> Vec<SubnetInterfaceAddress> {
    snapshot
        .ip
        .addresses
        .iter()
        .filter_map(|address| {
            if address.disabled == Some(true) || address.invalid == Some(true) {
                return None;
            }
            let prefix = address.address.as_ref()?;
            let network = Subnet::from(prefix);
            if !network.is_link_network() {
                return None;
            }
            let interface = address.actual_interface.as_ref().or(address.interface.as_ref())?;
            Some(SubnetInterfaceAddress {
                network,
                interface: interface.clone(),
                is_bridge: is_bridge_interface(snapshot, interface),
            })
        })
        .collect()
}

/// One interface address attached to a normalized subnet.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SubnetInterfaceAddress {
    /// Normalized network.
    network: Subnet,
    /// Interface connected to the network.
    interface: InterfaceName,
    /// Whether the interface is a bridge.
    is_bridge: bool,
}

/// Return the local interface whose configured prefix contains an address.
fn interface_for_reachable_address(
    snapshot: &GraphSnapshot,
    remote_address: IpAddr,
    preferred_interface: Option<&InterfaceName>,
) -> Option<InterfaceName> {
    interface_for_reachable_address_with(snapshot, remote_address, preferred_interface, |_| true)
}

/// Return the local interface whose configured prefix contains an address and passes a network predicate.
fn interface_for_reachable_address_with(
    snapshot: &GraphSnapshot,
    remote_address: IpAddr,
    preferred_interface: Option<&InterfaceName>,
    include_network: impl Fn(&Subnet) -> bool,
) -> Option<InterfaceName> {
    snapshot.ip.addresses.iter().find_map(|address| {
        if address.disabled == Some(true) || address.invalid == Some(true) {
            return None;
        }
        let interface = address.actual_interface.as_ref().or(address.interface.as_ref())?;
        if is_bridge_interface(snapshot, interface) {
            return None;
        }
        if preferred_interface.is_some_and(|preferred_interface| preferred_interface != interface) {
            return None;
        }
        let prefix = address.address.as_ref()?;
        let network = Subnet::from(prefix);
        if !include_network(&network) {
            return None;
        }
        if !network.contains(remote_address) {
            return None;
        }
        Some(interface.clone())
    })
}

/// Return whether an interface is explicitly a bridge interface in the snapshot.
fn is_bridge_interface(snapshot: &GraphSnapshot, interface: &InterfaceName) -> bool {
    snapshot.interface.interfaces.iter().any(|candidate| {
        candidate
            .name
            .as_ref()
            .or(candidate.default_name.as_ref())
            .is_some_and(|name| name == interface)
            && candidate.interface_type == Some(InterfaceType::Bridge)
    })
}
