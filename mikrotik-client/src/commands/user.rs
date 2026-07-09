//! `RouterOS` user print command paths.

/// `RouterOS` print command `/user/active/print`.
const USER_ACTIVE_USER_PRINT: &str = "/user/active/print";

/// `RouterOS` print command `/user/print`.
const USER_USER_PRINT: &str = "/user/print";

/// `RouterOS` print command `/user/aaa/print`.
const USER_USER_AAA_PRINT: &str = "/user/aaa/print";

/// `RouterOS` print command `/user/group/print`.
const USER_USER_GROUP_PRINT: &str = "/user/group/print";

/// `RouterOS` print command `/user/settings/print`.
const USER_USER_SETTINGS_PRINT: &str = "/user/settings/print";

/// `RouterOS` print commands in this command family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum User {
    /// `RouterOS` print command.
    ActiveUser,
    /// `RouterOS` print command.
    User,
    /// `RouterOS` print command.
    UserAaa,
    /// `RouterOS` print command.
    UserGroup,
    /// `RouterOS` print command.
    UserSettings,
}

impl User {
    /// All command variants in generated order.
    pub const ALL: &[Self] = &[
        Self::ActiveUser,
        Self::User,
        Self::UserAaa,
        Self::UserGroup,
        Self::UserSettings,
    ];

    /// Return the `RouterOS` API command path.
    pub const fn as_path(self) -> &'static str {
        match self {
            Self::ActiveUser => USER_ACTIVE_USER_PRINT,
            Self::User => USER_USER_PRINT,
            Self::UserAaa => USER_USER_AAA_PRINT,
            Self::UserGroup => USER_USER_GROUP_PRINT,
            Self::UserSettings => USER_USER_SETTINGS_PRINT,
        }
    }
}

mikrotik_common::impl_command_display!(User);
