use std::{fs, io::Write};

use flate2::{Compression, write::GzEncoder};
use tar::Builder;

use super::*;

#[test]
fn installer_replaces_current_executable_from_archive() {
    let temp = tempfile::tempdir().expect("tempdir should exist");
    let current = temp.path().join("juno");
    fs::write(&current, b"old").expect("current binary should write");
    let archive = archive_with_binary("juno", b"new");

    TarGzBinaryInstaller::new()
        .install(
            &archive,
            "juno",
            &current,
            None,
            "juno-0.2.0-aarch64-apple-darwin.tar.gz",
        )
        .expect("install should succeed");

    assert_eq!(fs::read(&current).expect("current should read"), b"new");
    assert!(!current.with_file_name("juno.old").exists());
}

#[test]
fn installer_rejects_checksum_mismatch() {
    let temp = tempfile::tempdir().expect("tempdir should exist");
    let current = temp.path().join("juno");
    fs::write(&current, b"old").expect("current binary should write");
    let archive = archive_with_binary("juno", b"new");

    let error = TarGzBinaryInstaller::new()
        .install(
            &archive,
            "juno",
            &current,
            Some("bad"),
            "juno-0.2.0-aarch64-apple-darwin.tar.gz",
        )
        .expect_err("checksum mismatch should fail");

    assert!(matches!(error, Error::ChecksumMismatch { .. }));
    assert_eq!(fs::read(&current).expect("current should read"), b"old");
}

fn archive_with_binary(binary: &str, bytes: &[u8]) -> Vec<u8> {
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut archive = Builder::new(&mut gz);
        let path = format!("{binary}-0.2.0-aarch64-apple-darwin/{binary}");
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        archive
            .append_data(&mut header, path, bytes)
            .expect("archive data should append");
        archive.finish().expect("archive should finish");
    }
    gz.flush().expect("gzip should flush");
    gz.finish().expect("gzip should finish")
}
