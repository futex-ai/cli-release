//! Binary archive installation.

use std::{
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};

use crate::{Error, Result};

/// Shared installer trait object.
pub type DynBinaryInstaller = Arc<dyn BinaryInstaller + Send + Sync>;

/// Installs an extracted binary over the current executable.
#[unimock::unimock(api = BinaryInstallerMock)]
pub trait BinaryInstaller {
    /// Installs `archive_bytes` for `binary` at `current_exe`.
    fn install(
        &self,
        archive_bytes: &[u8],
        binary: &str,
        current_exe: &Path,
        expected_sha256: Option<&str>,
        asset_name: &str,
    ) -> Result<()>;
}

/// Installer for `.tar.gz` release archives.
#[derive(Clone, Debug, Default)]
pub struct TarGzBinaryInstaller;

impl TarGzBinaryInstaller {
    /// Creates the production archive installer.
    pub fn new() -> Self {
        Self
    }
}

impl BinaryInstaller for TarGzBinaryInstaller {
    fn install(
        &self,
        archive_bytes: &[u8],
        binary: &str,
        current_exe: &Path,
        expected_sha256: Option<&str>,
        asset_name: &str,
    ) -> Result<()> {
        verify_checksum(archive_bytes, expected_sha256, asset_name)?;
        let temp_dir = tempfile::Builder::new()
            .prefix("cli-upgrade-")
            .tempdir()
            .map_err(|source| Error::FileSystem {
                operation: "create_temp_dir",
                path: std::env::temp_dir(),
                source,
            })?;
        let extracted = extract_binary(archive_bytes, binary, temp_dir.path())?;
        replace_executable(&extracted, current_exe)?;
        Ok(())
    }
}

fn verify_checksum(
    archive_bytes: &[u8],
    expected_sha256: Option<&str>,
    asset_name: &str,
) -> Result<()> {
    let Some(expected) = expected_sha256 else {
        return Ok(());
    };
    let mut hasher = Sha256::new();
    hasher.update(archive_bytes);
    let actual = hasher.finalize();
    let actual = actual
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if actual.eq_ignore_ascii_case(expected.trim()) {
        return Ok(());
    }
    Err(Error::ChecksumMismatch {
        asset_name: asset_name.to_owned(),
    })
}

fn extract_binary(archive_bytes: &[u8], binary: &str, temp_dir: &Path) -> Result<PathBuf> {
    let gz = GzDecoder::new(Cursor::new(archive_bytes));
    let mut archive = tar::Archive::new(gz);
    let entries = archive
        .entries()
        .map_err(|source| Error::Archive { source })?;

    for entry in entries {
        let mut entry = entry.map_err(|source| Error::Archive { source })?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry.path().map_err(|source| Error::Archive { source })?;
        if path.file_name().and_then(|name| name.to_str()) != Some(binary) {
            continue;
        }
        let destination = temp_dir.join(binary);
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|source| Error::Archive { source })?;
        fs::write(&destination, bytes).map_err(|source| Error::FileSystem {
            operation: "write_extracted_binary",
            path: destination.clone(),
            source,
        })?;
        set_executable_permissions(&destination)?;
        return Ok(destination);
    }

    Err(Error::BinaryMissingInArchive {
        binary: binary.to_owned(),
    })
}

fn replace_executable(extracted: &Path, current_exe: &Path) -> Result<()> {
    let parent = current_exe
        .parent()
        .ok_or_else(|| Error::MissingExecutableParent {
            path: current_exe.to_path_buf(),
        })?;
    let file_name = current_exe
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::MissingExecutableFileName {
            path: current_exe.to_path_buf(),
        })?;
    let backup = parent.join(format!("{file_name}.old"));

    if backup.exists() {
        fs::remove_file(&backup).map_err(|source| Error::FileSystem {
            operation: "remove_stale_backup",
            path: backup.clone(),
            source,
        })?;
    }

    fs::rename(current_exe, &backup).map_err(|source| Error::FileSystem {
        operation: "backup_current_executable",
        path: current_exe.to_path_buf(),
        source,
    })?;

    let install_result = fs::copy(extracted, current_exe)
        .map_err(|source| Error::FileSystem {
            operation: "copy_new_executable",
            path: current_exe.to_path_buf(),
            source,
        })
        .and_then(|_| set_executable_permissions(current_exe));

    if let Err(error) = install_result {
        let _ = fs::rename(&backup, current_exe);
        return Err(error);
    }

    fs::remove_file(&backup).map_err(|source| Error::FileSystem {
        operation: "remove_backup",
        path: backup,
        source,
    })?;
    Ok(())
}

#[cfg(unix)]
fn set_executable_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .map_err(|source| Error::FileSystem {
            operation: "read_permissions",
            path: path.to_path_buf(),
            source,
        })?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).map_err(|source| Error::FileSystem {
        operation: "set_permissions",
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(not(unix))]
fn set_executable_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
#[path = "_tests_/installer_tests.rs"]
mod installer_tests;
