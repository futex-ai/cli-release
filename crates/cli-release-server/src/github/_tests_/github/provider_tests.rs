use super::*;

use std::{net::TcpListener, thread, time::Duration};

use axum::{
    Router,
    http::header,
    response::{IntoResponse, Response},
    routing::get,
};

#[tokio::test]
async fn download_fetches_requested_release_version() {
    let (api_url, server) = spawn_github_fixture().await;
    let provider = GitHubReleaseProvider::new(crate::config::GitHubConfig {
        repository: "owner/repo".to_owned(),
        api_url,
        token: "token".to_owned(),
        tag_prefix: "v".to_owned(),
        asset_template: "{binary}-{version}-{target}.tar.gz".to_owned(),
    })
    .expect("provider should build");

    let download = provider
        .download("example-cli", "1.2.3", "aarch64-apple-darwin")
        .await
        .expect("versioned download should use requested tag");

    assert_eq!(
        download.asset_name,
        "example-cli-1.2.3-aarch64-apple-darwin.tar.gz"
    );
    assert_eq!(download.bytes, b"example-cli-1.2.3");

    server.abort();
}

#[tokio::test]
async fn index_skips_unconfigured_assets_before_checksum_resolution() {
    let (api_url, server) = spawn_github_fixture().await;
    let provider = GitHubReleaseProvider::new(crate::config::GitHubConfig {
        repository: "owner/repo".to_owned(),
        api_url,
        token: "token".to_owned(),
        tag_prefix: "v".to_owned(),
        asset_template: "{binary}-{version}-{target}.tar.gz".to_owned(),
    })
    .expect("provider should build");

    let index = provider
        .index(&["example-cli".to_owned()])
        .await
        .expect("index should load");
    let release = index
        .releases
        .iter()
        .find(|release| release.binary == "example-cli")
        .expect("example-cli should be indexed");

    assert_eq!(index.releases.len(), 1);
    assert_eq!(
        release.sha256,
        Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_owned())
    );

    server.abort();
}

#[tokio::test]
async fn custom_timeout_bounds_stalled_github_request() {
    let api_url = hanging_http_server();
    let provider = GitHubReleaseProvider::new_with_timeouts(
        crate::config::GitHubConfig {
            repository: "owner/repo".to_owned(),
            api_url,
            token: "token".to_owned(),
            tag_prefix: "v".to_owned(),
            asset_template: "{binary}-{version}-{target}.tar.gz".to_owned(),
        },
        Duration::from_millis(50),
        Duration::from_millis(50),
    )
    .expect("provider should build");

    let error = provider
        .latest("example-cli", "aarch64-apple-darwin")
        .await
        .expect_err("stalled GitHub request should time out");

    assert!(matches!(error, Error::GitHubRequest { .. }));
}

async fn spawn_github_fixture() -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("fixture should bind");
    let api_url = format!(
        "http://{}",
        listener.local_addr().expect("fixture address should exist")
    );
    let latest_url = api_url.clone();
    let tagged_url = api_url.clone();
    let app = Router::new()
        .route(
            "/repos/owner/repo/releases/latest",
            get(move || latest_release_response(latest_url.clone())),
        )
        .route(
            "/repos/owner/repo/releases/tags/v1.2.3",
            get(move || tagged_release_response(tagged_url.clone())),
        )
        .route(
            "/downloads/example-cli-1.2.3-aarch64-apple-darwin.tar.gz",
            get(example_cli_123_archive),
        )
        .route(
            "/downloads/example-cli-2.0.0-aarch64-apple-darwin.tar.gz.sha256",
            get(example_cli_200_checksum),
        );
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("fixture server should run");
    });
    (api_url, server)
}

fn hanging_http_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
    let address = listener.local_addr().expect("listener should have address");
    thread::spawn(move || {
        if let Ok((_stream, _address)) = listener.accept() {
            thread::sleep(Duration::from_secs(2));
        }
    });
    format!("http://{address}")
}

async fn latest_release_response(api_url: String) -> Response {
    json_response(format!(
        r#"{{
  "tag_name": "v2.0.0",
  "assets": [
    {{
      "name": "example-cli-2.0.0-aarch64-apple-darwin.tar.gz",
      "url": "{api_url}/downloads/example-cli-2.0.0-aarch64-apple-darwin.tar.gz",
      "digest": null
    }},
    {{
      "name": "example-cli-2.0.0-aarch64-apple-darwin.tar.gz.sha256",
      "url": "{api_url}/downloads/example-cli-2.0.0-aarch64-apple-darwin.tar.gz.sha256",
      "digest": null
    }},
    {{
      "name": "hidden-cli-2.0.0-aarch64-apple-darwin.tar.gz",
      "url": "{api_url}/downloads/hidden-cli-2.0.0-aarch64-apple-darwin.tar.gz",
      "digest": "md5:deadbeef"
    }}
  ]
}}"#
    ))
}

async fn tagged_release_response(api_url: String) -> Response {
    json_response(format!(
        r#"{{
  "tag_name": "v1.2.3",
  "assets": [
    {{
      "name": "example-cli-1.2.3-aarch64-apple-darwin.tar.gz",
      "url": "{api_url}/downloads/example-cli-1.2.3-aarch64-apple-darwin.tar.gz",
      "digest": null
    }}
  ]
}}"#
    ))
}

async fn example_cli_123_archive() -> &'static [u8] {
    b"example-cli-1.2.3"
}

async fn example_cli_200_checksum() -> &'static str {
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  example-cli-2.0.0-aarch64-apple-darwin.tar.gz\n"
}

fn json_response(body: String) -> Response {
    ([(header::CONTENT_TYPE, "application/json")], body).into_response()
}
