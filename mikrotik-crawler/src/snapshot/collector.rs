//! Required and optional `RouterOS` endpoint collection.

use core::time::Duration;

use mikrotik_client::client::Client;
use mikrotik_client::commands::PrintCommand;
use mikrotik_client::commands::interface::Interface;
use mikrotik_client::commands::ip::Ip;
use mikrotik_client::commands::routing::Routing;
use mikrotik_client::commands::system::System;
use mikrotik_client::error::Error as ClientError;
use mikrotik_common::debug_with_label;
use mikrotik_common::warn_with_label;
use mikrotik_types::device::EndpointError;
use mikrotik_types::device::EndpointErrorKind;
use mikrotik_types::device::EndpointSnapshot;
use serde::de::DeserializeOwned;
use tokio::time::timeout;

use crate::error::Error;
use crate::error::Result;

/// Shared context and failure policy for endpoint print commands.
pub(super) struct EndpointCollector<'a> {
    /// Address used to label collection diagnostics.
    target_address: &'a str,
    /// Connected binary API client.
    client: &'a Client,
    /// Maximum duration of one print command.
    command_timeout: Duration,
}

impl<'a> EndpointCollector<'a> {
    /// Create a collector for one connected target.
    pub(super) const fn new(target_address: &'a str, client: &'a Client, command_timeout: Duration) -> Self {
        Self {
            target_address,
            client,
            command_timeout,
        }
    }

    /// Collect every row from a required endpoint.
    pub(super) async fn required_many<T>(&self, command: PrintCommand) -> Result<EndpointSnapshot<Vec<T>>>
    where
        T: DeserializeOwned,
    {
        debug_with_label!(self.target_address, "running {command}");
        let rows = match timeout(self.command_timeout, self.client.print(command)).await {
            Err(_) => {
                return Err(Error::CommandTimeout {
                    command: command.to_string(),
                    duration: self.command_timeout,
                });
            }
            Ok(Ok(rows)) => rows,
            Ok(Err(error)) => {
                warn_with_label!(self.target_address, "{command} failed: {error}");
                return Err(error.into());
            }
        };
        Ok(EndpointSnapshot::success(rows))
    }

    /// Collect the single row from a required endpoint.
    pub(super) async fn required_first<T>(&self, command: PrintCommand) -> Result<EndpointSnapshot<T>>
    where
        T: DeserializeOwned,
    {
        let mut rows = self.required_many(command).await?.data;
        rows.pop()
            .map(EndpointSnapshot::success)
            .ok_or_else(|| Error::RequiredEndpointEmpty {
                command: command.to_string(),
            })
    }

    /// Collect every row from an optional endpoint, preserving any local failure.
    pub(super) async fn optional_many<T>(&self, command: PrintCommand) -> EndpointSnapshot<Vec<T>>
    where
        T: DeserializeOwned,
    {
        debug_with_label!(self.target_address, "running {command}");
        let command_name = command.to_string();
        match timeout(self.command_timeout, self.client.print(command)).await {
            Ok(Ok(rows)) => EndpointSnapshot::success(rows),
            Err(_) => endpoint_failure(
                command_name,
                EndpointErrorKind::Timeout,
                format!("command exceeded {:?}", self.command_timeout),
            ),
            Ok(Err(error)) => {
                let kind = classify_client_error(command, &error);
                let message = error.trap_message().map_or_else(|| error.to_string(), str::to_owned);
                warn_with_label!(self.target_address, "recording {command} failure: {message}");
                endpoint_failure(command_name, kind, message)
            }
        }
    }

    /// Collect the first row from an optional endpoint.
    pub(super) async fn optional_first<T>(&self, command: PrintCommand) -> EndpointSnapshot<T>
    where
        T: DeserializeOwned + Default,
    {
        let result = self.optional_many(command).await;
        EndpointSnapshot {
            data: result.data.into_iter().next().unwrap_or_default(),
            error: result.error,
        }
    }
}

/// Construct a failed endpoint result with empty data.
fn endpoint_failure<T: Default>(command: String, kind: EndpointErrorKind, message: String) -> EndpointSnapshot<T> {
    EndpointSnapshot {
        data: T::default(),
        error: Some(EndpointError { kind, command, message }),
    }
}

/// Classify a client failure for the endpoint-local public model.
fn classify_client_error(command: PrintCommand, error: &ClientError) -> EndpointErrorKind {
    match error {
        ClientError::PermissionDenied { .. } => EndpointErrorKind::PermissionDenied,
        ClientError::UnsupportedCommand { .. } => EndpointErrorKind::Unsupported,
        ClientError::Trap { message, .. } => classify_trap(command, message),
        ClientError::Decode(_) => EndpointErrorKind::Decode,
        ClientError::Timeout { .. } => EndpointErrorKind::Timeout,
        ClientError::Transport { .. }
        | ClientError::Connection { .. }
        | ClientError::Login(_)
        | ClientError::UnsupportedProtocol(_)
        | ClientError::ConnectionClosed { .. }
        | ClientError::Fatal { .. } => EndpointErrorKind::Transport,
    }
}

