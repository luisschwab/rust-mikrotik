//! `RouterOS` SNMP print command paths.

use core::fmt;

/// `RouterOS` print command `/snmp/print`.
const SNMP_SNMP_PRINT: &str = "/snmp/print";

/// `RouterOS` print command `/snmp/community/print`.
const SNMP_SNMP_COMMUNITY_PRINT: &str = "/snmp/community/print";

/// `RouterOS` print commands in this command family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Snmp {
    /// `RouterOS` print command.
    Snmp,
    /// `RouterOS` print command.
    SnmpCommunity,
}

impl Snmp {
    /// All command variants in generated order.
    pub const ALL: &[Self] = &[Self::Snmp, Self::SnmpCommunity];

    /// Return the `RouterOS` API command path.
    pub const fn as_path(self) -> &'static str {
        match self {
            Self::Snmp => SNMP_SNMP_PRINT,
            Self::SnmpCommunity => SNMP_SNMP_COMMUNITY_PRINT,
        }
    }
}

impl fmt::Display for Snmp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_path())
    }
}
