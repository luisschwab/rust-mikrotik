use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;

use mikrotik_types::api::ip::Neighbor;
use mikrotik_types::device::TopologyNodeKey;
use mikrotik_types::primitives::interface::InterfaceName;
use mikrotik_types::primitives::ip::DiscoveryProtocol;
use mikrotik_types::primitives::ip::MacAddress;
use mikrotik_types::topology::FailedNeighborCrawl;
use mikrotik_types::topology::InferredDevice;
use mikrotik_types::topology::InferredNeighborEvidence;
use mikrotik_types::topology::NetworkNode;
use mikrotik_types::topology::NetworkNodeStatus;
use mikrotik_types::topology::TopologyLink;

use super::super::rank::radio_name_parts;
use crate::snapshot::GraphSnapshot;

/// Build real wireless/backhaul topology edges from neighbor evidence between collected radios.
pub(super) fn wireless_neighbor_edges(
    nodes: &BTreeMap<TopologyNodeKey, NetworkNode>,
    neighbor_evidence: &[InferredNeighborEvidence],
    target_keys: &HashMap<String, TopologyNodeKey>,
) -> Vec<TopologyLink> {
    neighbor_evidence
        .iter()
        .filter_map(|evidence| {
            let remote_node = remote_key(&evidence.neighbor, target_keys);
            if remote_node == evidence.local_node {
                return None;
            }

            let local = nodes.get(&evidence.local_node)?;
            let remote = nodes.get(&remote_node)?;
            if local.status != NetworkNodeStatus::Collected || remote.status != NetworkNodeStatus::Collected {
                return None;
            }
            if !neighbor_evidence_matches_radio_link(local, remote, evidence) {
                return None;
            }

            Some(TopologyLink {
                local_node: evidence.local_node.clone(),
                local_interface: evidence.local_interface.clone(),
                remote_node,
                remote_interface: evidence.neighbor.interface_name.clone(),
                discovered_by: vec![DiscoveryProtocol::Unknown("wireless".to_owned())],
                confidence: 70,
            })
        })
        .collect()
}

/// Build low-confidence neighbor fallback edges for collected nodes that would otherwise float.
pub(super) fn unconnected_neighbor_fallback_edges(
    nodes: &BTreeMap<TopologyNodeKey, NetworkNode>,
    edges: &[TopologyLink],
    neighbor_evidence: &[InferredNeighborEvidence],
    target_keys: &HashMap<String, TopologyNodeKey>,
) -> Vec<TopologyLink> {
    let connected_nodes = edges
        .iter()
        .flat_map(|edge| [edge.local_node.clone(), edge.remote_node.clone()])
        .collect::<BTreeSet<_>>();

    neighbor_evidence
        .iter()
        .filter_map(|evidence| {
            let remote_node = remote_key(&evidence.neighbor, target_keys);
            if remote_node == evidence.local_node {
                return None;
            }
            let remote = nodes.get(&remote_node)?;
            if remote.status != NetworkNodeStatus::Collected {
                return None;
            }
            if connected_nodes.contains(&remote_node) {
                return None;
            }

            Some(TopologyLink {
                local_node: evidence.local_node.clone(),
                local_interface: evidence.local_interface.clone(),
                remote_node,
                remote_interface: evidence.neighbor.interface_name.clone(),
                discovered_by: vec![DiscoveryProtocol::Unknown("fallback".to_owned())],
                confidence: 25,
            })
        })
        .collect()
}

/// Build low-confidence fallback edges for failed discovered nodes that would otherwise float.
pub(super) fn unconnected_failed_neighbor_fallback_edges(
    nodes: &BTreeMap<TopologyNodeKey, NetworkNode>,
    edges: &[TopologyLink],
    failed_neighbors: &[FailedNeighborCrawl],
    target_keys: &HashMap<String, TopologyNodeKey>,
) -> Vec<TopologyLink> {
    let connected_nodes = edges
        .iter()
        .flat_map(|edge| [edge.local_node.clone(), edge.remote_node.clone()])
        .collect::<BTreeSet<_>>();

    failed_neighbors
        .iter()
        .filter_map(|failure| {
            let remote_node = remote_key(&failure.neighbor, target_keys);
            if remote_node == failure.local_node {
                return None;
            }
            let remote = nodes.get(&remote_node)?;
            if remote.status != NetworkNodeStatus::Inferred {
                return None;
            }
            if connected_nodes.contains(&remote_node) {
                return None;
            }

            Some(TopologyLink {
                local_node: failure.local_node.clone(),
                local_interface: failure.local_interface.clone(),
                remote_node,
                remote_interface: failure.neighbor.interface_name.clone(),
                discovered_by: vec![DiscoveryProtocol::Unknown("fallback".to_owned())],
                confidence: 20,
            })
        })
        .collect()
}

