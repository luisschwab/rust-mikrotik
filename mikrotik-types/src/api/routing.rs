//! Routing API response rows.

use alloc::string::String;
use alloc::string::ToString as _;
use alloc::vec::Vec;
use core::fmt;
use core::net::IpAddr;
use core::str::FromStr;

use serde::Deserialize;
use serde::Serialize;

use crate::RouterOsId;
use crate::primitives::ParseError;
use crate::primitives::interface::InterfaceName;
use crate::primitives::ip::IpPrefix;
use crate::primitives::ip::ScopedIpAddress;
use crate::primitives::routing::BgpSessionState;
use crate::primitives::routing::RouteDestination;
use crate::primitives::routing::RouteGateway;
use crate::primitives::routing::RoutingTableName;
use crate::primitives::system::RouterOsDuration;

/// BGP remote address as reported by `RouterOS`.
///
/// `RouterOS` may report BGP connection remote addresses either as a bare IP or
/// as a host prefix such as `192.0.2.1/32`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct BgpRemoteAddress(String);

impl BgpRemoteAddress {
    /// Return the stored `RouterOS` value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return the IP address portion.
    ///
    /// # Panics
    ///
    /// Panics if the stored value is not a valid IP address or IP prefix. Values
    /// are validated when constructing or deserializing `BgpRemoteAddress`.
    #[must_use]
    pub fn address(&self) -> IpAddr {
        self.0
            .split_once('/')
            .map_or(self.0.as_str(), |(address, _)| address)
            .parse()
            .expect("BgpRemoteAddress stores only validated IP addresses")
    }
}

impl FromStr for BgpRemoteAddress {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.parse::<IpAddr>().is_ok() || value.parse::<IpPrefix>().is_ok() {
            Ok(Self(value.to_string()))
        } else {
            Err(ParseError::IpEndpointAddress)
        }
    }
}

impl fmt::Display for BgpRemoteAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for BgpRemoteAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// Response row from `/routing/bgp/connection/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BgpConnection {
    /// Internal `RouterOS` row ID.
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    pub id: Option<RouterOsId>,
    /// Connection name.
    pub name: Option<String>,
    /// Remote peer address.
    #[serde(alias = "remote.address")]
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub remote_address: Option<BgpRemoteAddress>,
    /// Remote autonomous system number.
    #[serde(alias = "remote.as")]
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub remote_as: Option<u32>,
    /// Whether this connection is disabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub disabled: Option<bool>,
}

/// Response row from `RouterOS` v6 `/routing/bgp/peer/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BgpPeer {
    /// Internal `RouterOS` row ID.
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    pub id: Option<RouterOsId>,
    /// Peer name.
    pub name: Option<String>,
    /// Remote peer address.
    #[serde(alias = "remote.address")]
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub remote_address: Option<IpAddr>,
    /// Remote autonomous system number.
    #[serde(alias = "remote.as")]
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub remote_as: Option<u32>,
    /// Whether this peer is disabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub disabled: Option<bool>,
    /// Whether this peer session is established.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub established: Option<bool>,
}

/// Response row from `/routing/bgp/session/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BgpSession {
    /// Session name.
    pub name: Option<String>,
    /// Remote peer address.
    #[serde(alias = "remote.address")]
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub remote_address: Option<IpAddr>,
    /// Remote autonomous system number.
    #[serde(alias = "remote.as")]
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub remote_as: Option<u32>,
    /// Session state.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub state: Option<BgpSessionState>,
    /// Whether the session is established.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub established: Option<bool>,
}

/// Response row from `/routing/bgp/template/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BgpTemplate {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this bgp template.
    pub name: Option<String>,
    #[serde(rename = "as", deserialize_with = "crate::optional_from_str")]
    /// Autonomous system number configured for the BGP template.
    pub autonomous_system: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Routing table name.
    pub routing_table: Option<RoutingTableName>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is the default row.
    pub default: Option<bool>,
}

/// Response row from `/routing/id/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RoutingId {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this routing id.
    pub name: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Dynamically selected routing ID.
    pub dynamic_id: Option<IpAddr>,
    /// Policy for selecting a dynamic routing ID.
    pub select_dynamic_id: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// VRF name.
    pub select_from_vrf: Option<RoutingTableName>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row was created dynamically by `RouterOS`.
    pub dynamic: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is inactive.
    pub inactive: Option<bool>,
}

