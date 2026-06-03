//! Shared updater for released CLI binaries.

mod client;
mod error;
mod installer;
mod manager;
mod target;

pub use client::{DynReleaseClient, HttpReleaseClient, ReleaseClient};
pub use error::{Error, Result};
pub use installer::{BinaryInstaller, DynBinaryInstaller, TarGzBinaryInstaller};
pub use manager::{
    DEFAULT_RELEASE_SERVER_URL_ENV, UpdateCheck, UpdateRequest, UpdateStatus, Updater,
    UpgradeOutcome, format_status_summary, resolve_release_server_url,
};
pub use target::current_target;
