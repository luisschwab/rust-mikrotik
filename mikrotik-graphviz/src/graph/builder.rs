use std::collections::BTreeMap;
use std::collections::HashMap;

use mikrotik_types::topology::FailedNeighborCrawl;
use mikrotik_types::topology::InferredNeighborEvidence;
use mikrotik_types::topology::NetworkNode;
use mikrotik_types::topology::NetworkNodeStatus;

use super::model::NetworkGraph;
use crate::snapshot::GraphSnapshot;

/// BGP-based graph edge inference.
mod bgp;
/// Layer-3 and route-based graph edge inference.
mod l3;
/// Duplicate edge merging and deterministic edge ordering.
mod merge;
/// Neighbor evidence and fallback edge inference.
mod neighbor;

use self::bgp::bgp_connection_edges;
use self::bgp::bgp_peer_edges;
use self::bgp::bgp_session_edges;
use self::l3::l3_link_edges;
use self::l3::route_next_hop_edges;
use self::merge::edge_order;
use self::merge::mark_reciprocal_edges;
use self::merge::merge_duplicate_edges;
use self::neighbor::failed_neighbor_l3_edges;
use self::neighbor::inferred_node;
use self::neighbor::remote_key;
use self::neighbor::unconnected_failed_neighbor_fallback_edges;
use self::neighbor::unconnected_neighbor_fallback_edges;
use self::neighbor::wireless_neighbor_edges;

/// Build a graph from collected snapshots and failed neighbor crawl evidence.
pub fn build_graph<S, I>(snapshots: &[S], failed_neighbors: I) -> NetworkGraph
where
    for<'a> GraphSnapshot: From<&'a S>,
    I: IntoIterator<Item = FailedNeighborCrawl>,
{
    build_graph_with_neighbor_evidence(snapshots, [], failed_neighbors)
}

/// Build a graph from collected snapshots plus successful and failed neighbor crawl evidence.
pub fn build_graph_with_neighbor_evidence<S, I, J>(
    snapshots: &[S],
    neighbor_evidence: I,
    failed_neighbors: J,
) -> NetworkGraph
where
    for<'a> GraphSnapshot: From<&'a S>,
    I: IntoIterator<Item = InferredNeighborEvidence>,
    J: IntoIterator<Item = FailedNeighborCrawl>,
{
    let snapshots = snapshots.iter().map(GraphSnapshot::from).collect::<Vec<_>>();
    let snapshots = snapshots.as_slice();
    let mut nodes = BTreeMap::new();
    let mut target_keys = HashMap::new();
    let mut address_interfaces = HashMap::new();
    let mut neighbor_keys = HashMap::new();
    let neighbor_evidence = neighbor_evidence.into_iter().collect::<Vec<_>>();
    let failed_neighbors = failed_neighbors.into_iter().collect::<Vec<_>>();

    for snapshot in snapshots {
        let key = snapshot.topology_node_key();
        target_keys.insert(snapshot.target_address.to_string(), key.clone());
        target_keys.insert(snapshot.target_address.ip().to_string(), key.clone());
        for address in &snapshot.management_addresses {
            target_keys.insert(address.to_string(), key.clone());
        }
        for address in &snapshot.ip.addresses.data {
            let Some(prefix) = address.address.as_ref() else {
                continue;
            };
            if let Some((host, _prefix)) = prefix.as_str().split_once('/') {
                target_keys.insert(host.to_owned(), key.clone());
                if let Some(interface) = address.actual_interface.as_ref().or(address.interface.as_ref()) {
                    address_interfaces.insert(host.to_owned(), (key.clone(), interface.clone()));
                }
            }
        }
        nodes.entry(key.clone()).or_insert_with(|| NetworkNode {
            key,
            status: NetworkNodeStatus::Collected,
            role: Some(snapshot.role),
            target_address: Some(snapshot.target_address),
            management_addresses: snapshot.management_addresses.clone(),
            snapshot: Some(snapshot.snapshot.clone()),
            inferred: None,
        });
    }

    for failure in &failed_neighbors {
        let key = remote_key(&failure.neighbor, &target_keys);
        let mut node = inferred_node(key.clone(), &failure.neighbor);
        if let Some(inferred) = &mut node.inferred {
            inferred.failure = Some(failure.failure);
        }
        neighbor_keys.insert(key, node);
    }

    let mut edges = l3_link_edges(snapshots);
    edges.extend(route_next_hop_edges(snapshots, &target_keys, &address_interfaces));
    edges.extend(failed_neighbor_l3_edges(snapshots, &failed_neighbors, &target_keys));
    for snapshot in snapshots {
        edges.extend(bgp_session_edges(
            snapshot,
            &target_keys,
            &address_interfaces,
            &mut neighbor_keys,
        ));
        edges.extend(bgp_connection_edges(
            snapshot,
            &target_keys,
            &address_interfaces,
            &mut neighbor_keys,
        ));
        edges.extend(bgp_peer_edges(
            snapshot,
            &target_keys,
            &address_interfaces,
            &mut neighbor_keys,
        ));
    }
    for (key, node) in neighbor_keys {
        nodes.entry(key).or_insert(node);
    }
    edges.extend(wireless_neighbor_edges(&nodes, &neighbor_evidence, &target_keys));
    edges.extend(unconnected_neighbor_fallback_edges(
        &nodes,
        &edges,
        &neighbor_evidence,
        &target_keys,
    ));
    edges.extend(unconnected_failed_neighbor_fallback_edges(
        &nodes,
        &edges,
        &failed_neighbors,
        &target_keys,
    ));

    mark_reciprocal_edges(&mut edges);
    let mut edges = merge_duplicate_edges(edges);
    edges.sort_by(edge_order);
    edges.dedup();

    NetworkGraph {
        nodes: nodes.into_values().collect(),
        edges,
    }
}

/// Return the host/address portion of a target address when it includes an API port.
pub(super) fn target_address_host(target_address: &str) -> Option<&str> {
    if let Some(rest) = target_address.strip_prefix('[') {
        let (host, rest) = rest.split_once(']')?;
        return rest.strip_prefix(':').is_some().then_some(host);
    }

    let (host, port) = target_address.rsplit_once(':')?;
    port.parse::<u16>().ok()?;
    (!host.is_empty()).then_some(host)
}
