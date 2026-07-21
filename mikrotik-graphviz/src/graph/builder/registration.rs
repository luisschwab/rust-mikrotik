use std::collections::BTreeSet;
use std::collections::HashMap;

use mikrotik_types::api::interface::WifiRegistration;
use mikrotik_types::api::interface::WirelessRegistration;
use mikrotik_types::device::TopologyNodeKey;
use mikrotik_types::primitives::interface::InterfaceName;
use mikrotik_types::primitives::ip::DiscoveryProtocol;
use mikrotik_types::primitives::ip::MacAddress;
use mikrotik_types::topology::TopologyLink;

use crate::snapshot::GraphSnapshot;

/// Registration-derived graph edges and availability state used by heuristic fallback.
#[derive(Debug, Default)]
pub(super) struct RegistrationTopology {
    /// Resolved live registration edges.
    pub(super) edges: Vec<TopologyLink>,
    /// Resolved unordered device pairs.
    resolved_pairs: BTreeSet<(TopologyNodeKey, TopologyNodeKey)>,
    /// Devices with a successful authoritative `WiFi` registration endpoint.
    authoritative_wifi_nodes: BTreeSet<TopologyNodeKey>,
    /// Devices whose live registration rows could not be resolved uniquely.
    unresolved_nodes: BTreeSet<TopologyNodeKey>,
}

impl RegistrationTopology {
    /// Return whether name-based wireless inference may be used for this device pair.
    pub(super) fn allows_heuristic(&self, local: &TopologyNodeKey, remote: &TopologyNodeKey) -> bool {
        let pair = ordered_pair(local, remote);
        self.resolved_pairs.contains(&pair)
            || self.unresolved_nodes.contains(local)
            || self.unresolved_nodes.contains(remote)
            || (!self.authoritative_wifi_nodes.contains(local) && !self.authoritative_wifi_nodes.contains(remote))
    }
}

/// Registration evidence needed to resolve one live peer.
#[derive(Debug, Clone)]
struct RadioRegistrationEvidence {
    /// Device that reported the registration.
    local_node: TopologyNodeKey,
    /// Local radio interface.
    local_interface: Option<InterfaceName>,
    /// Registered peer MAC address.
    peer_mac: MacAddress,
}

/// Build live wireless registration edges and fallback availability state.
pub(super) fn registration_topology(snapshots: &[GraphSnapshot]) -> RegistrationTopology {
    let mac_index = interface_mac_index(snapshots);
    let mut topology = RegistrationTopology::default();
    let evidence = normalized_registration_evidence(snapshots, &mut topology);
    let mut resolved = Vec::new();

    for evidence in evidence {
        let candidates = mac_index.get(&evidence.peer_mac).map(Vec::as_slice).unwrap_or_default();
        let remote_nodes = candidates
            .iter()
            .filter(|candidate| candidate.node != evidence.local_node)
            .map(|candidate| candidate.node.clone())
            .collect::<BTreeSet<_>>();
        let Some(remote_node) = (remote_nodes.len() == 1)
            .then(|| remote_nodes.into_iter().next())
            .flatten()
        else {
            topology.unresolved_nodes.insert(evidence.local_node);
            continue;
        };
        let remote_interfaces = candidates
            .iter()
            .filter(|candidate| candidate.node == remote_node)
            .filter_map(|candidate| candidate.interface.clone())
            .collect::<BTreeSet<_>>();
        let remote_interface = (remote_interfaces.len() == 1)
            .then(|| remote_interfaces.into_iter().next())
            .flatten();
        resolved.push(ResolvedRegistration {
            local_node: evidence.local_node,
            local_interface: evidence.local_interface,
            remote_node,
            remote_interface,
        });
    }

    let directed_pairs = resolved
        .iter()
        .map(|registration| (registration.local_node.clone(), registration.remote_node.clone()))
        .collect::<BTreeSet<_>>();
    for registration in resolved {
        let reciprocal = directed_pairs.contains(&(registration.remote_node.clone(), registration.local_node.clone()));
        topology
            .resolved_pairs
            .insert(ordered_pair(&registration.local_node, &registration.remote_node));
        topology.edges.push(TopologyLink {
            local_node: registration.local_node,
            local_interface: registration.local_interface,
            remote_node: registration.remote_node,
            remote_interface: registration.remote_interface,
            discovered_by: vec![DiscoveryProtocol::Unknown("wireless-registration".to_owned())],
            confidence: if reciprocal { 100 } else { 95 },
        });
    }

    topology
}

