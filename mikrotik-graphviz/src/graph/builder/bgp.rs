use core::net::IpAddr;
use std::collections::HashMap;

use mikrotik_types::api::routing::BgpConnection;
use mikrotik_types::api::routing::BgpPeer;
use mikrotik_types::api::routing::BgpSession;
use mikrotik_types::device::DeviceKey;
use mikrotik_types::device::DeviceSnapshot;
use mikrotik_types::primitives::interface::InterfaceName;
use mikrotik_types::primitives::ip::DiscoveryProtocol;
use mikrotik_types::topology::InferredDevice;
use mikrotik_types::topology::NetworkNode;
use mikrotik_types::topology::NetworkNodeStatus;
use mikrotik_types::topology::TopologyLink;

/// Build graph edges from BGP sessions reported by one collected device.
pub(super) fn bgp_session_edges(
    snapshot: &DeviceSnapshot,
    target_keys: &HashMap<String, DeviceKey>,
    address_interfaces: &HashMap<String, (DeviceKey, InterfaceName)>,
    inferred_nodes: &mut HashMap<DeviceKey, NetworkNode>,
) -> Vec<TopologyLink> {
    let local_key = snapshot.stable_key();
    snapshot
        .bgp_sessions
        .iter()
        .filter_map(|session| {
            let remote_address = session.remote_address?;
            let (remote_key, remote_interface) = bgp_remote_endpoint(
                remote_address,
                session.name.as_deref(),
                session.remote_as,
                target_keys,
                address_interfaces,
                inferred_nodes,
            );
            if remote_key == local_key {
                return None;
            }

            Some(TopologyLink {
                local_node: local_key.clone(),
                local_interface: super::l3::interface_for_remote_address(snapshot, remote_address),
                remote_node: remote_key,
                remote_interface,
                discovered_by: vec![DiscoveryProtocol::Unknown("bgp".to_owned())],
                confidence: bgp_edge_confidence(session),
            })
        })
        .collect()
}

/// Build graph edges from configured BGP connections reported by one collected device.
pub(super) fn bgp_connection_edges(
    snapshot: &DeviceSnapshot,
    target_keys: &HashMap<String, DeviceKey>,
    address_interfaces: &HashMap<String, (DeviceKey, InterfaceName)>,
    inferred_nodes: &mut HashMap<DeviceKey, NetworkNode>,
) -> Vec<TopologyLink> {
    let local_key = snapshot.stable_key();
    snapshot
        .bgp_connections
        .iter()
        .filter_map(|connection| {
            if connection.disabled == Some(true) {
                return None;
            }
            let remote_address = connection.remote_address.as_ref()?.address();
            let (remote_key, remote_interface) = bgp_remote_endpoint(
                remote_address,
                connection.name.as_deref(),
                connection.remote_as,
                target_keys,
                address_interfaces,
                inferred_nodes,
            );
            if remote_key == local_key {
                return None;
            }

            Some(TopologyLink {
                local_node: local_key.clone(),
                local_interface: super::l3::interface_for_remote_address(snapshot, remote_address),
                remote_node: remote_key,
                remote_interface,
                discovered_by: vec![DiscoveryProtocol::Unknown("bgp".to_owned())],
                confidence: bgp_connection_confidence(connection),
            })
        })
        .collect()
}

/// Build graph edges from `RouterOS` v6 configured BGP peers reported by one collected device.
pub(super) fn bgp_peer_edges(
    snapshot: &DeviceSnapshot,
    target_keys: &HashMap<String, DeviceKey>,
    address_interfaces: &HashMap<String, (DeviceKey, InterfaceName)>,
    inferred_nodes: &mut HashMap<DeviceKey, NetworkNode>,
) -> Vec<TopologyLink> {
    let local_key = snapshot.stable_key();
    snapshot
        .bgp_peers
        .iter()
        .filter_map(|peer| {
            if peer.disabled == Some(true) {
                return None;
            }
            let remote_address = peer.remote_address?;
            let (remote_key, remote_interface) = bgp_remote_endpoint(
                remote_address,
                peer.name.as_deref(),
                peer.remote_as,
                target_keys,
                address_interfaces,
                inferred_nodes,
            );
            if remote_key == local_key {
                return None;
            }

            Some(TopologyLink {
                local_node: local_key.clone(),
                local_interface: super::l3::interface_for_remote_address(snapshot, remote_address),
                remote_node: remote_key,
                remote_interface,
                discovered_by: vec![DiscoveryProtocol::Unknown("bgp".to_owned())],
                confidence: bgp_peer_confidence(peer),
            })
        })
        .collect()
}

/// Return a BGP remote endpoint, creating an opaque external node for uncrawled peers.
fn bgp_remote_endpoint(
    remote_address: IpAddr,
    remote_name: Option<&str>,
    remote_as: Option<u32>,
    target_keys: &HashMap<String, DeviceKey>,
    address_interfaces: &HashMap<String, (DeviceKey, InterfaceName)>,
    inferred_nodes: &mut HashMap<DeviceKey, NetworkNode>,
) -> (DeviceKey, Option<InterfaceName>) {
    if let Some(remote_key) = target_keys.get(&remote_address.to_string()) {
        return (
            remote_key.clone(),
            super::l3::interface_for_address(remote_key, remote_address, address_interfaces),
        );
    }

    let remote_key = bgp_peer_key(remote_address);
    inferred_nodes
        .entry(remote_key.clone())
        .or_insert_with(|| inferred_bgp_peer_node(remote_key.clone(), remote_address, remote_name, remote_as));
    (remote_key, None)
}

/// Return a stable key for an opaque external BGP peer.
fn bgp_peer_key(remote_address: IpAddr) -> DeviceKey {
    format!("bgp:{remote_address}").into()
}

/// Build an inferred node for an opaque external BGP peer.
fn inferred_bgp_peer_node(
    key: DeviceKey,
    remote_address: IpAddr,
    remote_name: Option<&str>,
    remote_as: Option<u32>,
) -> NetworkNode {
    let identity = remote_name.filter(|name| !name.trim().is_empty()).map_or_else(
        || {
            remote_as.map_or_else(
                || format!("BGP peer\n{remote_address}"),
                |remote_as| format!("BGP AS{remote_as}\n{remote_address}"),
            )
        },
        |name| format!("{name}\n{remote_address}"),
    );
    NetworkNode {
        key,
        status: NetworkNodeStatus::Inferred,
        snapshot: None,
        inferred: Some(InferredDevice {
            management_address: Some(remote_address),
            identity: Some(identity),
            board: None,
            platform: None,
            version: None,
            mac_address: None,
            failure: None,
        }),
    }
}

/// Return confidence for an edge discovered from BGP state.
fn bgp_edge_confidence(session: &BgpSession) -> u8 {
    if session.established == Some(true) { 95 } else { 75 }
}

/// Return confidence for an edge discovered from BGP configuration.
fn bgp_connection_confidence(connection: &BgpConnection) -> u8 {
    debug_assert_ne!(connection.disabled, Some(true));
    70
}

/// Return confidence for an edge discovered from a `RouterOS` v6 BGP peer.
fn bgp_peer_confidence(peer: &BgpPeer) -> u8 {
    if peer.established == Some(true) {
        95
    } else {
        debug_assert_ne!(peer.disabled, Some(true));
        70
    }
}