/// Response row from `/routing/nexthop/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RoutingNexthop {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Resolved nexthop address.
    pub address: Option<String>,
    /// Address family identifier for this routing entry.
    pub afi: Option<String>,
    /// Gateway health-check method.
    pub check_gateway: Option<String>,
    #[serde(alias = "immediate-gw.address")]
    /// Resolved immediate gateway address.
    pub immediate_gw_address: Option<String>,
    #[serde(alias = "immediate-gw.interface-idx", deserialize_with = "crate::optional_from_str")]
    /// Internal interface index for the resolved immediate gateway.
    pub immediate_gw_interface_idx: Option<u32>,
    #[serde(alias = "immediate-gw.weight", deserialize_with = "crate::optional_from_str")]
    /// Weight assigned to the resolved immediate gateway.
    pub immediate_gw_weight: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Route scope value.
    pub scope: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Target scope used to resolve nexthops.
    pub target_scope: Option<u32>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the nexthop belongs to a BGP VPN route.
    pub bgp_vpn: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether gateway checking currently succeeds.
    pub gw_check_ok: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the nexthop interface is currently usable.
    pub interface_ok: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the nexthop is currently reachable.
    pub reachable: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether nexthop resolution failed.
    pub unresolved: Option<bool>,
}

/// Response row from `/routing/route/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RoutingRoute {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Address family identifier for this routing entry.
    pub afi: Option<String>,
    /// Routing component that owns this route.
    pub belongs_to: Option<String>,
    /// How this route contributes to route selection.
    pub contribution: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Destination address or destination address matcher.
    pub dst_address: Option<RouteDestination>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Configured gateway value.
    pub gateway: Option<RouteGateway>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Resolved immediate gateway value.
    pub immediate_gw: Option<RouteGateway>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Local address associated with this row.
    pub local_address: Option<ScopedIpAddress>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Routing table name.
    pub routing_table: Option<RoutingTableName>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// VRF interface associated with this route.
    pub vrf_interface: Option<InterfaceName>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Internal nexthop identifier selected for the route.
    pub nexthop_id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Administrative distance used during route selection.
    pub distance: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Route scope value.
    pub scope: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Target scope used to resolve nexthops.
    pub target_scope: Option<u32>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is active.
    pub active: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this route discards matching traffic.
    pub blackhole: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is a connected route.
    pub connect: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this route was learned from DHCP.
    pub dhcp: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether equal-cost multipath is active for this route.
    pub ecmp: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the route is hardware offloaded.
    pub hw_offloaded: Option<bool>,
    #[serde(rename = "static", deserialize_with = "crate::optional_bool")]
    /// Whether this is a static route.
    pub static_route: Option<bool>,
}

/// Response row from `/routing/igmp-proxy/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct IgmpProxy {
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interval between IGMP proxy membership queries.
    pub query_interval: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Time allowed for IGMP proxy query responses.
    pub query_response_interval: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether IGMP proxy quick-leave behavior is enabled.
    pub quick_leave: Option<bool>,
}

/// Response row from `/routing/table/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RoutingTable {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Name of this routing table.
    pub name: Option<RoutingTableName>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row was created dynamically by `RouterOS`.
    pub dynamic: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `RouterOS` considers this row invalid.
    pub invalid: Option<bool>,
}

/// Response row from `/routing/stats/process/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RoutingStatsProcess {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID for this row.
    pub row_id: Option<RouterOsId>,
    /// Internal `RouterOS` row ID.
    pub id: Option<String>,
    /// Process identifier.
    pub pid: Option<String>,
    /// Routing process runtime identifier.
    pub rpid: Option<String>,
    /// Private memory blocks used by this process.
    pub private_mem_blocks: Option<String>,
    /// Shared memory blocks used by this process.
    pub shared_mem_blocks: Option<String>,
    /// Proportional set size reported by `RouterOS`.
    pub pss: Option<String>,
    /// Resident set size reported by `RouterOS`.
    pub rss: Option<String>,
    /// Virtual memory size reported by `RouterOS`.
    pub vms: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Kernel time consumed by the routing process.
    pub kernel_time: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// User-space time consumed by the routing process.
    pub process_time: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Current busy time for the routing process.
    pub cur_busy: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum busy time observed for the routing process.
    pub max_busy: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Current routing calculation time.
    pub cur_calc: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum routing calculation time.
    pub max_calc: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::comma_list")]
    /// Tasks currently associated with the routing process.
    pub tasks: Vec<String>,
}

