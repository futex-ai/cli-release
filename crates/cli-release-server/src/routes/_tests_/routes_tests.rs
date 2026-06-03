use super::root::build_commit_from;
use super::*;
use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::to_bytes,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, header},
};
use cli_release_interface::{ReleaseAssetInfo, ReleaseIndex};

use crate::config::InstallConfig;
use crate::{ReleaseDownload, ReleaseProvider};

async fn response_text(response: Response) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    String::from_utf8(bytes.to_vec()).expect("body should be utf-8")
}

fn test_state() -> ReleaseRouteState {
    ReleaseRouteState {
        provider: None,
        install: InstallConfig::new(
            vec!["install.example.com".to_owned()],
            Some("example-cli".to_owned()),
            vec!["example-cli".to_owned(), "example-admin".to_owned()],
            Some("https://releases.example.com".to_owned()),
            "CLI_RELEASE_SERVER_URL".to_owned(),
            "CLI_RELEASE_INSTALL_DIR".to_owned(),
        )
        .expect("install config should be valid"),
    }
}

fn test_state_with_provider() -> ReleaseRouteState {
    let mut state = test_state();
    state.provider = Some(Arc::new(TestProvider));
    state
}

#[derive(Clone)]
struct TestProvider;

#[async_trait]
impl ReleaseProvider for TestProvider {
    async fn index(&self, binaries: &[String]) -> crate::Result<ReleaseIndex> {
        Ok(ReleaseIndex {
            releases: vec![
                release_asset("example-cli"),
                release_asset("example-admin"),
                release_asset("hidden-cli"),
            ]
            .into_iter()
            .filter(|release| binaries.iter().any(|binary| binary == &release.binary))
            .collect(),
        })
    }

    async fn latest(&self, binary: &str, _target: &str) -> crate::Result<ReleaseAssetInfo> {
        Ok(release_asset(binary))
    }

    async fn download(
        &self,
        binary: &str,
        version: &str,
        target: &str,
    ) -> crate::Result<ReleaseDownload> {
        Ok(ReleaseDownload {
            asset_name: format!("{binary}-{version}-{target}.tar.gz"),
            bytes: binary.as_bytes().to_vec(),
        })
    }
}

fn release_asset(binary: &str) -> ReleaseAssetInfo {
    ReleaseAssetInfo {
        binary: binary.to_owned(),
        version: "1.2.3".to_owned(),
        target: "aarch64-apple-darwin".to_owned(),
        asset_name: format!("{binary}-1.2.3-aarch64-apple-darwin.tar.gz"),
        download_url: format!("/releases/download/{binary}/1.2.3/aarch64-apple-darwin"),
        sha256: None,
    }
}

#[tokio::test]
async fn root_returns_service_info_by_default() {
    let response = root(State(test_state()), HeaderMap::new()).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE),
        Some(&HeaderValue::from_static("application/json"))
    );

    let body = response_text(response).await;
    assert!(body.contains("\"name\":\"cli-release-server\""));
}

#[tokio::test]
async fn root_returns_default_install_script_on_install_host() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::HOST,
        HeaderValue::from_static("install.example.com"),
    );
    let response = root(State(test_state()), headers).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE),
        Some(&HeaderValue::from_static(
            "text/x-shellscript; charset=utf-8"
        ))
    );

    let body = response_text(response).await;
    assert!(body.contains("binary='example-cli'"));
    assert!(body.contains("https://releases.example.com"));
}

#[tokio::test]
async fn slash_binary_returns_binary_install_script() {
    let response = install_script(State(test_state()), Path("example-admin".to_owned()))
        .await
        .expect("install script should render");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_text(response).await;
    assert!(body.contains("binary='example-admin'"));
}

#[tokio::test]
async fn unsupported_binary_returns_not_found() {
    let Err(error) = install_script(State(test_state()), Path("example-server".to_owned())).await
    else {
        panic!("unsupported binary should fail");
    };

    assert_eq!(error.into_response().status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn release_index_filters_to_configured_binaries() {
    let Json(index) = index(State(test_state_with_provider()))
        .await
        .expect("index should render");

    let binaries = index
        .releases
        .into_iter()
        .map(|release| release.binary)
        .collect::<Vec<_>>();

    assert_eq!(binaries, vec!["example-cli", "example-admin"]);
}

#[tokio::test]
async fn latest_rejects_unconfigured_binary() {
    let Err(error) = latest(
        State(test_state_with_provider()),
        Path(("hidden-cli".to_owned(), "aarch64-apple-darwin".to_owned())),
    )
    .await
    else {
        panic!("unconfigured binary should fail");
    };

    assert_eq!(error.into_response().status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn download_rejects_unconfigured_binary() {
    let Err(error) = download(
        State(test_state_with_provider()),
        Path((
            "hidden-cli".to_owned(),
            "1.2.3".to_owned(),
            "aarch64-apple-darwin".to_owned(),
        )),
    )
    .await
    else {
        panic!("unconfigured binary should fail");
    };

    assert_eq!(error.into_response().status(), StatusCode::NOT_FOUND);
}

#[test]
fn accepts_release_path_components() {
    validate_path_component("juno-host").expect("binary should be valid");
    validate_path_component("1.2.3").expect("version should be valid");
    validate_path_component("aarch64-apple-darwin").expect("target should be valid");
}

#[test]
fn rejects_unsafe_release_path_components() {
    assert!(validate_path_component("").is_err());
    assert!(validate_path_component("../juno").is_err());
    assert!(validate_path_component("juno/server").is_err());
}

#[test]
fn build_commit_prefers_runtime_value() {
    let commit = build_commit_from(Some("runtime".to_owned()), Some("compiled"), Some("github"));

    assert_eq!(commit, "runtime");
}
