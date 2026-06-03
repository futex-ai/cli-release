use std::path::PathBuf;
use std::sync::Arc;

use cli_release_interface::ReleaseAssetInfo;
use unimock::{MockFn, Unimock, matching};

use crate::client::ReleaseClientMock;

use super::*;

#[test]
fn release_server_url_resolution_uses_consumer_default() {
    assert_eq!(DEFAULT_RELEASE_SERVER_URL_ENV, "CLI_RELEASE_SERVER_URL");
    assert_eq!(
        resolve_release_server_url(
            Some("http://127.0.0.1:8080".to_owned()),
            DEFAULT_RELEASE_SERVER_URL_ENV,
            "https://releases.example.com",
        ),
        "http://127.0.0.1:8080"
    );
    assert_eq!(
        resolve_release_server_url(
            None,
            "CLI_RELEASE_TEST_UNSET_SERVER_URL",
            "https://releases.example.com",
        ),
        "https://releases.example.com"
    );
}

#[tokio::test]
async fn status_reports_newer_release() {
    let updater = test_updater("0.2.0", Arc::new(Unimock::new(())));

    let status = updater
        .status(UpdateCheck {
            binary: "juno".to_owned(),
            current_version: "0.1.0".to_owned(),
            target: "aarch64-apple-darwin".to_owned(),
        })
        .await
        .expect("status should succeed");

    assert!(status.update_available);
    assert_eq!(
        format_status_summary(&status),
        "update available: juno 0.1.0 -> 0.2.0 (aarch64-apple-darwin)"
    );
}

#[tokio::test]
async fn upgrade_skips_current_version_without_force() {
    let installer = Arc::new(Unimock::new(()));
    let updater = test_updater_with_installer("0.1.0", installer.clone());

    let outcome = updater
        .upgrade(UpdateRequest {
            check: UpdateCheck {
                binary: "juno".to_owned(),
                current_version: "0.1.0".to_owned(),
                target: "aarch64-apple-darwin".to_owned(),
            },
            current_exe: PathBuf::from("/tmp/juno"),
            force: false,
        })
        .await
        .expect("upgrade should succeed");

    assert!(matches!(outcome, UpgradeOutcome::UpToDate(_)));
}

fn test_updater(version: &str, installer: Arc<Unimock>) -> Updater {
    test_updater_with_installer(version, installer)
}

fn test_updater_with_installer(version: &str, installer: Arc<Unimock>) -> Updater {
    Updater::new(
        Arc::new(Unimock::new(
            ReleaseClientMock::latest
                .next_call(matching!("juno", "aarch64-apple-darwin"))
                .returns(Ok(ReleaseAssetInfo {
                    binary: "juno".to_owned(),
                    version: version.to_owned(),
                    target: "aarch64-apple-darwin".to_owned(),
                    asset_name: format!("juno-{version}-aarch64-apple-darwin.tar.gz"),
                    download_url: format!("/releases/download/juno/{version}/aarch64-apple-darwin"),
                    sha256: None,
                })),
        )),
        installer,
    )
}
