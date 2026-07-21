//! `/interface` snapshot collection.

use mikrotik_client::commands::PrintCommand;
use mikrotik_client::commands::interface::Interface;
use mikrotik_types::device::InterfaceSnapshot;

use super::EndpointCollector;
use crate::error::Result;

/// Collect required interface inventory and optional interface-family endpoints.
pub(super) async fn collect(collector: &EndpointCollector<'_>) -> Result<InterfaceSnapshot> {
    Ok(InterfaceSnapshot {
        interfaces: collector.required_many(command(Interface::Interface)).await?,
        ethernet_interfaces: collector.optional_many(command(Interface::EthernetInterface)).await,
        bridges: collector.optional_many(command(Interface::Bridge)).await,
        bridge_hosts: collector.optional_many(command(Interface::BridgeHost)).await,
        bridge_ports: collector.optional_many(command(Interface::BridgePort)).await,
        bridge_settings: collector.optional_many(command(Interface::BridgeSettings)).await,
        bridge_vlans: collector.optional_many(command(Interface::BridgeVlan)).await,
        detect_internet: collector.optional_many(command(Interface::DetectInternet)).await,
        ethernet_switches: collector.optional_many(command(Interface::EthernetSwitch)).await,
        ethernet_switch_ports: collector.optional_many(command(Interface::EthernetSwitchPort)).await,
        ethernet_switch_port_isolations: collector
            .optional_many(command(Interface::EthernetSwitchPortIsolation))
            .await,
        interface_lists: collector.optional_many(command(Interface::InterfaceList)).await,
        interface_list_members: collector.optional_many(command(Interface::InterfaceListMember)).await,
        lte_apns: collector.optional_many(command(Interface::LteApn)).await,
        vlan_interfaces: collector.optional_many(command(Interface::VlanInterface)).await,
        wireguard_interfaces: collector.optional_many(command(Interface::WireGuardInterface)).await,
        wireguard_peers: collector.optional_many(command(Interface::WireGuardPeer)).await,
        wireless_security_profiles: collector
            .optional_many(command(Interface::WirelessSecurityProfile))
            .await,
        wireless_registrations: collector.optional_many(command(Interface::WirelessRegistration)).await,
        wifi_registrations: collector.optional_many(command(Interface::WifiRegistration)).await,
    })
}

/// Wrap an `/interface` command in the top-level print command.
const fn command(command: Interface) -> PrintCommand {
    PrintCommand::Interface(command)
}
