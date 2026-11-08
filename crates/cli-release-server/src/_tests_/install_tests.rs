//! Shell installer rendering and execution tests.

use std::{env, fs, process::Command};

use crate::config::InstallConfig;

use self::support::{
    InstallerFixture, archive_with_traversal_member, make_executable, sha256, write_curl_shim,
    write_failing_curl_shim, write_uname_shim,
};
use super::*;

#[path = "install_support.rs"]
mod support;

fn test_config() -> InstallConfig {
    InstallConfig::new(
        vec!["install.example.com".to_owned()],
        Some("example-cli".to_owned()),
        vec!["example-cli".to_owned(), "example-admin".to_owned()],
        Some("https://releases.example.com".to_owned()),
        "CLI_RELEASE_SERVER_URL".to_owned(),
        "CLI_RELEASE_INSTALL_DIR".to_owned(),
    )
    .expect("config should be valid")
}

#[test]
fn detects_install_host() {
    let config = test_config();
    assert!(is_install_host(
        &config,
        Some(&HeaderValue::from_static("install.example.com"))
    ));
    assert!(is_install_host(
        &config,
        Some(&HeaderValue::from_static("install.example.com:443"))
    ));
    assert!(!is_install_host(
        &config,
        Some(&HeaderValue::from_static("releases.example.com"))
    ));
}

#[test]
fn generates_configured_install_script() {
    let config = test_config();
    let script = script_for(&config, "example-cli").expect("script should generate");

    assert!(script.starts_with("#!/bin/sh\nset -eu"));
    assert!(script.contains("binary='example-cli'"));
    assert!(script.contains("https://releases.example.com"));
    assert!(script.contains("CLI_RELEASE_SERVER_URL"));
    assert!(script.contains("CLI_RELEASE_INSTALL_DIR"));
    assert!(script.contains("x86_64-unknown-linux-gnu"));
    assert!(script.contains("aarch64-apple-darwin"));
    assert!(script.contains("/releases/latest/$binary/$target"));
    assert!(script.contains("sha256sum -c -"));
    assert!(script.contains("archive_name=\"${asset_name##*/}\""));
    assert!(script.contains("archive contained unsafe path"));
    assert!(script.contains("tar -tzf \"$archive\""));
    assert!(script.contains("tar -xOf \"$archive\" -- \"$binary_member\""));
}

#[test]
fn generated_installer_rejects_traversal_archive_members() {
    let temp = tempfile::tempdir().expect("tempdir should exist");
    let bin_dir = temp.path().join("bin");
    let install_dir = temp.path().join("install");
    let tmp_root = temp.path().join("runtime");
    fs::create_dir_all(&bin_dir).expect("bin dir should create");
    fs::create_dir_all(&install_dir).expect("install dir should create");
    fs::create_dir_all(&tmp_root).expect("runtime dir should create");

    let archive = temp.path().join("example-cli.tar.gz");
    archive_with_traversal_member(&archive, "example-cli");
    let sha256 = sha256(&archive);
    write_curl_shim(&bin_dir.join("curl"), &archive, &sha256);
    write_uname_shim(&bin_dir.join("uname"));

    let script = script_for(&test_config(), "example-cli").expect("script should generate");
    let script_path = temp.path().join("install.sh");
    fs::write(&script_path, script).expect("install script should write");
    make_executable(&script_path);

    let path = format!(
        "{}:{}",
        bin_dir.display(),
        env::var("PATH").expect("PATH should be set")
    );
    let output = Command::new("sh")
        .arg(&script_path)
        .env("PATH", path)
        .env("TMPDIR", format!("{}/", tmp_root.display()))
        .env("CLI_RELEASE_INSTALL_DIR", &install_dir)
        .output()
        .expect("install script should run");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");

    assert!(!output.status.success());
    assert!(stderr.contains("archive contained unsafe path"));
    assert!(!tmp_root.join("outside-marker").exists());
    assert!(!install_dir.join("example-cli").exists());
}

