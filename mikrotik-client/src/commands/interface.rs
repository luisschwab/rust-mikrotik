//! `RouterOS` `/interface` print command paths.

/// `RouterOS` print command `/interface/bridge/print`.
const INTERFACE_BRIDGE_PRINT: &str = "/interface/bridge/print";

/// `RouterOS` print command `/interface/bridge/host/print`.
const INTERFACE_BRIDGE_HOST_PRINT: &str = "/interface/bridge/host/print";

/// `RouterOS` print command `/interface/bridge/port/print`.
const INTERFACE_BRIDGE_PORT_PRINT: &str = "/interface/bridge/port/print";

/// `RouterOS` print command `/interface/bridge/settings/print`.
const INTERFACE_BRIDGE_SETTINGS_PRINT: &str = "/interface/bridge/settings/print";

/// `RouterOS` print command `/interface/bridge/vlan/print`.
const INTERFACE_BRIDGE_VLAN_PRINT: &str = "/interface/bridge/vlan/print";

/// `RouterOS` print command `/interface/detect-internet/print`.
const INTERFACE_DETECT_INTERNET_PRINT: &str = "/interface/detect-internet/print";

/// `RouterOS` print command `/interface/ethernet/print`.
const INTERFACE_ETHERNET_INTERFACE_PRINT: &str = "/interface/ethernet/print";

/// `RouterOS` print command `/interface/ethernet/switch/print`.
const INTERFACE_ETHERNET_SWITCH_PRINT: &str = "/interface/ethernet/switch/print";

/// `RouterOS` print command `/interface/ethernet/switch/port/print`.
const INTERFACE_ETHERNET_SWITCH_PORT_PRINT: &str = "/interface/ethernet/switch/port/print";

/// `RouterOS` print command `/interface/ethernet/switch/port-isolation/print`.
const INTERFACE_ETHERNET_SWITCH_PORT_ISOLATION_PRINT: &str = "/interface/ethernet/switch/port-isolation/print";

/// `RouterOS` print command `/interface/print`.
const INTERFACE_INTERFACE_PRINT: &str = "/interface/print";

/// `RouterOS` print command `/interface/list/print`.
const INTERFACE_INTERFACE_LIST_PRINT: &str = "/interface/list/print";

/// `RouterOS` print command `/interface/list/member/print`.
const INTERFACE_INTERFACE_LIST_MEMBER_PRINT: &str = "/interface/list/member/print";

/// `RouterOS` print command `/interface/lte/apn/print`.
const INTERFACE_LTE_APN_PRINT: &str = "/interface/lte/apn/print";

/// `RouterOS` print command `/interface/vlan/print`.
const INTERFACE_VLAN_INTERFACE_PRINT: &str = "/interface/vlan/print";

/// `RouterOS` print command `/interface/wireguard/print`.
const INTERFACE_WIRE_GUARD_INTERFACE_PRINT: &str = "/interface/wireguard/print";

/// `RouterOS` print command `/interface/wireguard/peers/print`.
const INTERFACE_WIRE_GUARD_PEER_PRINT: &str = "/interface/wireguard/peers/print";

/// `RouterOS` print command `/interface/wireless/security-profiles/print`.
const INTERFACE_WIRELESS_SECURITY_PROFILE_PRINT: &str = "/interface/wireless/security-profiles/print";

/// `RouterOS` print commands in this command family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Interface {
    /// `RouterOS` print command.
    Bridge,
    /// `RouterOS` print command.
    BridgeHost,
    /// `RouterOS` print command.
    BridgePort,
    /// `RouterOS` print command.
    BridgeSettings,
    /// `RouterOS` print command.
    BridgeVlan,
    /// `RouterOS` print command.
    DetectInternet,
    /// `RouterOS` print command.
    EthernetInterface,
    /// `RouterOS` print command.
    EthernetSwitch,
    /// `RouterOS` print command.
    EthernetSwitchPort,
    /// `RouterOS` print command.
    EthernetSwitchPortIsolation,
    /// `RouterOS` print command.
    Interface,
    /// `RouterOS` print command.
    InterfaceList,
    /// `RouterOS` print command.
    InterfaceListMember,
    /// `RouterOS` print command.
    LteApn,
    /// `RouterOS` print command.
    VlanInterface,
    /// `RouterOS` print command.
    WireGuardInterface,
    /// `RouterOS` print command.
    WireGuardPeer,
    /// `RouterOS` print command.
    WirelessSecurityProfile,
}

impl Interface {
    /// All command variants in generated order.
    pub const ALL: &[Self] = &[
        Self::Bridge,
        Self::BridgeHost,
        Self::BridgePort,
        Self::BridgeSettings,
        Self::BridgeVlan,
        Self::DetectInternet,
        Self::EthernetInterface,
        Self::EthernetSwitch,
        Self::EthernetSwitchPort,
        Self::EthernetSwitchPortIsolation,
        Self::Interface,
        Self::InterfaceList,
        Self::InterfaceListMember,
        Self::LteApn,
        Self::VlanInterface,
        Self::WireGuardInterface,
        Self::WireGuardPeer,
        Self::WirelessSecurityProfile,
    ];

    /// Return the `RouterOS` API command path.
    pub const fn as_path(self) -> &'static str {
        match self {
            Self::Bridge => INTERFACE_BRIDGE_PRINT,
            Self::BridgeHost => INTERFACE_BRIDGE_HOST_PRINT,
            Self::BridgePort => INTERFACE_BRIDGE_PORT_PRINT,
            Self::BridgeSettings => INTERFACE_BRIDGE_SETTINGS_PRINT,
            Self::BridgeVlan => INTERFACE_BRIDGE_VLAN_PRINT,
            Self::DetectInternet => INTERFACE_DETECT_INTERNET_PRINT,
            Self::EthernetInterface => INTERFACE_ETHERNET_INTERFACE_PRINT,
            Self::EthernetSwitch => INTERFACE_ETHERNET_SWITCH_PRINT,
            Self::EthernetSwitchPort => INTERFACE_ETHERNET_SWITCH_PORT_PRINT,
            Self::EthernetSwitchPortIsolation => INTERFACE_ETHERNET_SWITCH_PORT_ISOLATION_PRINT,
            Self::Interface => INTERFACE_INTERFACE_PRINT,
            Self::InterfaceList => INTERFACE_INTERFACE_LIST_PRINT,
            Self::InterfaceListMember => INTERFACE_INTERFACE_LIST_MEMBER_PRINT,
            Self::LteApn => INTERFACE_LTE_APN_PRINT,
            Self::VlanInterface => INTERFACE_VLAN_INTERFACE_PRINT,
            Self::WireGuardInterface => INTERFACE_WIRE_GUARD_INTERFACE_PRINT,
            Self::WireGuardPeer => INTERFACE_WIRE_GUARD_PEER_PRINT,
            Self::WirelessSecurityProfile => INTERFACE_WIRELESS_SECURITY_PROFILE_PRINT,
        }
    }
}

mikrotik_common::impl_command_display!(Interface);
