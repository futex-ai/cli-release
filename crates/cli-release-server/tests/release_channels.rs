//! Release channel behavior through public provider APIs and HTTP routes.

mod support;

use cli_release_interface::{ReleaseAssetInfo, ReleaseIndex};
use cli_release_server::{
    Error, ReleaseProvider, channel::ReleaseChannel, github::GitHubReleaseProvider,
};
use reqwest::StatusCode;

use support::github_fixture::{BINARY, DIGEST, GitHubFixture, TARGET, release};

#[tokio::test]
async fn prerelease_routes_follow_release_order_and_keep_exact_downloads() {
    let mut draft = release("cli-10.0.0", "10.0.0");
    draft.draft = true;
    let mut preview = release("cli-1.3.0-rc.1", "1.3.0-rc.1");
    preview.prerelease = true;
    let fixture = GitHubFixture::start(
        release("cli-1.2.3", "1.2.3"),
        vec![vec![
            draft,
            release("v9.0.0", "9.0.0"),
            release("cli-", "ignored"),
            preview.clone(),
            release("cli-9.0.0", "9.0.0"),
        ]],
        vec![preview],
        None,
    )
    .await;
    let client = reqwest::Client::new();
    for (channel, expected_version) in [
        (ReleaseChannel::default(), "1.2.3"),
        (ReleaseChannel::Prerelease, "1.3.0-rc.1"),
    ] {
        let server = fixture.release_server(channel).await;
        let info = client
            .get(format!("{}/releases/latest/{BINARY}/{TARGET}", server.url))
            .send()
            .await
            .expect("latest request should complete")
            .json::<ReleaseAssetInfo>()
            .await
            .expect("latest response should decode");
        assert_eq!(info.version, expected_version, "{channel:?}");
        assert_eq!(info.sha256.as_deref(), Some(DIGEST));
        let index = client
            .get(format!("{}/releases/latest", server.url))
            .send()
            .await
            .expect("index request should complete")
            .json::<ReleaseIndex>()
            .await
            .expect("index response should decode");
        assert_eq!(index.releases, vec![info], "{channel:?}");
        let download = client
            .get(format!(
                "{}/releases/download/{BINARY}/1.3.0-rc.1/{TARGET}",
                server.url
            ))
            .send()
            .await
            .expect("exact download should complete");
        assert_eq!(download.status(), StatusCode::OK);
        assert_eq!(download.headers()["content-type"], "application/gzip");
        assert_eq!(
            download.text().await.expect("archive should load"),
            "archive:example-cli-1.3.0-rc.1-aarch64-apple-darwin.tar.gz"
        );
    }
    assert_eq!(
        fixture.requests(),
        [
            "/repos/owner/repo/releases/latest",
            "/repos/owner/repo/releases/latest",
            "/repos/owner/repo/releases/tags/cli-1.3.0-rc.1",
            "/repos/owner/repo/releases?per_page=100&page=1",
            "/repos/owner/repo/releases?per_page=100&page=1",
            "/repos/owner/repo/releases/tags/cli-1.3.0-rc.1",
        ]
    );

    let mut without_assets = release("cli-1.4.0", "1.4.0");
    without_assets.assets.clear();
    let fixture = GitHubFixture::start(
        release("cli-1.2.3", "1.2.3"),
        vec![vec![without_assets, release("cli-9.0.0", "9.0.0")]],
        Vec::new(),
        None,
    )
    .await;
    let server = fixture.release_server(ReleaseChannel::Prerelease).await;
    let response = client
        .get(format!("{}/releases/latest/{BINARY}/{TARGET}", server.url))
        .send()
        .await
        .expect("missing selected asset response should load");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let index = client
        .get(format!("{}/releases/latest", server.url))
        .send()
        .await
        .expect("empty selected index should load")
        .json::<ReleaseIndex>()
        .await
        .expect("empty index should decode");
    assert_eq!(index.releases, Vec::new());
}

#[tokio::test]
async fn prerelease_reads_later_pages_and_accepts_full_non_semver_releases() {
    let fixture = GitHubFixture::start(
        release("cli-1.2.3", "1.2.3"),
        vec![
            vec![release("other-10.0.0", "10.0.0"); 100],
            vec![
                release("cli-nightly-build", "nightly-build"),
                release("cli-99.0.0", "99.0.0"),
            ],
        ],
        Vec::new(),
        None,
    )
    .await;
    let provider = GitHubReleaseProvider::new(fixture.config(ReleaseChannel::Prerelease))
        .expect("provider should build");
    let latest = provider
        .latest(BINARY, TARGET)
        .await
        .expect("second-page full release should qualify");

    assert_eq!(latest.version, "nightly-build");
    assert_eq!(
        fixture.requests(),
        [
            "/repos/owner/repo/releases?per_page=100&page=1",
            "/repos/owner/repo/releases?per_page=100&page=2",
        ]
    );
}

#[tokio::test]
async fn prerelease_empty_lists_return_404_and_upstream_failures_keep_502() {
    let client = reqwest::Client::new();
    let mut draft = release("cli-2.0.0-rc.1", "2.0.0-rc.1");
    draft.draft = true;
    for pages in [
        Vec::new(),
        vec![vec![
            draft,
            release("v2.0.0", "2.0.0"),
            release("cli-", "ignored"),
        ]],
        vec![vec![release("other-2.0.0", "2.0.0"); 100]],
    ] {
        let fixture =
            GitHubFixture::start(release("cli-1.2.3", "1.2.3"), pages, Vec::new(), None).await;
        let provider = GitHubReleaseProvider::new(fixture.config(ReleaseChannel::Prerelease))
            .expect("provider should build");
        assert!(matches!(
            provider.latest(BINARY, TARGET).await,
            Err(Error::NoMatchingRelease)
        ));
        assert!(matches!(
            provider.index(&[BINARY.to_owned()]).await,
            Err(Error::NoMatchingRelease)
        ));
        let server = fixture.release_server(ReleaseChannel::Prerelease).await;
        for path in [
            "/releases/latest".to_owned(),
            format!("/releases/latest/{BINARY}/{TARGET}"),
        ] {
            let response = client
                .get(format!("{}{path}", server.url))
                .send()
                .await
                .expect("empty response should load");
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        }
    }
    for channel in [ReleaseChannel::Stable, ReleaseChannel::Prerelease] {
        for status in [StatusCode::NOT_FOUND, StatusCode::TOO_MANY_REQUESTS] {
            let fixture = GitHubFixture::start(
                release("cli-1.2.3", "1.2.3"),
                Vec::new(),
                Vec::new(),
                Some(status),
            )
            .await;
            let server = fixture.release_server(channel).await;
            let response = client
                .get(format!("{}/releases/latest", server.url))
                .send()
                .await
                .expect("upstream failure response should load");
            assert_eq!(response.status(), StatusCode::BAD_GATEWAY, "{channel:?}");
        }
    }
    for tag in ["v1.2.3", "cli-"] {
        let fixture =
            GitHubFixture::start(release(tag, "1.2.3"), Vec::new(), Vec::new(), None).await;
        let server = fixture.release_server(ReleaseChannel::Stable).await;
        let response = client
            .get(format!("{}/releases/latest", server.url))
            .send()
            .await
            .expect("invalid stable tag response should load");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