/// One interface-MAC candidate.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MacCandidate {
    /// Device that owns the MAC.
    node: TopologyNodeKey,
    /// Interface that exposes the MAC.
    interface: Option<InterfaceName>,
}

/// One registration row whose peer resolved to a collected device.
#[derive(Debug)]
struct ResolvedRegistration {
    /// Device that reported the row.
    local_node: TopologyNodeKey,
    /// Local registered interface.
    local_interface: Option<InterfaceName>,
    /// Uniquely resolved peer device.
    remote_node: TopologyNodeKey,
    /// Uniquely resolved peer interface, when known.
    remote_interface: Option<InterfaceName>,
}

/// Index all collected interface MACs without assuming they are globally unique.
fn interface_mac_index(snapshots: &[GraphSnapshot]) -> HashMap<MacAddress, Vec<MacCandidate>> {
    let mut index = HashMap::<MacAddress, BTreeSet<MacCandidate>>::new();
    for snapshot in snapshots {
        let node = snapshot.topology_node_key();
        for interface in &snapshot.interface.interfaces.data {
            let Some(mac_address) = interface.mac_address else {
                continue;
            };
            index.entry(mac_address).or_default().insert(MacCandidate {
                node: node.clone(),
                interface: interface.name.clone(),
            });
        }
    }
    index
        .into_iter()
        .map(|(mac, candidates)| (mac, candidates.into_iter().collect()))
        .collect()
}

/// Normalize the fields needed from both registration-table shapes.
fn normalized_registration_evidence(
    snapshots: &[GraphSnapshot],
    topology: &mut RegistrationTopology,
) -> Vec<RadioRegistrationEvidence> {
    let mut evidence = Vec::new();
    for snapshot in snapshots {
        let local_node = snapshot.topology_node_key();
        if snapshot.interface.wifi_registrations.error.is_none() && has_wifi_interface(snapshot) {
            topology.authoritative_wifi_nodes.insert(local_node.clone());
        }
        evidence.extend(
            snapshot
                .interface
                .wireless_registrations
                .data
                .iter()
                .filter_map(|row| normalize_legacy(row, &local_node)),
        );
        evidence.extend(
            snapshot
                .interface
                .wifi_registrations
                .data
                .iter()
                .filter_map(|row| normalize_wifi(row, &local_node)),
        );
    }
    evidence
}

/// Return whether this snapshot has an interface owned by the newer `WiFi` stack.
fn has_wifi_interface(snapshot: &GraphSnapshot) -> bool {
    snapshot.interface.interfaces.data.iter().any(|interface| {
        interface
            .interface_type
            .as_ref()
            .is_some_and(|interface_type| interface_type.to_string().eq_ignore_ascii_case("wifi"))
    })
}

/// Normalize one legacy Wireless registration row.
fn normalize_legacy(row: &WirelessRegistration, local_node: &TopologyNodeKey) -> Option<RadioRegistrationEvidence> {
    Some(RadioRegistrationEvidence {
        local_node: local_node.clone(),
        local_interface: row.interface.clone(),
        peer_mac: row.mac_address?,
    })
}

/// Normalize one newer `WiFi` registration row.
fn normalize_wifi(row: &WifiRegistration, local_node: &TopologyNodeKey) -> Option<RadioRegistrationEvidence> {
    Some(RadioRegistrationEvidence {
        local_node: local_node.clone(),
        local_interface: row.interface.clone(),
        peer_mac: row.mac_address?,
    })
}

/// Return a deterministic unordered device pair.
fn ordered_pair(left: &TopologyNodeKey, right: &TopologyNodeKey) -> (TopologyNodeKey, TopologyNodeKey) {
    if left <= right {
        (left.clone(), right.clone())
    } else {
        (right.clone(), left.clone())
    }
}