#[test]
fn generated_installer_quotes_configured_release_server_url() {
    let temp = tempfile::tempdir().expect("tempdir should exist");
    let bin_dir = temp.path().join("bin");
    let marker = temp.path().join("release-url-marker");
    fs::create_dir_all(&bin_dir).expect("bin dir should create");
    write_failing_curl_shim(&bin_dir.join("curl"));
    write_uname_shim(&bin_dir.join("uname"));

    let release_server_url = format!("https://releases.example.com/$(touch {})", marker.display());
    let config = InstallConfig::new(
        Vec::new(),
        None,
        vec!["example-cli".to_owned()],
        Some(release_server_url),
        "CLI_RELEASE_SERVER_URL".to_owned(),
        "CLI_RELEASE_INSTALL_DIR".to_owned(),
    )
    .expect("config should be valid");
    let script = script_for(&config, "example-cli").expect("script should generate");
    let script_path = temp.path().join("install.sh");
    fs::write(&script_path, script).expect("install script should write");
    make_executable(&script_path);

    let path = format!(
        "{}:{}",
        bin_dir.display(),
        env::var("PATH").expect("PATH should be set")
    );
    let output = Command::new("sh")
        .arg(&script_path)
        .env("PATH", path)
        .output()
        .expect("install script should run");

    assert!(!output.status.success());
    assert!(!marker.exists());
}

#[test]
fn rejects_unsupported_binaries() {
    let config = test_config();
    assert!(script_for(&config, "example-server").is_err());
}

#[test]
fn generated_installer_forwards_arguments_once_without_logging_values() {
    let fixture = InstallerFixture::new("example-admin", 0);
    let install_only = fixture.run(&[]);
    assert_eq!(install_only.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(install_only.stdout.clone()).expect("stdout should be utf-8"),
        format!(
            "installed example-admin 1.2.3 to {}/example-admin\n",
            fixture.install_dir.display()
        )
    );
    assert_eq!(
        String::from_utf8(install_only.stderr.clone()).expect("stderr should be utf-8"),
        format!(
            "warning: {} is not on PATH\n",
            fixture.install_dir.display()
        )
    );
    assert!(fixture.install_dir.join("example-admin").is_file());
    assert!(!fixture.calls.exists(), "no arguments must only install");
    assert_eq!(
        fs::read_dir(&fixture.tmp_root)
            .expect("runtime should read")
            .count(),
        0
    );

    let output = fixture.run(&[
        "install",
        "--token",
        "private token 'single' \"double\"",
        "--",
        "",
        "$(touch marker); * ? [x] | & > < `touch marker` \\ $HOME",
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert!(
        fixture.calls.exists(),
        "installed binary must run with arguments"
    );
    assert_eq!(
        fs::read(&fixture.calls).expect("calls should read"),
        b"run\n"
    );
    assert_eq!(
        fs::read(&fixture.arguments).expect("arguments should read"),
        b"install\0--token\0private token 'single' \"double\"\0--\0\0$(touch marker); * ? [x] | & > < `touch marker` \\ $HOME\0"
    );
    assert_eq!(output.stdout, install_only.stdout);
    assert_eq!(output.stderr, install_only.stderr);
    assert_eq!(
        fs::read_dir(&fixture.tmp_root)
            .expect("runtime should read")
            .count(),
        0
    );
}

#[test]
fn generated_installer_preserves_binary_failure_status_and_cleans_up() {
    let fixture = InstallerFixture::new("example-cli", 37);
    let output = fixture.run(&["private failure token"]);

    assert_eq!(
        output.status.code(),
        Some(37),
        "installer must return the binary exit status"
    );
    assert_eq!(
        fs::read(&fixture.calls).expect("calls should read"),
        b"run\n"
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf-8"),
        format!(
            "installed example-cli 1.2.3 to {}/example-cli\n",
            fixture.install_dir.display()
        )
    );
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr should be utf-8"),
        format!(
            "warning: {} is not on PATH\n",
            fixture.install_dir.display()
        )
    );
    assert_eq!(
        fs::read_dir(&fixture.tmp_root)
            .expect("runtime should read")
            .count(),
        0
    );
}
