//! Queue, SNMP, tool, and user snapshot collection.

use mikrotik_client::commands::PrintCommand;
use mikrotik_client::commands::queue::Queue;
use mikrotik_client::commands::snmp::Snmp;
use mikrotik_client::commands::tool::Tool;
use mikrotik_client::commands::user::User;
use mikrotik_types::device::QueueSnapshot;
use mikrotik_types::device::SnmpSnapshot;
use mikrotik_types::device::ToolSnapshot;
use mikrotik_types::device::UserSnapshot;

use super::EndpointCollector;

/// Collect `/queue` endpoints.
pub(super) async fn collect_queue(collector: &EndpointCollector<'_>) -> QueueSnapshot {
    QueueSnapshot {
        queue_interfaces: collector
            .optional_many(PrintCommand::Queue(Queue::QueueInterface))
            .await,
        queue_types: collector.optional_many(PrintCommand::Queue(Queue::QueueType)).await,
    }
}

/// Collect `/snmp` endpoints.
pub(super) async fn collect_snmp(collector: &EndpointCollector<'_>) -> SnmpSnapshot {
    SnmpSnapshot {
        snmp: collector.optional_many(PrintCommand::Snmp(Snmp::Snmp)).await,
        snmp_communities: collector.optional_many(PrintCommand::Snmp(Snmp::SnmpCommunity)).await,
    }
}

/// Collect `/tool` endpoints.
pub(super) async fn collect_tool(collector: &EndpointCollector<'_>) -> ToolSnapshot {
    ToolSnapshot {
        bandwidth_servers: collector.optional_many(tool(Tool::BandwidthServer)).await,
        emails: collector.optional_many(tool(Tool::Email)).await,
        graphing: collector.optional_many(tool(Tool::Graphing)).await,
        mac_server_pings: collector.optional_many(tool(Tool::MacServerPing)).await,
        romon: collector.optional_many(tool(Tool::Romon)).await,
        romon_ports: collector.optional_many(tool(Tool::RomonPort)).await,
        sms: collector.optional_many(tool(Tool::Sms)).await,
        sniffers: collector.optional_many(tool(Tool::Sniffer)).await,
        traffic_generators: collector.optional_many(tool(Tool::TrafficGenerator)).await,
        traffic_generator_latency_distributions: collector
            .optional_many(tool(Tool::TrafficGeneratorLatencyDistribution))
            .await,
    }
}

/// Collect `/user` endpoints.
pub(super) async fn collect_user(collector: &EndpointCollector<'_>) -> UserSnapshot {
    UserSnapshot {
        active_users: collector.optional_many(user(User::ActiveUser)).await,
        ssh_keys: collector.optional_many(user(User::SshKey)).await,
        users: collector.optional_many(user(User::User)).await,
        user_aaa: collector.optional_many(user(User::UserAaa)).await,
        user_groups: collector.optional_many(user(User::UserGroup)).await,
        user_settings: collector.optional_many(user(User::UserSettings)).await,
    }
}

/// Wrap a `/tool` command in the top-level print command.
const fn tool(command: Tool) -> PrintCommand {
    PrintCommand::Tool(command)
}

/// Wrap a `/user` command in the top-level print command.
const fn user(command: User) -> PrintCommand {
    PrintCommand::User(command)
}
