//! Platform target resolution for release assets.

use crate::{Error, Result};

/// Returns the release target triple for the current platform.
pub fn current_target() -> Result<&'static str> {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("x86_64", "linux") => Ok("x86_64-unknown-linux-gnu"),
        ("aarch64", "linux") => Ok("aarch64-unknown-linux-gnu"),
        ("x86_64", "macos") => Ok("x86_64-apple-darwin"),
        ("aarch64", "macos") => Ok("aarch64-apple-darwin"),
        (arch, os) => Err(Error::UnsupportedPlatform { arch, os }),
    }
}
