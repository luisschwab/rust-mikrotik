//! `RouterOS` queue print command paths.

use core::fmt;

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

impl fmt::Display for Queue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_path())
    }
}
