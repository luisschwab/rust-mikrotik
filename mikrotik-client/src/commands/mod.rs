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

#[cfg(test)]
mod tests {
    #[test]
    fn representative_print_constants_match_routeros_paths() {
        assert_eq!(super::IP_ROUTE_PRINT, "/ip/route/print");
        assert_eq!(super::SYSTEM_RESOURCE_PRINT, "/system/resource/print");
        assert_eq!(
            super::INTERFACE_WIRE_GUARD_PEER_PRINT,
            "/interface/wireguard/peers/print"
        );
    }
}
