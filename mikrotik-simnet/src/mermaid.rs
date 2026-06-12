//! Mermaid diagram rendering for parsed topology manifests.

use std::collections::BTreeSet;

use crate::topology::Endpoint;
use crate::topology::Link;
use crate::topology::Router;
use crate::topology::RouterCommand;
use crate::topology::Topology;

/// Render a topology as a Mermaid flowchart.
pub(crate) fn render_topology_mermaid(topology: &Topology) -> String {
    let mut lines = vec!["flowchart LR".to_owned()];

    for (index, router) in topology.routers.iter().enumerate() {
        lines.push(format!(
            "  {}[({}<br/>RouterOS {})]",
            router_id(index),
            mermaid_label(&router.name),
            mermaid_label(&router.version)
        ));
    }

    for link in &topology.links {
        let a_index = router_index(topology, &link.a.router);
        let b_index = router_index(topology, &link.b.router);
        lines.push(format!(
            "  {} ---|\"{}\"| {}",
            router_id(a_index),
            mermaid_edge_label(&link_label(topology, link)),
            router_id(b_index)
        ));
    }

    format!("{}\n", lines.join("\n"))
}

/// Return the label for one topology link.
fn link_label(topology: &Topology, link: &Link) -> String {
    let a_router = topology
        .router(&link.a.router)
        .expect("validated topology link should reference an existing router");
    let b_router = topology
        .router(&link.b.router)
        .expect("validated topology link should reference an existing router");
    let a_addresses = endpoint_addresses(a_router, &link.a);
    let b_addresses = endpoint_addresses(b_router, &link.b);
    let kind = link_kind(a_router, b_router, &a_addresses, &b_addresses);

    format!(
        "{}<br/>{} - {}",
        kind,
        endpoint_label(&link.a, &a_addresses),
        endpoint_label(&link.b, &b_addresses)
    )
}

/// Return the display text for one endpoint and its configured addresses.
fn endpoint_label(endpoint: &Endpoint, addresses: &[String]) -> String {
    if addresses.is_empty() {
        endpoint.interface.clone()
    } else {
        format!("{} {}", endpoint.interface, addresses.join(", "))
    }
}

/// Return the configured IPv4 and IPv6 addresses for one router interface.
fn endpoint_addresses(router: &Router, endpoint: &Endpoint) -> Vec<String> {
    router
        .bootstrap
        .iter()
        .filter(|command| matches!(command.command.as_str(), "/ip/address/add" | "/ipv6/address/add"))
        .filter(|command| attribute_value(command, "interface") == Some(endpoint.interface.as_str()))
        .filter_map(|command| attribute_value(command, "address").map(ToOwned::to_owned))
        .collect()
}

/// Infer the logical kind of traffic configured across one physical link.
fn link_kind(a_router: &Router, b_router: &Router, a_addresses: &[String], b_addresses: &[String]) -> String {
    let a_hosts = address_hosts(a_addresses);
    let b_hosts = address_hosts(b_addresses);
    let mut kinds = BTreeSet::new();

    collect_router_link_kinds(a_router, &b_hosts, &mut kinds);
    collect_router_link_kinds(b_router, &a_hosts, &mut kinds);

    if kinds.is_empty() {
        "Ethernet".to_owned()
    } else {
        kinds.into_iter().collect::<Vec<_>>().join("/")
    }
}

/// Collect protocol kinds from commands that target a peer endpoint address.
fn collect_router_link_kinds(router: &Router, peer_hosts: &BTreeSet<String>, kinds: &mut BTreeSet<&'static str>) {
    for command in &router.bootstrap {
        if is_bgp_connection(command, peer_hosts) {
            kinds.insert("BGP");
        }
        if is_vpn_connection(command, peer_hosts) {
            kinds.insert("VPN");
        }
    }
}

/// Return whether this command configures BGP toward a peer endpoint address.
fn is_bgp_connection(command: &RouterCommand, peer_hosts: &BTreeSet<String>) -> bool {
    command.command.starts_with("/routing/bgp/connection/")
        && attribute_value(command, "remote.address").is_some_and(|address| peer_hosts.contains(address))
}

