//! Update status and upgrade orchestration.

use std::path::PathBuf;

use cli_release_interface::ReleaseAssetInfo;
use semver::Version;

use crate::{DynBinaryInstaller, DynReleaseClient, Error, Result};

/// Default release-server override environment variable.
pub const DEFAULT_RELEASE_SERVER_URL_ENV: &str = "CLI_RELEASE_SERVER_URL";

/// Inputs for checking one binary.
#[derive(Clone, Debug)]
pub struct UpdateCheck {
    /// Binary name.
    pub binary: String,
    /// Current installed version without a leading `v`.
    pub current_version: String,
    /// Release target triple.
    pub target: String,
}

/// Inputs for applying an upgrade.
#[derive(Clone, Debug)]
pub struct UpdateRequest {
    /// Version and target information to check.
    pub check: UpdateCheck,
    /// Path to the currently running executable.
    pub current_exe: PathBuf,
    /// Install even when the latest version is not newer.
    pub force: bool,
}

/// Result of an update status check.
#[derive(Clone, Debug)]
pub struct UpdateStatus {
    /// Binary name.
    pub binary: String,
    /// Current installed version.
    pub current_version: String,
    /// Release target triple.
    pub target: String,
    /// Latest matching release asset.
    pub latest: ReleaseAssetInfo,
    /// Whether the latest version is newer than the current version.
    pub update_available: bool,
}

/// Result of an explicit upgrade request.
#[derive(Clone, Debug)]
pub enum UpgradeOutcome {
    /// The binary was already current and no install occurred.
    UpToDate(UpdateStatus),
    /// The latest binary was downloaded and installed.
    Installed(UpdateStatus),
}

/// Coordinates release checks and binary installation.
pub struct Updater {
    release_client: DynReleaseClient,
    installer: DynBinaryInstaller,
}

impl Updater {
    /// Creates an updater with explicit dependencies.
    pub fn new(release_client: DynReleaseClient, installer: DynBinaryInstaller) -> Self {
        Self {
            release_client,
            installer,
        }
    }

    /// Checks whether a newer release is available.
    pub async fn status(&self, check: UpdateCheck) -> Result<UpdateStatus> {
        let latest = self
            .release_client
            .latest(&check.binary, &check.target)
            .await?;
        let current_version = parse_version(&check.current_version)?;
        let latest_version = parse_version(&latest.version)?;
        Ok(UpdateStatus {
            binary: check.binary,
            current_version: check.current_version,
            target: check.target,
            latest,
            update_available: latest_version > current_version,
        })
    }

    /// Applies an explicit upgrade when a newer version is available.
    pub async fn upgrade(&self, request: UpdateRequest) -> Result<UpgradeOutcome> {
        let status = self.status(request.check).await?;
        if !status.update_available && !request.force {
            return Ok(UpgradeOutcome::UpToDate(status));
        }
        let archive = self.release_client.download(&status.latest).await?;
        self.installer.install(
            &archive,
            &status.binary,
            &request.current_exe,
            status.latest.sha256.as_deref(),
            &status.latest.asset_name,
        )?;
        Ok(UpgradeOutcome::Installed(status))
    }
}

/// Formats a concise human-readable update status.
pub fn format_status_summary(status: &UpdateStatus) -> String {
    if status.update_available {
        return format!(
            "update available: {} {} -> {} ({})",
            status.binary, status.current_version, status.latest.version, status.target
        );
    }
    format!(
        "{} is up to date ({})",
        status.binary, status.current_version
    )
}

/// Resolves a release server from an explicit flag, env override, then caller default.
pub fn resolve_release_server_url(
    explicit: Option<String>,
    env_var_name: &str,
    default_url: &str,
) -> String {
    explicit
        .or_else(|| {
            std::env::var(env_var_name)
                .ok()
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| default_url.to_owned())
}

fn parse_version(version: &str) -> Result<Version> {
    Version::parse(version).map_err(|source| Error::InvalidVersion {
        version: version.to_owned(),
        source,
    })
}

#[cfg(test)]
#[path = "_tests_/manager_tests.rs"]
mod manager_tests;
