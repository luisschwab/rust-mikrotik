//! `RouterOS` tool print command paths.

use core::fmt;

/// `RouterOS` print command `/tool/bandwidth-server/print`.
const TOOL_BANDWIDTH_SERVER_PRINT: &str = "/tool/bandwidth-server/print";

/// `RouterOS` print command `/tool/e-mail/print`.
const TOOL_EMAIL_PRINT: &str = "/tool/e-mail/print";

/// `RouterOS` print command `/tool/graphing/print`.
const TOOL_GRAPHING_PRINT: &str = "/tool/graphing/print";

/// `RouterOS` print command `/tool/mac-server/ping/print`.
const TOOL_MAC_SERVER_PING_PRINT: &str = "/tool/mac-server/ping/print";

/// `RouterOS` print command `/tool/romon/print`.
const TOOL_ROMON_PRINT: &str = "/tool/romon/print";

/// `RouterOS` print command `/tool/romon/port/print`.
const TOOL_ROMON_PORT_PRINT: &str = "/tool/romon/port/print";

/// `RouterOS` print command `/tool/sms/print`.
const TOOL_SMS_PRINT: &str = "/tool/sms/print";

/// `RouterOS` print command `/tool/sniffer/print`.
const TOOL_SNIFFER_PRINT: &str = "/tool/sniffer/print";

/// `RouterOS` print command `/tool/traffic-generator/print`.
const TOOL_TRAFFIC_GENERATOR_PRINT: &str = "/tool/traffic-generator/print";

/// `RouterOS` print command `/tool/traffic-generator/stats/latency-distribution/print`.
const TOOL_TRAFFIC_GENERATOR_LATENCY_DISTRIBUTION_PRINT: &str =
    "/tool/traffic-generator/stats/latency-distribution/print";

/// `RouterOS` print commands in this command family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    /// `RouterOS` print command.
    BandwidthServer,
    /// `RouterOS` print command.
    Email,
    /// `RouterOS` print command.
    Graphing,
    /// `RouterOS` print command.
    MacServerPing,
    /// `RouterOS` print command.
    Romon,
    /// `RouterOS` print command.
    RomonPort,
    /// `RouterOS` print command.
    Sms,
    /// `RouterOS` print command.
    Sniffer,
    /// `RouterOS` print command.
    TrafficGenerator,
    /// `RouterOS` print command.
    TrafficGeneratorLatencyDistribution,
}

impl Tool {
    /// All command variants in generated order.
    pub const ALL: &[Self] = &[
        Self::BandwidthServer,
        Self::Email,
        Self::Graphing,
        Self::MacServerPing,
        Self::Romon,
        Self::RomonPort,
        Self::Sms,
        Self::Sniffer,
        Self::TrafficGenerator,
        Self::TrafficGeneratorLatencyDistribution,
    ];

    /// Return the `RouterOS` API command path.
    pub const fn as_path(self) -> &'static str {
        match self {
            Self::BandwidthServer => TOOL_BANDWIDTH_SERVER_PRINT,
            Self::Email => TOOL_EMAIL_PRINT,
            Self::Graphing => TOOL_GRAPHING_PRINT,
            Self::MacServerPing => TOOL_MAC_SERVER_PING_PRINT,
            Self::Romon => TOOL_ROMON_PRINT,
            Self::RomonPort => TOOL_ROMON_PORT_PRINT,
            Self::Sms => TOOL_SMS_PRINT,
            Self::Sniffer => TOOL_SNIFFER_PRINT,
            Self::TrafficGenerator => TOOL_TRAFFIC_GENERATOR_PRINT,
            Self::TrafficGeneratorLatencyDistribution => TOOL_TRAFFIC_GENERATOR_LATENCY_DISTRIBUTION_PRINT,
        }
    }
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_path())
    }
}