/// Response row from `/routing/settings/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RoutingSettings {
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether routing runs in a single process.
    pub single_process: Option<bool>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interval between gateway ping checks.
    pub check_gateway_ping_interval: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Timeout for gateway ping checks.
    pub check_gateway_ping_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Number of gateway pings used by each check.
    pub check_gateway_ping_count: Option<u32>,
    #[serde(deserialize_with = "crate::comma_list")]
    /// Enabled routing policy rule sources.
    pub policy_rules: Vec<String>,
}

/// Response row from `/routing/stats/memory/print`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RoutingStatsMemory {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this routing stats memory.
    pub name: Option<String>,
    /// Routing process identifier.
    pub procid: Option<String>,
    /// Size of each object tracked by the memory pool.
    pub object_size: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Number of objects tracked by the memory pool.
    pub objects: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Cells available per memory pool page.
    pub page_cell_count: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Unused space left in memory pool pages.
    pub page_slack: Option<f64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Number of pages allocated by the memory pool.
    pub pages: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Unused bytes in the memory pool.
    pub unused: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Unused objects in the memory pool.
    pub unused_objects: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Used bytes in the memory pool.
    pub used: Option<u64>,
}

/// Response row from `/routing/stats/origin/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RoutingStatsOrigin {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this routing stats origin.
    pub name: Option<String>,
    /// Process identifier.
    pub pid: Option<String>,
    /// Number of routes associated with the origin.
    pub route_count: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Routing instance identifier.
    pub instance_id: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Routing publisher index.
    pub publisher_idx: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Route type associated with the origin.
    pub route_type: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Total number of routes associated with the origin.
    pub total_route_count: Option<u64>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the routing origin has been abandoned.
    pub abandoned: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Number of route attribute merge operations.
    pub attrs_merge: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Number of route attribute update operations.
    pub attrs_updated: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the routing origin is being held.
    pub hold: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the routing origin is stopping.
    pub stopping: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the routing origin is synthetic.
    pub synthetic: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this routing origin is terminal.
    pub terminal: Option<bool>,
}

