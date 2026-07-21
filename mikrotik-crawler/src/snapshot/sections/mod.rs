//! Snapshot collectors grouped by `RouterOS` command section.

mod auxiliary;
mod interface;
mod ip;
mod platform;
mod routing;
mod system;

use std::collections::BTreeMap;

use mikrotik_types::device::RouterOsSnapshot;

use super::collector::EndpointCollector;
use crate::error::Result;

/// Collect every supported section into one typed `RouterOS` snapshot.
pub(super) async fn collect_router_os_snapshot(collector: &EndpointCollector<'_>) -> Result<RouterOsSnapshot> {
    let system = system::collect(collector).await?;
    let interface = interface::collect(collector).await?;
    let mut ip = ip::collect_ip(collector).await?;
    let ipv6 = ip::collect_ipv6(collector).await;
    let platform = platform::collect(collector).await;
    ip.ip_services = platform.ip_services;

    Ok(RouterOsSnapshot {
        system,
        interface,
        ip,
        ipv6,
        certificate: platform.certificate,
        console: platform.console,
        disk: platform.disk,
        file: platform.file,
        partitions: platform.partitions,
        caps_man: platform.caps_man,
        mpls: platform.mpls,
        ppp: platform.ppp,
        radius: platform.radius,
        queue: auxiliary::collect_queue(collector).await,
        snmp: auxiliary::collect_snmp(collector).await,
        tool: auxiliary::collect_tool(collector).await,
        routing: routing::collect(collector).await,
        user: auxiliary::collect_user(collector).await,
        raw: BTreeMap::new(),
    })
}
