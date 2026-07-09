//! `RouterOS` routing print command paths.

/// `RouterOS` print command `/routing/bgp/session/print`.
const ROUTING_BGP_SESSION_PRINT: &str = "/routing/bgp/session/print";

/// `RouterOS` print command `/routing/bgp/connection/print`.
const ROUTING_BGP_CONNECTION_PRINT: &str = "/routing/bgp/connection/print";

/// `RouterOS` v6 print command `/routing/bgp/peer/print`.
const ROUTING_BGP_PEER_PRINT: &str = "/routing/bgp/peer/print";

/// `RouterOS` print command `/routing/bgp/template/print`.
const ROUTING_BGP_TEMPLATE_PRINT: &str = "/routing/bgp/template/print";

/// `RouterOS` print command `/routing/igmp-proxy/print`.
const ROUTING_IGMP_PROXY_PRINT: &str = "/routing/igmp-proxy/print";

/// `RouterOS` print command `/routing/id/print`.
const ROUTING_ROUTING_ID_PRINT: &str = "/routing/id/print";

/// `RouterOS` print command `/routing/nexthop/print`.
const ROUTING_ROUTING_NEXTHOP_PRINT: &str = "/routing/nexthop/print";

/// `RouterOS` print command `/routing/route/print`.
const ROUTING_ROUTING_ROUTE_PRINT: &str = "/routing/route/print";

/// `RouterOS` print command `/routing/settings/print`.
const ROUTING_ROUTING_SETTINGS_PRINT: &str = "/routing/settings/print";

/// `RouterOS` print command `/routing/stats/memory/print`.
const ROUTING_ROUTING_STATS_MEMORY_PRINT: &str = "/routing/stats/memory/print";

/// `RouterOS` print command `/routing/stats/origin/print`.
const ROUTING_ROUTING_STATS_ORIGIN_PRINT: &str = "/routing/stats/origin/print";

/// `RouterOS` print command `/routing/stats/process/print`.
const ROUTING_ROUTING_STATS_PROCESS_PRINT: &str = "/routing/stats/process/print";

/// `RouterOS` print command `/routing/stats/step/print`.
const ROUTING_ROUTING_STATS_STEP_PRINT: &str = "/routing/stats/step/print";

/// `RouterOS` print command `/routing/table/print`.
const ROUTING_ROUTING_TABLE_PRINT: &str = "/routing/table/print";

/// `RouterOS` print commands in this command family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Routing {
    /// `RouterOS` print command.
    BgpConnection,
    /// `RouterOS` print command.
    BgpPeer,
    /// `RouterOS` print command.
    BgpSession,
    /// `RouterOS` print command.
    BgpTemplate,
    /// `RouterOS` print command.
    IgmpProxy,
    /// `RouterOS` print command.
    RoutingId,
    /// `RouterOS` print command.
    RoutingNexthop,
    /// `RouterOS` print command.
    RoutingRoute,
    /// `RouterOS` print command.
    RoutingSettings,
    /// `RouterOS` print command.
    RoutingStatsMemory,
    /// `RouterOS` print command.
    RoutingStatsOrigin,
    /// `RouterOS` print command.
    RoutingStatsProcess,
    /// `RouterOS` print command.
    RoutingStatsStep,
    /// `RouterOS` print command.
    RoutingTable,
}

impl Routing {
    /// All command variants in generated order.
    pub const ALL: &[Self] = &[
        Self::BgpConnection,
        Self::BgpPeer,
        Self::BgpSession,
        Self::BgpTemplate,
        Self::IgmpProxy,
        Self::RoutingId,
        Self::RoutingNexthop,
        Self::RoutingRoute,
        Self::RoutingSettings,
        Self::RoutingStatsMemory,
        Self::RoutingStatsOrigin,
        Self::RoutingStatsProcess,
        Self::RoutingStatsStep,
        Self::RoutingTable,
    ];

    /// Return the `RouterOS` API command path.
    pub const fn as_path(self) -> &'static str {
        match self {
            Self::BgpConnection => ROUTING_BGP_CONNECTION_PRINT,
            Self::BgpPeer => ROUTING_BGP_PEER_PRINT,
            Self::BgpSession => ROUTING_BGP_SESSION_PRINT,
            Self::BgpTemplate => ROUTING_BGP_TEMPLATE_PRINT,
            Self::IgmpProxy => ROUTING_IGMP_PROXY_PRINT,
            Self::RoutingId => ROUTING_ROUTING_ID_PRINT,
            Self::RoutingNexthop => ROUTING_ROUTING_NEXTHOP_PRINT,
            Self::RoutingRoute => ROUTING_ROUTING_ROUTE_PRINT,
            Self::RoutingSettings => ROUTING_ROUTING_SETTINGS_PRINT,
            Self::RoutingStatsMemory => ROUTING_ROUTING_STATS_MEMORY_PRINT,
            Self::RoutingStatsOrigin => ROUTING_ROUTING_STATS_ORIGIN_PRINT,
            Self::RoutingStatsProcess => ROUTING_ROUTING_STATS_PROCESS_PRINT,
            Self::RoutingStatsStep => ROUTING_ROUTING_STATS_STEP_PRINT,
            Self::RoutingTable => ROUTING_ROUTING_TABLE_PRINT,
        }
    }
}

mikrotik_common::impl_command_display!(Routing);
