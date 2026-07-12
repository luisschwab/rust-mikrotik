use mikrotik_types::topology::NetworkNode;
use mikrotik_types::topology::TopologyLink;
use serde::Deserialize;
use serde::Serialize;

/// Device-centric network graph discovered by the crawler.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NetworkGraph {
    /// Devices collected or inferred during discovery.
    pub nodes: Vec<NetworkNode>,
    /// Discovery links between device nodes.
    pub edges: Vec<TopologyLink>,
}
