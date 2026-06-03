//! Lightweight HTTP release server for private CLI release assets.

use std::sync::Arc;

use async_trait::async_trait;
use cli_release_interface::{ReleaseAssetInfo, ReleaseIndex};
use thiserror::Error;

pub mod config;
pub mod github;
pub mod install;
pub mod logging;
pub mod routes;

/// Shared release provider.
pub type DynReleaseProvider = Arc<dyn ReleaseProvider + Send + Sync>;

/// Release provider used by HTTP routes.
#[async_trait]
pub trait ReleaseProvider {
    /// Returns the latest release index for configured binaries.
    async fn index(&self, binaries: &[String]) -> Result<ReleaseIndex>;

    /// Returns the latest release asset for one binary and target.
    async fn latest(&self, binary: &str, target: &str) -> Result<ReleaseAssetInfo>;

    /// Downloads an archive for a binary, version, and target.
    async fn download(&self, binary: &str, version: &str, target: &str) -> Result<ReleaseDownload>;
}

/// Downloaded release archive.
pub struct ReleaseDownload {
    /// Archive asset name.
    pub asset_name: String,
    /// Archive bytes.
    pub bytes: Vec<u8>,
}

/// Release-server errors.
#[derive(Debug, Error)]
pub enum Error {
    /// Release server has not been configured with a provider.
    #[error("[cli_release_server] release server is not configured")]
    NotConfigured,
    /// URL path component failed validation.
    #[error("[cli_release_server/routes] invalid release path component `{value}`")]
    InvalidPathComponent { value: String },
    /// Requested binary is not published through install-script endpoints.
    #[error("[cli_release_server/install] unsupported install binary `{binary}`")]
    UnsupportedInstallBinary { binary: String },
    /// Install-script routes need a configured default binary.
    #[error("[cli_release_server/config] missing default install binary")]
    MissingDefaultInstallBinary,
    /// Install-script rendering needs a public release-server URL.
    #[error("[cli_release_server/config] missing public release server URL")]
    MissingPublicReleaseServerUrl,
    /// Default install binary is not in the configured binary allowlist.
    #[error("[cli_release_server/config] default binary `{binary}` is not configured")]
    DefaultBinaryNotConfigured { binary: String },
    /// GitHub-backed deployments need at least one configured release binary.
    #[error("[cli_release_server/config] GitHub release provider requires configured binaries")]
    MissingReleaseBinaries,
    /// Environment variable name failed validation.
    #[error("[cli_release_server/config] invalid environment variable name `{value}`")]
    InvalidEnvVarName { value: String },
    /// Configured install binary failed route path-component validation.
    #[error("[cli_release_server/config] invalid install binary name `{binary}`")]
    InvalidInstallBinaryName { binary: String },
    /// GitHub repository and token configuration is incomplete.
    #[error(
        "[cli_release_server/config] incomplete GitHub configuration: set both {repository_env} and {token_env}, or neither"
    )]
    IncompleteGitHubConfig {
        /// Repository environment variable name.
        repository_env: &'static str,
        /// Token environment variable name.
        token_env: &'static str,
    },
    /// Asset template failed validation.
    #[error("[cli_release_server/github] invalid asset template `{template}`")]
    InvalidAssetTemplate { template: String },
    /// A requested release asset is missing.
    #[error("[cli_release_server/github] missing release asset `{asset_name}`")]
    MissingAsset { asset_name: String },
    /// GitHub API request failed.
    #[error("[cli_release_server/github] GitHub API request failed for {url}: {source}")]
    GitHubRequest { url: String, source: reqwest::Error },
    /// GitHub HTTP client failed to build.
    #[error("[cli_release_server/github] GitHub HTTP client build failed: {source}")]
    GitHubClientBuild { source: reqwest::Error },
    /// GitHub API returned a non-success response.
    #[error("[cli_release_server/github] GitHub API returned {status} for {url}")]
    GitHubStatus { url: String, status: u16 },
    /// GitHub API response failed to decode.
    #[error("[cli_release_server/github] GitHub API response decode failed for {url}: {source}")]
    GitHubDecode { url: String, source: reqwest::Error },
    /// GitHub release tag did not match the workspace release convention.
    #[error("[cli_release_server/github] invalid GitHub release tag `{tag}`")]
    InvalidReleaseTag { tag: String },
    /// A checksum companion asset did not contain a valid SHA-256 digest.
    #[error("[cli_release_server/github] invalid checksum asset `{asset_name}`")]
    InvalidChecksumAsset { asset_name: String },
    /// A GitHub release asset digest was present but not a valid SHA-256 digest.
    #[error("[cli_release_server/github] invalid asset digest for `{asset_name}`: `{digest}`")]
    InvalidAssetDigest { asset_name: String, digest: String },
}

/// Result alias for release-server operations.
pub type Result<T> = std::result::Result<T, Error>;
