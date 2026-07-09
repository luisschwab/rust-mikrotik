//! `RouterOS` binary API command paths.

pub mod interface;
pub mod ip;
pub mod queue;
pub mod routing;
pub mod service;
pub mod snmp;
pub mod system;
pub mod tool;
pub mod user;

#[allow(clippy::wildcard_imports)]
pub use interface::*;
#[allow(clippy::wildcard_imports)]
pub use ip::*;
#[allow(clippy::wildcard_imports)]
pub use queue::*;
#[allow(clippy::wildcard_imports)]
pub use routing::*;
#[allow(clippy::wildcard_imports)]
pub use service::*;
#[allow(clippy::wildcard_imports)]
pub use snmp::*;
#[allow(clippy::wildcard_imports)]
pub use system::*;
#[allow(clippy::wildcard_imports)]
pub use tool::*;
#[allow(clippy::wildcard_imports)]
pub use user::*;

/// Typed print command path grouped by top-level command family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrintCommand {
    /// `/interface` print command.
    Interface(Interface),
    /// `/ip` or `/ipv6` print command.
    Ip(Ip),
    /// `/queue` print command.
    Queue(Queue),
    /// `/routing` print command.
    Routing(Routing),
    /// Service or package-family print command.
    Service(Service),
    /// `/snmp` print command.
    Snmp(Snmp),
    /// `/system` print command.
    System(System),
    /// `/tool` print command.
    Tool(Tool),
    /// `/user` print command.
    User(User),
}

impl PrintCommand {
    /// Return every known print command in generated order.
    #[must_use]
    pub fn all() -> Vec<Self> {
        let mut commands = Vec::with_capacity(Self::count());
        commands.extend(Interface::ALL.iter().copied().map(Self::Interface));
        commands.extend(Ip::ALL.iter().copied().map(Self::Ip));
        commands.extend(Queue::ALL.iter().copied().map(Self::Queue));
        commands.extend(Routing::ALL.iter().copied().map(Self::Routing));
        commands.extend(Service::ALL.iter().copied().map(Self::Service));
        commands.extend(Snmp::ALL.iter().copied().map(Self::Snmp));
        commands.extend(System::ALL.iter().copied().map(Self::System));
        commands.extend(Tool::ALL.iter().copied().map(Self::Tool));
        commands.extend(User::ALL.iter().copied().map(Self::User));
        commands
    }

    /// Return the number of known print commands.
    #[must_use]
    pub const fn count() -> usize {
        Interface::ALL.len()
            + Ip::ALL.len()
            + Queue::ALL.len()
            + Routing::ALL.len()
            + Service::ALL.len()
            + Snmp::ALL.len()
            + System::ALL.len()
            + Tool::ALL.len()
            + User::ALL.len()
    }

    /// Return the `RouterOS` API command path.
    pub const fn as_path(self) -> &'static str {
        match self {
            Self::Interface(command) => command.as_path(),
            Self::Ip(command) => command.as_path(),
            Self::Queue(command) => command.as_path(),
            Self::Routing(command) => command.as_path(),
            Self::Service(command) => command.as_path(),
            Self::Snmp(command) => command.as_path(),
            Self::System(command) => command.as_path(),
            Self::Tool(command) => command.as_path(),
            Self::User(command) => command.as_path(),
        }
    }
}

mikrotik_common::impl_command_display!(PrintCommand);

#[cfg(test)]
mod tests {
    #[test]
    fn representative_print_commands_match_routeros_paths() {
        assert_eq!(super::Ip::Route.as_path(), "/ip/route/print");
        assert_eq!(super::System::Resource.to_string(), "/system/resource/print");
        assert_eq!(
            super::PrintCommand::Interface(super::Interface::WireGuardPeer).as_path(),
            "/interface/wireguard/peers/print"
        );
    }
}
