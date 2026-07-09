//! `RouterOS` queue print command paths.

/// `RouterOS` print command `/queue/interface/print`.
const QUEUE_QUEUE_INTERFACE_PRINT: &str = "/queue/interface/print";

/// `RouterOS` print command `/queue/type/print`.
const QUEUE_QUEUE_TYPE_PRINT: &str = "/queue/type/print";

/// `RouterOS` print commands in this command family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Queue {
    /// `RouterOS` print command.
    QueueInterface,
    /// `RouterOS` print command.
    QueueType,
}

impl Queue {
    /// All command variants in generated order.
    pub const ALL: &[Self] = &[Self::QueueInterface, Self::QueueType];

    /// Return the `RouterOS` API command path.
    pub const fn as_path(self) -> &'static str {
        match self {
            Self::QueueInterface => QUEUE_QUEUE_INTERFACE_PRINT,
            Self::QueueType => QUEUE_QUEUE_TYPE_PRINT,
        }
    }
}

mikrotik_common::impl_command_display!(Queue);
