//! A local GitHub API and configured release-server fixture.

use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
};
use cli_release_server::{
    channel::ReleaseChannel,
    config::{GitHubConfig, InstallConfig, ServerConfig},
    routes,
};
use serde::{Deserialize, Serialize};

/// The binary exposed by channel fixtures.
pub(crate) const BINARY: &str = "example-cli";
/// The platform exposed by channel fixtures.
pub(crate) const TARGET: &str = "aarch64-apple-darwin";
/// A valid digest used by fixture metadata.
pub(crate) const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// A serialized GitHub release, including draft and prerelease flags.
#[derive(Clone, Serialize)]
pub(crate) struct Release {
    /// The complete GitHub tag.
    pub(crate) tag_name: String,
    /// Whether the release is unpublished.
    pub(crate) draft: bool,
    /// Whether GitHub marks the release as a prerelease.
    pub(crate) prerelease: bool,
    /// The archives attached to this release.
    pub(crate) assets: Vec<Asset>,
}

/// A serialized GitHub asset.
#[derive(Clone, Serialize)]
pub(crate) struct Asset {
    name: String,
    url: String,
    digest: String,
}

/// Builds a full release with one archive for the configured binary.
pub(crate) fn release(tag: &str, version: &str) -> Release {
    Release {
        tag_name: tag.to_owned(),
        draft: false,
        prerelease: false,
        assets: vec![Asset {
            name: format!("{BINARY}-{version}-{TARGET}.tar.gz"),
            url: String::new(),
            digest: format!("sha256:{DIGEST}"),
        }],
    }
}

/// A local HTTP server that stops when its fixture leaves scope.
pub(crate) struct TestServer {
    /// The bound HTTP address.
    pub(crate) url: String,
    task: tokio::task::JoinHandle<()>,
}

impl TestServer {
    /// Binds an isolated port and serves an application.
    pub(crate) async fn start(app: Router) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("fixture should bind");
        Self::serve(listener, app)
    }

    fn serve(listener: tokio::net::TcpListener, app: Router) -> Self {
        let url = format!("http://{}", listener.local_addr().expect("fixture address"));
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("fixture should serve");
        });
        Self { url, task }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// A GitHub API fixture with recorded release requests.
pub(crate) struct GitHubFixture {
    /// The bound GitHub API server.
    pub(crate) server: TestServer,
    requests: Arc<Mutex<Vec<String>>>,
}

struct GitHubState {
    latest: Release,
    pages: Vec<Vec<Release>>,
    exact: Vec<Release>,
    status: Option<StatusCode>,
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
}

impl GitHubFixture {
    /// Starts configured release-list, latest, tag, and asset routes.
    pub(crate) async fn start(
        latest: Release,
        pages: Vec<Vec<Release>>,
        exact: Vec<Release>,
        status: Option<StatusCode>,
    ) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("GitHub fixture should bind");
        let url = format!("http://{}", listener.local_addr().expect("GitHub address"));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let state = Arc::new(GitHubState {
            latest,
            pages,
            exact,
            status,
            url,
            requests: requests.clone(),
        });
        let app = Router::new()
            .route("/repos/owner/repo/releases/latest", get(latest_release))
            .route("/repos/owner/repo/releases", get(list_releases))
            .route("/repos/owner/repo/releases/tags/{tag}", get(exact_release))
            .route("/assets/{name}", get(archive))
            .with_state(state);
        Self {
            server: TestServer::serve(listener, app),
            requests,
        }
    }

    /// Builds a provider configuration for this GitHub API.
    pub(crate) fn config(&self, channel: ReleaseChannel) -> GitHubConfig {
        GitHubConfig {
            repository: "owner/repo".to_owned(),
            api_url: self.server.url.clone(),
            token: "private-fixture-token".to_owned(),
            tag_prefix: "cli-".to_owned(),
            asset_template: "{binary}-{version}-{target}.tar.gz".to_owned(),
            channel,
        }
    }

    /// Starts the production HTTP routes with the requested channel.
    pub(crate) async fn release_server(&self, channel: ReleaseChannel) -> TestServer {
        TestServer::start(
            routes::app(ServerConfig {
                bind: "127.0.0.1:0".to_owned(),
                github: Some(self.config(channel)),
                install: InstallConfig::new(
                    Vec::new(),
                    None,
                    vec![BINARY.to_owned()],
                    Some("https://releases.example.com".to_owned()),
                    "CLI_RELEASE_SERVER_URL".to_owned(),
                    "CLI_RELEASE_INSTALL_DIR".to_owned(),
                )
                .expect("install config should build"),
            })
            .expect("release app should build"),
        )
        .await
    }

    /// Returns release API requests in their received order.
    pub(crate) fn requests(&self) -> Vec<String> {
        self.requests
            .lock()
            .expect("requests should unlock")
            .clone()
    }
}

impl GitHubState {
    fn record(&self, uri: Uri) {
        self.requests
            .lock()
            .expect("requests should unlock")
            .push(uri.to_string());
    }

    fn with_urls(&self, mut release: Release) -> Release {
        for asset in &mut release.assets {
            asset.url = format!("{}/assets/{}", self.url, asset.name);
        }
        release
    }
}

async fn latest_release(State(state): State<Arc<GitHubState>>, uri: Uri) -> Response {
    state.record(uri);
    if let Some(status) = state.status {
        return status.into_response();
    }
    Json(state.with_urls(state.latest.clone())).into_response()
}

#[derive(Deserialize)]
struct Pagination {
    page: usize,
    per_page: usize,
}

async fn list_releases(
    State(state): State<Arc<GitHubState>>,
    Query(pagination): Query<Pagination>,
    uri: Uri,
) -> Response {
    state.record(uri);
    if let Some(status) = state.status {
        return status.into_response();
    }
    assert_eq!(pagination.per_page, 100);
    let releases = state
        .pages
        .get(pagination.page - 1)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|release| state.with_urls(release))
        .collect::<Vec<_>>();
    Json(releases).into_response()
}

async fn exact_release(
    State(state): State<Arc<GitHubState>>,
    Path(tag): Path<String>,
    uri: Uri,
) -> Response {
    state.record(uri);
    match state.exact.iter().find(|release| release.tag_name == tag) {
        Some(release) => Json(state.with_urls(release.clone())).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn archive(Path(name): Path<String>) -> String {
    format!("archive:{name}")
}