/// Classify a `RouterOS` command trap for UI and metrics consumers.
fn classify_trap(command: PrintCommand, message: &str) -> EndpointErrorKind {
    let normalized = message.to_ascii_lowercase();
    if normalized.contains("not enough permissions") || normalized.contains("permission denied") {
        EndpointErrorKind::PermissionDenied
    } else if is_unsupported_command_trap(command, message) {
        EndpointErrorKind::Unsupported
    } else {
        EndpointErrorKind::RouterOsTrap
    }
}

/// Return whether a trap identifies an endpoint unavailable on this device.
fn is_unsupported_command_trap(command: PrintCommand, message: &str) -> bool {
    message == "no such command prefix"
        || message.starts_with("no such command or directory")
        || is_optional_no_such_item_trap(command, message)
        || is_known_support_trap(command, message)
}

/// Return whether a `no such item` trap came from a known optional volatile table.
fn is_optional_no_such_item_trap(command: PrintCommand, message: &str) -> bool {
    matches!(message, "no such item" | "no such item (4)")
        && matches!(command, PrintCommand::Ip(Ip::FirewallConnection))
}

/// Return whether a `MikroTik` support trap is a known optional command failure.
fn is_known_support_trap(command: PrintCommand, message: &str) -> bool {
    message.to_ascii_lowercase().contains("contact mikrotik support")
        && matches!(
            command,
            PrintCommand::System(System::RouterboardResetButton)
                | PrintCommand::Interface(Interface::DetectInternet)
                | PrintCommand::Ip(Ip::IpsecPolicy | Ip::IpsecProfile | Ip::IpsecProposal | Ip::IpsecStatistics)
                | PrintCommand::Routing(Routing::RoutingStatsMemory)
        )
}

#[cfg(test)]
mod tests {
    use mikrotik_types::api::system::Identity;

    use super::*;

    const SUPPORT_TRAP: &str = "error - contact MikroTik support and send a supout file (3)";

    #[test]
    fn support_trap_is_unsupported_for_known_optional_commands() {
        for command in [
            PrintCommand::Interface(Interface::DetectInternet),
            PrintCommand::Routing(Routing::RoutingStatsMemory),
            PrintCommand::Ip(Ip::IpsecPolicy),
            PrintCommand::Ip(Ip::IpsecProfile),
            PrintCommand::Ip(Ip::IpsecProposal),
            PrintCommand::Ip(Ip::IpsecStatistics),
        ] {
            assert!(is_unsupported_command_trap(command, SUPPORT_TRAP));
        }
    }

    #[test]
    fn support_trap_is_not_unsupported_for_required_commands() {
        assert!(!is_unsupported_command_trap(
            PrintCommand::System(System::Identity),
            SUPPORT_TRAP,
        ));
    }

    #[test]
    fn no_such_item_is_only_unsupported_for_firewall_connections() {
        let command = PrintCommand::Ip(Ip::FirewallConnection);
        assert!(is_unsupported_command_trap(command, "no such item"));
        assert!(is_unsupported_command_trap(command, "no such item (4)"));
        assert!(!is_unsupported_command_trap(
            PrintCommand::System(System::Identity),
            "no such item",
        ));
    }

    #[test]
    fn endpoint_failure_preserves_command_and_kind() {
        let snapshot: EndpointSnapshot<Vec<Identity>> = endpoint_failure(
            "/system/identity/print".to_owned(),
            EndpointErrorKind::Timeout,
            "command exceeded 5s".to_owned(),
        );

        assert!(snapshot.data.is_empty());
        let error = snapshot.error.expect("failure must be preserved");
        assert_eq!(error.kind, EndpointErrorKind::Timeout);
        assert_eq!(error.command, "/system/identity/print");
    }

    #[test]
    fn traps_are_classified_for_public_consumers() {
        let command = PrintCommand::Interface(Interface::DetectInternet);
        assert_eq!(classify_trap(command, SUPPORT_TRAP), EndpointErrorKind::Unsupported);
        assert_eq!(
            classify_trap(command, "not enough permissions (9)"),
            EndpointErrorKind::PermissionDenied
        );
        assert_eq!(
            classify_trap(command, "operation failed (1)"),
            EndpointErrorKind::RouterOsTrap
        );
    }
}
