//! Shared release metadata exchanged between CLIs and the release server.

use serde::{Deserialize, Serialize};

/// Metadata for one downloadable CLI release asset.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReleaseAssetInfo {
    /// Binary name, such as `example-cli` or `example-admin`.
    pub binary: String,
    /// Semantic version without a leading `v`.
    pub version: String,
    /// Rust release target triple.
    pub target: String,
    /// Archive asset name in the release.
    pub asset_name: String,
    /// Server-relative or absolute URL used to download the archive.
    pub download_url: String,
    /// Optional SHA-256 checksum for the archive.
    pub sha256: Option<String>,
}

/// Latest release index for all assets known to the release server.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReleaseIndex {
    /// Release assets available for CLI updates.
    pub releases: Vec<ReleaseAssetInfo>,
}

#[cfg(test)]
#[path = "_tests_/lib_tests.rs"]
mod lib_tests;