/// Build inferred L3 edges to failed neighbor targets when local prefixes prove reachability.
pub(super) fn failed_neighbor_l3_edges(
    snapshots: &[GraphSnapshot],
    failed_neighbors: &[FailedNeighborCrawl],
    target_keys: &HashMap<String, TopologyNodeKey>,
) -> Vec<TopologyLink> {
    let snapshots_by_key = snapshots
        .iter()
        .map(|snapshot| (snapshot.topology_node_key(), snapshot))
        .collect::<HashMap<_, _>>();
    failed_neighbors
        .iter()
        .filter_map(|failure| {
            let remote_address = failure.neighbor.management_address()?;
            let snapshot = snapshots_by_key.get(&failure.local_node)?;
            let local_interface =
                super::l3::interface_for_l3_link_address(snapshot, remote_address, failure.local_interface.as_ref())?;
            let remote_node = remote_key(&failure.neighbor, target_keys);
            if remote_node == failure.local_node {
                return None;
            }
            Some(TopologyLink {
                local_node: failure.local_node.clone(),
                local_interface: Some(local_interface),
                remote_node,
                remote_interface: None,
                discovered_by: vec![DiscoveryProtocol::Unknown("l3".to_owned())],
                confidence: 60,
            })
        })
        .collect()
}

/// Return a remote graph key from neighbor evidence.
pub(super) fn remote_key(neighbor: &Neighbor, target_keys: &HashMap<String, TopologyNodeKey>) -> TopologyNodeKey {
    if let Some(address) = neighbor.management_address() {
        if let Some(key) = target_keys.get(&address.to_string()) {
            return key.clone();
        }
    }

    neighbor
        .identity
        .as_deref()
        .filter(|identity| !identity.trim().is_empty())
        .map(|identity| identity.to_owned().into())
        .or_else(|| neighbor.mac_address.map(mac_key))
        .or_else(|| neighbor.management_address().map(|address| address.to_string().into()))
        .unwrap_or_else(|| "unknown-neighbor".to_owned().into())
}

/// Build an inferred node from neighbor evidence.
pub(super) fn inferred_node(key: TopologyNodeKey, neighbor: &Neighbor) -> NetworkNode {
    NetworkNode {
        key,
        status: NetworkNodeStatus::Inferred,
        role: None,
        target_address: None,
        management_addresses: Vec::new(),
        snapshot: None,
        inferred: Some(InferredDevice {
            management_address: neighbor.management_address(),
            identity: neighbor.identity.clone(),
            board: neighbor.board.clone(),
            platform: neighbor.platform.clone(),
            version: neighbor.version.clone(),
            mac_address: neighbor.mac_address,
            failure: None,
        }),
    }
}

/// Return true when neighbor evidence plausibly identifies a radio/backhaul topology link.
fn neighbor_evidence_matches_radio_link(
    local: &NetworkNode,
    remote: &NetworkNode,
    evidence: &InferredNeighborEvidence,
) -> bool {
    radio_endpoint_match(
        &local.label(),
        evidence.local_interface.as_ref(),
        &remote.label(),
        evidence.neighbor.interface_name.as_ref(),
    ) || radio_endpoint_match(
        &remote.label(),
        evidence.neighbor.interface_name.as_ref(),
        &local.label(),
        evidence.local_interface.as_ref(),
    )
}

/// Return true when one side is a radio and the other side names one of its endpoints.
fn radio_endpoint_match(
    radio_label: &str,
    radio_interface: Option<&InterfaceName>,
    other_label: &str,
    other_interface: Option<&InterfaceName>,
) -> bool {
    let Some((left, right)) = radio_name_parts(radio_label) else {
        return false;
    };
    let other_label = normalized_endpoint_name(other_label);
    let radio_interface = radio_interface
        .map(ToString::to_string)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let other_interface = other_interface
        .map(ToString::to_string)
        .unwrap_or_default()
        .to_ascii_lowercase();
    [left, right].iter().any(|token| {
        let token = normalized_endpoint_name(token);
        other_label == token || other_interface.contains(&token) || radio_interface.contains(&token)
    })
}

/// Normalize labels for exact radio endpoint matching.
fn normalized_endpoint_name(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("Rt_")
        .trim_start_matches("RT_")
        .trim_start_matches("Rt-")
        .trim_start_matches("RT-")
        .to_ascii_lowercase()
}

/// Build a stable provisional key from a MAC address.
fn mac_key(mac_address: MacAddress) -> TopologyNodeKey {
    format!("mac:{mac_address}").into()
}