/// Response row from `/routing/stats/step/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RoutingStatsStep {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Routing worker context for this step.
    pub context: Option<String>,
    /// Name of this routing stats step.
    pub name: Option<String>,
    /// Process identifier.
    pub pid: Option<String>,
    /// Current state of the routing worker step.
    pub state: Option<String>,
    /// Routing targets processed by this step.
    pub targets: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Current runtime spent in this routing step.
    pub cur_time: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum runtime observed for this routing step.
    pub max_time: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Execution order for this routing step.
    pub order: Option<i32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Number of times this routing step has run.
    pub runs: Option<u64>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this interface or service is running.
    pub running: Option<bool>,
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::BgpConnection;
    use super::BgpPeer;
    use super::BgpSession;
    use super::RoutingRoute;
    use super::RoutingStatsMemory;
    use crate::Row;

    #[test]
    fn routing_route_deserializes_live_typed_fields() {
        let mut row = Row::new();
        row.insert(".id".into(), "*8000000D".into());
        row.insert("afi".into(), "ip4".into());
        row.insert("dst-address".into(), "0.0.0.0/0".into());
        row.insert("gateway".into(), "192.168.1.254".into());
        row.insert("immediate-gw".into(), "192.168.1.254%ether1".into());
        row.insert("routing-table".into(), "main".into());
        row.insert("vrf-interface".into(), "ether1".into());
        row.insert("nexthop-id".into(), "*20182720".into());
        row.insert("distance".into(), "1".into());
        row.insert("active".into(), "true".into());

        let route = crate::deserialize::<RoutingRoute>(&row).expect("routing route row should deserialize");

        assert_eq!(route.afi.as_deref(), Some("ip4"));
        assert_eq!(route.distance, Some(1));
        assert_eq!(route.active, Some(true));
        assert_eq!(
            route.nexthop_id.expect("nexthop id should be present").as_str(),
            "*20182720"
        );
    }

    #[test]
    fn routing_route_deserializes_scoped_ipv6_prefixes() {
        let mut row = Row::new();
        row.insert(".id".into(), "*201040CC".into());
        row.insert("afi".into(), "ip6".into());
        row.insert("dst-address".into(), "fe80::%ether1/64".into());
        row.insert("gateway".into(), "ether1".into());

        let route = crate::deserialize::<RoutingRoute>(&row).expect("scoped IPv6 route should deserialize");

        assert_eq!(
            route.dst_address.as_ref().map(ToString::to_string).as_deref(),
            Some("fe80::%ether1/64")
        );
    }

    #[test]
    fn bgp_session_deserializes_string_established_flag() {
        let mut row = Row::new();
        row.insert(".id".into(), "*2800001".into());
        row.insert("name".into(), "right-0-1".into());
        row.insert("established".into(), "true".into());
        row.insert("remote.address".into(), "10.200.0.2".into());
        row.insert("remote.as".into(), "65001".into());

        let session = crate::deserialize::<BgpSession>(&row).expect("BGP session row should deserialize");

        assert_eq!(session.established, Some(true));
        assert_eq!(session.remote_as, Some(65001));
    }

    #[test]
    fn bgp_connection_deserializes_remote_peer_fields() {
        let mut row = Row::new();
        row.insert(".id".into(), "*2".into());
        row.insert("name".into(), "upstream-a".into());
        row.insert("disabled".into(), "false".into());
        row.insert("remote.address".into(), "198.51.100.9/32".into());
        row.insert("remote.as".into(), "64501".into());

        let connection = crate::deserialize::<BgpConnection>(&row).expect("BGP connection row should deserialize");

        assert_eq!(connection.name.as_deref(), Some("upstream-a"));
        assert_eq!(connection.disabled, Some(false));
        assert_eq!(
            connection.remote_address.map(|address| address.to_string()).as_deref(),
            Some("198.51.100.9/32")
        );
        assert_eq!(connection.remote_as, Some(64501));
    }

    #[test]
    fn bgp_peer_deserializes_v6_remote_peer_fields() {
        let mut row = Row::new();
        row.insert(".id".into(), "*7".into());
        row.insert("name".into(), "Rt_BORDERv1".into());
        row.insert("disabled".into(), "false".into());
        row.insert("established".into(), "true".into());
        row.insert("remote-address".into(), "10.100.0.157".into());
        row.insert("remote-as".into(), "65001".into());

        let peer = crate::deserialize::<BgpPeer>(&row).expect("RouterOS v6 BGP peer row should deserialize");

        assert_eq!(peer.name.as_deref(), Some("Rt_BORDERv1"));
        assert_eq!(peer.disabled, Some(false));
        assert_eq!(peer.established, Some(true));
        assert_eq!(
            peer.remote_address.map(|address| address.to_string()).as_deref(),
            Some("10.100.0.157")
        );
        assert_eq!(peer.remote_as, Some(65001));
    }

    #[test]
    fn routing_stats_memory_deserializes_fractional_page_slack() {
        let mut row = Row::new();
        row.insert(".id".into(), "*2".into());
        row.insert("name".into(), "Dst+1".into());
        row.insert("objects".into(), "1".into());
        row.insert("page-cell-count".into(), "282".into());
        row.insert("page-slack".into(), "253.75".into());

        let memory = crate::deserialize::<RoutingStatsMemory>(&row).expect("routing stats memory should deserialize");

        assert_eq!(memory.objects, Some(1));
        assert_eq!(memory.page_slack, Some(253.75));
    }

    #[test]
    fn routing_stats_step_deserializes_negative_order() {
        let mut row = Row::new();
        row.insert(".id".into(), "*11".into());
        row.insert("context".into(), "PUBLISH".into());
        row.insert("cur-time".into(), "0ms".into());
        row.insert("max-time".into(), "0ms".into());
        row.insert("name".into(), "route/static/dst/publish".into());
        row.insert("order".into(), "-100".into());
        row.insert("pid".into(), "main".into());
        row.insert("running".into(), "false".into());
        row.insert("runs".into(), "274".into());
        row.insert("state".into(), "on".into());

        let step = crate::deserialize::<super::RoutingStatsStep>(&row).expect("routing stats step should deserialize");

        assert_eq!(step.order, Some(-100));
        assert_eq!(step.runs, Some(274));
    }
}
