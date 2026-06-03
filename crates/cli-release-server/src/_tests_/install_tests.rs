use std::{env, fs, io::Write, os::unix::fs::PermissionsExt, path::Path, process::Command};

use flate2::{Compression, write::GzEncoder};

use crate::config::InstallConfig;

use super::*;

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

fn archive_with_traversal_member(path: &Path, binary: &str) {
    let file = fs::File::create(path).expect("archive should create");
    let mut encoder = GzEncoder::new(file, Compression::default());
    append_archive_file(&mut encoder, "../outside-marker", b"owned");
    append_archive_file(&mut encoder, &format!("package/{binary}"), b"safe");
    encoder
        .write_all(&[0; 1024])
        .expect("tar trailer should write");
    encoder.finish().expect("gzip archive should finish");
}

fn append_archive_file(archive: &mut GzEncoder<fs::File>, path: &str, bytes: &[u8]) {
    let mut header = [0_u8; 512];
    write_tar_bytes(&mut header[0..100], path.as_bytes());
    write_tar_octal(&mut header[100..108], 0o755);
    write_tar_octal(&mut header[108..116], 0);
    write_tar_octal(&mut header[116..124], 0);
    write_tar_octal(&mut header[124..136], bytes.len() as u64);
    write_tar_octal(&mut header[136..148], 0);
    header[148..156].fill(b' ');
    header[156] = b'0';
    write_tar_bytes(&mut header[257..263], b"ustar\0");
    write_tar_bytes(&mut header[263..265], b"00");
    let checksum = header.iter().map(|byte| u64::from(*byte)).sum();
    write_tar_checksum(&mut header[148..156], checksum);

    archive.write_all(&header).expect("tar header should write");
    archive.write_all(bytes).expect("tar body should write");
    let padding = (512 - (bytes.len() % 512)) % 512;
    if padding > 0 {
        archive
            .write_all(&vec![0; padding])
            .expect("tar padding should write");
    }
}

fn write_tar_bytes(field: &mut [u8], value: &[u8]) {
    let len = value.len().min(field.len());
    field[..len].copy_from_slice(&value[..len]);
}

fn write_tar_octal(field: &mut [u8], value: u64) {
    let octal = format!("{value:0width$o}\0", width = field.len() - 1);
    write_tar_bytes(field, octal.as_bytes());
}

fn write_tar_checksum(field: &mut [u8], value: u64) {
    let octal = format!("{value:06o}\0 ");
    write_tar_bytes(field, octal.as_bytes());
}

fn sha256(path: &Path) -> String {
    let output = Command::new("sh")
        .arg("-c")
        .arg(
            "if command -v sha256sum >/dev/null 2>&1; then sha256sum \"$1\"; else shasum -a 256 \"$1\"; fi",
        )
        .arg("sha256")
        .arg(path)
        .output()
        .expect("sha256 command should run");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .expect("sha256 output should be utf-8")
        .split_whitespace()
        .next()
        .expect("sha256 output should include digest")
        .to_owned()
}

fn write_curl_shim(path: &Path, archive: &Path, sha256: &str) {
    let metadata = format!(
        r#"{{"version":"1.2.3","asset_name":"example-cli.tar.gz","download_url":"/archive.tar.gz","sha256":"{sha256}"}}"#
    );
    write_executable(
        path,
        &format!(
            r#"#!/bin/sh
set -eu
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-o" ]; then
    shift
    out="$1"
  fi
  shift || true
done
if [ -n "$out" ]; then
  cp {} "$out"
else
  printf '%s\n' {}
fi
"#,
            shell_quote(&archive.display().to_string()),
            shell_quote(&metadata)
        ),
    );
}

fn write_uname_shim(path: &Path) {
    write_executable(
        path,
        r#"#!/bin/sh
set -eu
case "$1" in
  -s) printf '%s\n' Darwin ;;
  -m) printf '%s\n' x86_64 ;;
esac
"#,
    );
}

fn write_failing_curl_shim(path: &Path) {
    write_executable(
        path,
        r#"#!/bin/sh
set -eu
exit 22
"#,
    );
}

fn write_executable(path: &Path, content: &str) {
    fs::write(path, content).expect("executable should write");
    make_executable(path);
}

fn make_executable(path: &Path) {
    let mut permissions = fs::metadata(path)
        .expect("executable metadata should read")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("executable mode should set");
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}
