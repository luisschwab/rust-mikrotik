//! Platform and management-service snapshot collection.

use mikrotik_client::commands::PrintCommand;
use mikrotik_client::commands::ip::Ip;
use mikrotik_client::commands::service::Service;
use mikrotik_types::api::ip::IpService;
use mikrotik_types::device::CapsManSnapshot;
use mikrotik_types::device::CertificateSnapshot;
use mikrotik_types::device::ConsoleSnapshot;
use mikrotik_types::device::DiskSnapshot;
use mikrotik_types::device::EndpointSnapshot;
use mikrotik_types::device::FileSnapshot;
use mikrotik_types::device::MplsSnapshot;
use mikrotik_types::device::PartitionsSnapshot;
use mikrotik_types::device::PppSnapshot;
use mikrotik_types::device::RadiusSnapshot;

use super::EndpointCollector;

/// Sections whose commands live outside the primary system and network families.
pub(super) struct PlatformSnapshots {
    /// `/ip/service` rows, retained in the `/ip` section of the public model.
    pub(super) ip_services: EndpointSnapshot<Vec<IpService>>,
    /// `/certificate` section.
    pub(super) certificate: CertificateSnapshot,
    /// `/console` section.
    pub(super) console: ConsoleSnapshot,
    /// `/disk` section.
    pub(super) disk: DiskSnapshot,
    /// `/file` section.
    pub(super) file: FileSnapshot,
    /// `/partitions` section.
    pub(super) partitions: PartitionsSnapshot,
    /// `/caps-man` section.
    pub(super) caps_man: CapsManSnapshot,
    /// `/mpls` section.
    pub(super) mpls: MplsSnapshot,
    /// `/ppp` section.
    pub(super) ppp: PppSnapshot,
    /// `/radius` section.
    pub(super) radius: RadiusSnapshot,
}

/// Collect platform and management-service sections.
pub(super) async fn collect(collector: &EndpointCollector<'_>) -> PlatformSnapshots {
    PlatformSnapshots {
        ip_services: collector.optional_many(PrintCommand::Ip(Ip::IpService)).await,
        certificate: CertificateSnapshot {
            certificates: collector.optional_many(command(Service::Certificate)).await,
            certificate_settings: collector.optional_many(command(Service::CertificateSettings)).await,
        },
        console: ConsoleSnapshot {
            console_settings: collector.optional_many(command(Service::ConsoleSettings)).await,
        },
        disk: DiskSnapshot {
            disks: collector.optional_many(command(Service::Disk)).await,
            disk_settings: collector.optional_many(command(Service::DiskSettings)).await,
        },
        file: FileSnapshot {
            files: collector.optional_many(command(Service::File)).await,
        },
        partitions: PartitionsSnapshot {
            partitions: collector.optional_many(command(Service::Partition)).await,
        },
        caps_man: CapsManSnapshot {
            caps_man_aaa: collector.optional_many(command(Service::CapsManAaa)).await,
            caps_man_managers: collector.optional_many(command(Service::CapsManManager)).await,
            caps_man_manager_interfaces: collector.optional_many(command(Service::CapsManManagerInterface)).await,
        },
        mpls: MplsSnapshot {
            mpls_settings: collector.optional_many(command(Service::MplsSettings)).await,
        },
        ppp: PppSnapshot {
            ppp_aaa: collector.optional_many(command(Service::PppAaa)).await,
            ppp_profiles: collector.optional_many(command(Service::PppProfile)).await,
        },
        radius: RadiusSnapshot {
            radius_incoming: collector.optional_many(command(Service::RadiusIncoming)).await,
        },
    }
}

/// Wrap a platform-service command in the top-level print command.
const fn command(command: Service) -> PrintCommand {
    PrintCommand::Service(command)
}
