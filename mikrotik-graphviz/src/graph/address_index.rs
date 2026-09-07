use std::collections::BTreeMap;

use mikrotik_types::device::RouterOsSnapshot;
use mikrotik_types::device::TopologyNodeKey;

use super::model::NetworkGraph;

/// Index of interface IP prefixes by graph node and interface name.
#[derive(Debug, Default)]
pub(super) struct GraphAddressIndex {
    /// `node -> interface -> prefixes` map.
    addresses: BTreeMap<TopologyNodeKey, BTreeMap<String, Vec<String>>>,
}

impl GraphAddressIndex {
    /// Build an index for a graph.
    pub(super) fn new(graph: &NetworkGraph) -> Self {
        let mut addresses = BTreeMap::new();
        for node in &graph.nodes {
            let Some(snapshot) = &node.snapshot else {
                continue;
            };
            addresses.insert(node.key.clone(), interface_addresses(snapshot));
        }
        Self { addresses }
    }

    /// Return addresses for one node/interface.
    pub(super) fn addresses(&self, node: &TopologyNodeKey, interface: &str) -> &[String] {
        self.addresses
            .get(node)
            .and_then(|interfaces| interfaces.get(interface))
            .map_or(&[], Vec::as_slice)
    }
}

/// Return configured address values grouped by interface name.
fn interface_addresses(snapshot: &RouterOsSnapshot) -> BTreeMap<String, Vec<String>> {
    let mut addresses = BTreeMap::<String, Vec<String>>::new();
    for address in &snapshot.ip.addresses.data {
        let Some(prefix) = &address.address else {
            continue;
        };
        let interface = address
            .actual_interface
            .as_ref()
            .or(address.interface.as_ref())
            .map_or_else(|| "?".to_owned(), ToString::to_string);
        addresses.entry(interface).or_default().push(prefix.as_str().to_owned());
    }
    addresses
}

#[cfg(test)]
mod tests {
    use mikrotik_types::api::ip::Address;
    use mikrotik_types::device::IpSnapshot;

    use super::*;

    #[test]
    fn addresses_skip_empty_rows_and_prefer_actual_interface_names() {
        let snapshot = RouterOsSnapshot {
            ip: IpSnapshot {
                addresses: vec![
                    Address::default(),
                    Address {
                        address: Some("192.0.2.1/24".parse().unwrap()),
                        interface: Some("bridge".parse().unwrap()),
                        actual_interface: Some("ether2".parse().unwrap()),
                        ..Address::default()
                    },
                    Address {
                        address: Some("198.51.100.1/24".parse().unwrap()),
                        ..Address::default()
                    },
                ]
                .into(),
                ..IpSnapshot::default()
            },
            ..RouterOsSnapshot::default()
        };
        let addresses = interface_addresses(&snapshot);
        assert_eq!(addresses["ether2"], ["192.0.2.1/24"]);
        assert_eq!(addresses["?"], ["198.51.100.1/24"]);

        let node: TopologyNodeKey = "router".to_owned().into();
        let missing: TopologyNodeKey = "missing".to_owned().into();
        let graph = NetworkGraph {
            nodes: vec![mikrotik_types::topology::NetworkNode {
                key: node.clone(),
                status: mikrotik_types::topology::NetworkNodeStatus::Collected,
                role: None,
                target_address: None,
                management_addresses: Vec::new(),
                snapshot: Some(snapshot),
                inferred: None,
            }],
            ..NetworkGraph::default()
        };
        let index = GraphAddressIndex::new(&graph);
        assert_eq!(index.addresses(&node, "ether2"), ["192.0.2.1/24"]);
        assert!(index.addresses(&node, "missing").is_empty());
        assert!(index.addresses(&missing, "ether2").is_empty());
    }
}
