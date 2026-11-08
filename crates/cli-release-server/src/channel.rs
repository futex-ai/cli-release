//! Release channel configuration for latest metadata.

/// Release channel environment variable.
pub const CHANNEL_ENV: &str = "CLI_RELEASE_CHANNEL";

/// Release selection policy for a server deployment.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ReleaseChannel {
    /// Use GitHub's latest full release.
    #[default]
    Stable,
    /// Include prereleases in GitHub's release-list order.
    Prerelease,
}

impl ReleaseChannel {
    /// Parses a configured channel, warning and using stable for unknown values.
    pub fn from_config(value: &str) -> Self {
        match value {
            "" | "stable" => Self::Stable,
            "prerelease" => Self::Prerelease,
            channel => {
                tracing::warn!(channel, "unknown release channel, using stable");
                Self::Stable
            }
        }
    }
}

pub(crate) fn from_env() -> ReleaseChannel {
    let value = std::env::var(CHANNEL_ENV).unwrap_or_default();
    ReleaseChannel::from_config(&value)
}

#[cfg(test)]
#[path = "_tests_/channel_tests.rs"]
mod channel_tests;
