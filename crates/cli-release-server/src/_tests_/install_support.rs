//! Archive and shell fixtures for generated installer tests.

use std::{
    env, fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use flate2::{Compression, write::GzEncoder};
use tempfile::TempDir;

use super::{script_for, test_config};

/// Creates an archive with a parent-directory traversal member.
pub(super) fn archive_with_traversal_member(path: &Path, binary: &str) {
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

/// Calculates the digest consumed by the generated installer.
pub(super) fn sha256(path: &Path) -> String {
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

/// Writes a curl stub that serves release metadata and an archive.
pub(super) fn write_curl_shim(path: &Path, archive: &Path, sha256: &str) {
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

/// Writes a uname stub for one supported target.
pub(super) fn write_uname_shim(path: &Path) {
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

/// Writes a curl stub that fails before installation.
pub(super) fn write_failing_curl_shim(path: &Path) {
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

/// Sets executable permissions on a fixture file.
pub(super) fn make_executable(path: &Path) {
    let mut permissions = fs::metadata(path)
        .expect("executable metadata should read")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("executable mode should set");
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

/// Provides a safe archive and records calls to its installed shell binary.
pub(super) struct InstallerFixture {
    temp: TempDir,
    /// The configured installation path, including spaces.
    pub(super) install_dir: PathBuf,
    /// The temporary directory used by the generated installer.
    pub(super) tmp_root: PathBuf,
    /// One record for each invocation of the installed binary.
    pub(super) calls: PathBuf,
    /// NUL-separated arguments recorded by the installed binary.
    pub(super) arguments: PathBuf,
}

impl InstallerFixture {
    /// Creates a supported installation fixture with the requested binary status.
    pub(super) fn new(binary: &str, exit_status: u8) -> Self {
        let temp = tempfile::tempdir().expect("tempdir should exist");
        let bin_dir = temp.path().join("bin");
        let install_dir = temp.path().join("installed binaries");
        let tmp_root = temp.path().join("runtime");
        let calls = temp.path().join("calls");
        let arguments = temp.path().join("arguments");
        for dir in [&bin_dir, &install_dir, &tmp_root] {
            fs::create_dir_all(dir).expect("fixture directory should create");
        }
        let binary_script = format!(
            "#!/bin/sh\nset -eu\nprintf 'run\\n' >> {}\nprintf '%s\\0' \"$@\" >> {}\nexit {exit_status}\n",
            shell_quote(&calls.display().to_string()),
            shell_quote(&arguments.display().to_string()),
        );
        let archive = temp.path().join("fixture.tar.gz");
        let file = fs::File::create(&archive).expect("archive should create");
        let mut encoder = GzEncoder::new(file, Compression::default());
        append_archive_file(
            &mut encoder,
            &format!("package/{binary}"),
            binary_script.as_bytes(),
        );
        encoder
            .write_all(&[0; 1024])
            .expect("tar trailer should write");
        encoder.finish().expect("gzip archive should finish");
        write_curl_shim(&bin_dir.join("curl"), &archive, &sha256(&archive));
        write_uname_shim(&bin_dir.join("uname"));
        fs::write(
            temp.path().join("install.sh"),
            script_for(&test_config(), binary).expect("script should generate"),
        )
        .expect("install script should write");
        Self {
            temp,
            install_dir,
            tmp_root,
            calls,
            arguments,
        }
    }

    /// Runs the generated installer with exact argument boundaries.
    pub(super) fn run(&self, args: &[&str]) -> Output {
        Command::new("sh")
            .arg(self.temp.path().join("install.sh"))
            .args(args)
            .current_dir(self.temp.path())
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    self.temp.path().join("bin").display(),
                    env::var("PATH").expect("PATH should be set")
                ),
            )
            .env("TMPDIR", format!("{}/", self.tmp_root.display()))
            .env("CLI_RELEASE_INSTALL_DIR", &self.install_dir)
            .output()
            .expect("install script should run")
    }
}
