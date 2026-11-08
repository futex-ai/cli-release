//! Native archive and release-source fixtures for installer process checks.

use std::{
    env, fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

use cli_release_server::{config::InstallConfig, install::script_for};
use tempfile::TempDir;

/// Provides the generated script and a real archive with a native probe.
pub(super) struct InstallerFixture {
    temp: TempDir,
    archive: PathBuf,
    metadata: PathBuf,
}

/// Holds each shell's separate installation and observation paths.
pub(super) struct InstallPaths {
    /// Directory that receives the selected binary.
    pub(super) install_dir: PathBuf,
    /// Parent of the generated installer's temporary directory.
    pub(super) tmp_root: PathBuf,
    /// NUL-separated native probe arguments.
    pub(super) arguments: PathBuf,
    /// Captured installer diagnostics.
    pub(super) stderr: PathBuf,
}

impl InstallerFixture {
    /// Compiles the probe and packages it without changing archive contents.
    /// GNU tar isolates this handoff check from the separate BusyBox gzip defect.
    pub(super) fn new() -> Self {
        let temp = tempfile::tempdir().expect("fixture directory should create");
        let package = temp.path().join("package");
        let bin = temp.path().join("bin");
        fs::create_dir(&package).expect("package should create");
        fs::create_dir(&bin).expect("fixture bin should create");
        let compiler = env::var_os("CC").unwrap_or_else(|| "cc".into());
        let probe_source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/installer_handoff/probe.c");
        let compile = Command::new(compiler)
            .arg("-std=c99")
            .arg(&probe_source)
            .arg("-o")
            .arg(package.join("example-cli"))
            .output()
            .expect("C compiler must be available");
        assert!(
            compile.status.success(),
            "native probe must compile: {}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let tar = gnu_tar();
        let archive = temp.path().join("fixture.tar.gz");
        assert!(
            Command::new(&tar)
                .arg("-czf")
                .arg(&archive)
                .arg("-C")
                .arg(temp.path())
                .arg("package/example-cli")
                .status()
                .expect("GNU tar should package the probe")
                .success()
        );
        let digest = Command::new("sha256sum")
            .arg(&archive)
            .output()
            .expect("sha256sum must be available");
        assert!(digest.status.success(), "archive checksum should calculate");
        let digest = String::from_utf8(digest.stdout).expect("digest should be UTF-8");
        let digest = digest.split_whitespace().next().expect("archive digest");
        let metadata = temp.path().join("metadata.json");
        fs::write(
            &metadata,
            format!(
                r#"{{"version":"1.2.3","asset_name":"fixture.tar.gz","download_url":"/archive.tar.gz","sha256":"{digest}"}}"#
            ),
        )
        .expect("release metadata should write");
        fs::write(
            bin.join("curl"),
            r#"#!/bin/sh
set -eu
case "$#" in
  2) cat "$HANDOFF_METADATA" ;;
  4) [ "$3" = '-o' ]; cp "$HANDOFF_ARCHIVE" "$4" ;;
  *) exit 85 ;;
esac
"#,
        )
        .expect("fixture release source should write");
        fs::set_permissions(bin.join("curl"), fs::Permissions::from_mode(0o755))
            .expect("release source should be executable");
        let config = InstallConfig::new(
            Vec::new(),
            None,
            vec!["example-cli".to_owned()],
            Some("https://releases.example.com".to_owned()),
            "CLI_RELEASE_SERVER_URL".to_owned(),
            "CLI_RELEASE_INSTALL_DIR".to_owned(),
        )
        .expect("install configuration should be valid");
        let script = script_for(&config, "example-cli").expect("script should generate");
        let script = format!("tar() {{ {} \"$@\"; }}\n{script}", shell_quote(&tar));
        fs::write(temp.path().join("install.sh"), script).expect("script should write");
        Self {
            temp,
            archive,
            metadata,
        }
    }

    /// Configures one required shell with isolated files and exact arguments.
    pub(super) fn command(
        &self,
        program: &str,
        shell_args: &[&str],
        arguments: &[&str],
    ) -> (Command, InstallPaths) {
        let root = self.temp.path().join(program);
        fs::create_dir(&root).expect("shell fixture should create");
        let paths = InstallPaths {
            install_dir: root.join("installed binaries"),
            tmp_root: root.join("runtime"),
            arguments: root.join("arguments"),
            stderr: root.join("stderr"),
        };
        fs::create_dir(&paths.tmp_root).expect("runtime directory should create");
        let mut command = Command::new(program);
        command
            .args(shell_args)
            .arg(self.temp.path().join("install.sh"))
            .args(arguments)
            .current_dir(&root)
            .env_clear()
            .env("LC_ALL", "C")
            .env("TZ", "UTC")
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    self.temp.path().join("bin").display(),
                    env::var("PATH").expect("PATH must be set")
                ),
            )
            .env("TMPDIR", &paths.tmp_root)
            .env("CLI_RELEASE_INSTALL_DIR", &paths.install_dir)
            .env("HANDOFF_METADATA", &self.metadata)
            .env("HANDOFF_ARCHIVE", &self.archive)
            .env("HANDOFF_ARGUMENTS", &paths.arguments)
            .env("HANDOFF_STATUS", "37")
            .stderr(fs::File::create(&paths.stderr).expect("stderr should capture"));
        (command, paths)
    }
}

fn gnu_tar() -> PathBuf {
    let path = Command::new("sh")
        .args(["-c", "command -v tar"])
        .output()
        .expect("tar should resolve");
    assert!(path.status.success(), "GNU tar must be available");
    let path = String::from_utf8(path.stdout).expect("tar path should be UTF-8");
    let path = fs::canonicalize(path.trim()).expect("tar path should be absolute");
    let version = Command::new(&path)
        .arg("--version")
        .output()
        .expect("tar version should read");
    assert!(
        String::from_utf8_lossy(&version.stdout).contains("GNU tar"),
        "fixture requires GNU tar"
    );
    path
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', r#"'\''"#))
}
