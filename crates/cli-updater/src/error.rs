//! Updater error types.

use std::path::PathBuf;

use thiserror::Error;

/// Result alias for updater operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced while checking for or applying CLI updates.
#[derive(Debug, Error)]
pub enum Error {
    /// The current platform does not have a release asset target.
    #[error("[cli_updator/target] unsupported platform {arch}-{os}")]
    UnsupportedPlatform {
        /// `std::env::consts::ARCH`.
        arch: &'static str,
        /// `std::env::consts::OS`.
        os: &'static str,
    },
    /// The release-server URL is invalid.
    #[error("[cli_updator/client] invalid release server url `{url}`")]
    InvalidReleaseServerUrl {
        /// Invalid URL.
        url: String,
    },
    /// The release server returned a non-success response.
    #[error("[cli_updator/client] release server returned {status} for {url}")]
    ReleaseServerStatus {
        /// Requested URL.
        url: String,
        /// HTTP status code.
        status: u16,
    },
    /// The release-server request failed.
    #[error("[cli_updator/client] release server request failed for {url}: {source}")]
    ReleaseServerRequest {
        /// Requested URL.
        url: String,
        /// Request failure.
        source: reqwest::Error,
    },
    /// The release-server HTTP client failed to build.
    #[error("[cli_updator/client] release server HTTP client build failed: {source}")]
    ReleaseServerClientBuild {
        /// Client build failure.
        source: reqwest::Error,
    },
    /// The release-server response body could not be decoded.
    #[error("[cli_updator/client] release server response decode failed for {url}: {source}")]
    ReleaseServerDecode {
        /// Requested URL.
        url: String,
        /// Decode failure.
        source: reqwest::Error,
    },
    /// The current binary path could not be determined.
    #[error("[cli_updator/install] failed to resolve current executable: {source}")]
    CurrentExe {
        /// Source IO error.
        source: std::io::Error,
    },
    /// A filesystem operation failed.
    #[error("[cli_updator/install] filesystem operation `{operation}` failed at {path}: {source}")]
    FileSystem {
        /// Operation label.
        operation: &'static str,
        /// Path that failed.
        path: PathBuf,
        /// Source IO error.
        source: std::io::Error,
    },
    /// Archive extraction failed.
    #[error("[cli_updator/install] archive extraction failed: {source}")]
    Archive {
        /// Source IO error.
        source: std::io::Error,
    },
    /// The downloaded archive did not contain the requested binary.
    #[error("[cli_updator/install] archive did not contain binary `{binary}`")]
    BinaryMissingInArchive {
        /// Missing binary name.
        binary: String,
    },
    /// The current executable path has no parent directory.
    #[error("[cli_updator/install] executable path has no parent directory: {path}")]
    MissingExecutableParent {
        /// Invalid executable path.
        path: PathBuf,
    },
    /// The current executable path has no file name.
    #[error("[cli_updator/install] executable path has no file name: {path}")]
    MissingExecutableFileName {
        /// Invalid executable path.
        path: PathBuf,
    },
    /// A release version could not be parsed as semantic version.
    #[error("[cli_updator/version] invalid semantic version `{version}`: {source}")]
    InvalidVersion {
        /// Invalid version.
        version: String,
        /// Parse failure.
        source: semver::Error,
    },
    /// A release checksum did not match the downloaded archive.
    #[error("[cli_updator/install] checksum mismatch for {asset_name}")]
    ChecksumMismatch {
        /// Archive asset name.
        asset_name: String,
    },
}