/// Return whether this command configures a VPN-like connection toward a peer endpoint address.
fn is_vpn_connection(command: &RouterCommand, peer_hosts: &BTreeSet<String>) -> bool {
    if command.command.starts_with("/interface/gre/") {
        return attribute_value(command, "remote-address").is_some_and(|address| peer_hosts.contains(address));
    }
    if command.command.starts_with("/interface/wireguard/peer/") {
        return attribute_value(command, "endpoint-address").is_some_and(|address| peer_hosts.contains(address));
    }
    if command.command.starts_with("/ip/ipsec/peer/") {
        return attribute_value(command, "address").is_some_and(|address| {
            address_host(address)
                .as_deref()
                .is_some_and(|host| peer_hosts.contains(host))
        });
    }

    false
}

/// Return all address host portions without prefix lengths.
fn address_hosts(addresses: &[String]) -> BTreeSet<String> {
    addresses.iter().filter_map(|address| address_host(address)).collect()
}

/// Return one address host portion without a prefix length.
fn address_host(address: &str) -> Option<String> {
    let host = address.split_once('/').map_or(address, |(host, _)| host);
    (!host.is_empty()).then(|| host.to_owned())
}

/// Return a command attribute value by key.
fn attribute_value<'a>(command: &'a RouterCommand, key: &str) -> Option<&'a str> {
    command
        .attributes
        .iter()
        .find(|attribute| attribute.key == key)
        .and_then(|attribute| attribute.value.as_deref())
}

/// Return the deterministic Mermaid node id for one router index.
fn router_id(index: usize) -> String {
    format!("r{index}")
}

/// Return the index of a router already validated by the topology parser.
fn router_index(topology: &Topology, router_name: &str) -> usize {
    topology
        .routers
        .iter()
        .position(|router| router.name == router_name)
        .expect("validated topology link should reference an existing router")
}

/// Escape text for a Mermaid node label.
fn mermaid_label(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Escape text for a Mermaid edge label.
fn mermaid_edge_label(value: &str) -> String {
    value.replace('|', "\\|").replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Topology;

    #[test]
    fn renders_routers_and_links_as_mermaid_flowchart() {
        let topology = Topology::parse(
            r#"
name = "two-router"

[[routers]]
name = "R01"
version = "7.23.1"

[[routers]]
name = "R02"
version = "7.23.1"

[[links]]
a = "R01:ether2"
b = "R02:ether1"
"#,
        )
        .expect("topology should parse");

        assert_eq!(
            render_topology_mermaid(&topology),
            "flowchart LR\n  r0[(R01<br/>RouterOS 7.23.1)]\n  r1[(R02<br/>RouterOS 7.23.1)]\n  r0 ---|\"Ethernet<br/>ether2 - ether1\"| r1\n"
        );
    }

    #[test]
    fn renders_link_addresses_and_connection_kind() {
        let topology = Topology::parse(
            r#"
name = "bgp"

[[routers]]
name = "R01"
version = "7.23.1"
bootstrap = [
  "/ip/address/add address=192.0.2.1/30 interface=ether2",
  "/routing/bgp/connection/add name=R1-R2 remote.address=192.0.2.2 remote.as=65002 local.role=ebgp",
]

[[routers]]
name = "R02"
version = "7.23.1"
bootstrap = [
  "/ip/address/add address=192.0.2.2/30 interface=ether1",
  "/routing/bgp/connection/add name=R2-R1 remote.address=192.0.2.1 remote.as=65001 local.role=ebgp",
]

[[links]]
a = "R01:ether2"
b = "R02:ether1"
"#,
        )
        .expect("topology should parse");

        assert_eq!(
            render_topology_mermaid(&topology),
            "flowchart LR\n  r0[(R01<br/>RouterOS 7.23.1)]\n  r1[(R02<br/>RouterOS 7.23.1)]\n  r0 ---|\"BGP<br/>ether2 192.0.2.1/30 - ether1 192.0.2.2/30\"| r1\n"
        );
    }

    #[test]
    fn escapes_mermaid_label_text() {
        let topology = Topology::parse(
            r#"
name = "escaped"

[[routers]]
name = "R<1>"
version = "7.23.1"
"#,
        )
        .expect("topology should parse");

        assert_eq!(
            render_topology_mermaid(&topology),
            "flowchart LR\n  r0[(R&lt;1&gt;<br/>RouterOS 7.23.1)]\n"
        );
    }
}
