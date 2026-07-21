//! `/routing` snapshot collection.

use mikrotik_client::commands::PrintCommand;
use mikrotik_client::commands::routing::Routing;
use mikrotik_types::device::RoutingSnapshot;

use super::EndpointCollector;

/// Collect optional routing protocol and routing diagnostics endpoints.
pub(super) async fn collect(collector: &EndpointCollector<'_>) -> RoutingSnapshot {
    RoutingSnapshot {
        bgp_sessions: collector.optional_many(command(Routing::BgpSession)).await,
        bgp_connections: collector.optional_many(command(Routing::BgpConnection)).await,
        bgp_peers: collector.optional_many(command(Routing::BgpPeer)).await,
        bgp_templates: collector.optional_many(command(Routing::BgpTemplate)).await,
        igmp_proxy: collector.optional_many(command(Routing::IgmpProxy)).await,
        routing_ids: collector.optional_many(command(Routing::RoutingId)).await,
        routing_nexthops: collector.optional_many(command(Routing::RoutingNexthop)).await,
        routing_routes: collector.optional_many(command(Routing::RoutingRoute)).await,
        routing_settings: collector.optional_many(command(Routing::RoutingSettings)).await,
        routing_stats_memory: collector.optional_many(command(Routing::RoutingStatsMemory)).await,
        routing_stats_origin: collector.optional_many(command(Routing::RoutingStatsOrigin)).await,
        routing_stats_processes: collector.optional_many(command(Routing::RoutingStatsProcess)).await,
        routing_stats_steps: collector.optional_many(command(Routing::RoutingStatsStep)).await,
        routing_tables: collector.optional_many(command(Routing::RoutingTable)).await,
    }
}

/// Wrap a `/routing` command in the top-level print command.
const fn command(command: Routing) -> PrintCommand {
    PrintCommand::Routing(command)
}
