use std::collections::BTreeMap;

use mikrotik_types::device::DeviceKey;
use mikrotik_types::device::DeviceSnapshot;

use super::model::NetworkGraph;

/// Index of interface IP prefixes by graph node and interface name.
#[derive(Debug, Default)]
pub(super) struct GraphAddressIndex {
    /// `node -> interface -> prefixes` map.
    addresses: BTreeMap<DeviceKey, BTreeMap<String, Vec<String>>>,
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
    pub(super) fn addresses(&self, node: &DeviceKey, interface: &str) -> &[String] {
        self.addresses
            .get(node)
            .and_then(|interfaces| interfaces.get(interface))
            .map_or(&[], Vec::as_slice)
    }
}

/// Return configured address values grouped by interface name.
fn interface_addresses(snapshot: &DeviceSnapshot) -> BTreeMap<String, Vec<String>> {
    let mut addresses = BTreeMap::<String, Vec<String>>::new();
    for address in &snapshot.addresses {
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
