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
