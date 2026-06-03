use super::*;

#[test]
fn parses_hyphenated_binary_release_asset_names() {
    let info = assets::legacy_asset_info_from_name(
        "juno-host-1.2.3-aarch64-apple-darwin.tar.gz",
        "1.2.3",
        None,
    )
    .expect("asset should parse");

    assert_eq!(info.binary, "juno-host");
    assert_eq!(info.target, "aarch64-apple-darwin");
    assert_eq!(
        info.download_url,
        "/releases/download/juno-host/1.2.3/aarch64-apple-darwin"
    );
}

#[test]
fn ignores_non_archive_assets() {
    assert!(
        assets::legacy_asset_info_from_name(
            "juno-1.2.3-aarch64-apple-darwin.tar.gz.sha256",
            "1.2.3",
            None,
        )
        .is_none()
    );
}

#[test]
fn rejects_tags_without_workspace_version_prefix() {
    assert!(releases::version_from_tag("1.2.3").is_err());
    assert!(releases::version_from_tag("v").is_err());
}

#[test]
fn provider_uses_configured_tag_prefix() {
    let provider = GitHubReleaseProvider::new(crate::config::GitHubConfig {
        repository: "owner/repo".to_owned(),
        api_url: "https://api.github.com".to_owned(),
        token: "token".to_owned(),
        tag_prefix: "release-".to_owned(),
        asset_template: "{binary}-{version}-{target}.tar.gz".to_owned(),
    })
    .expect("provider should build");

    assert_eq!(
        provider
            .version_from_tag("release-1.2.3")
            .expect("tag should parse"),
        "1.2.3"
    );
    assert!(provider.version_from_tag("v1.2.3").is_err());
}

#[test]
fn asset_template_parses_configured_layout() {
    let template = assets::AssetTemplate::new("dist/{target}/{binary}-{version}.tgz".to_owned())
        .expect("template should be valid");
    let info = template
        .asset_info_from_name(
            "dist/aarch64-apple-darwin/example-cli-1.2.3.tgz",
            "1.2.3",
            None,
        )
        .expect("template should parse")
        .expect("asset should match");

    assert_eq!(info.binary, "example-cli");
    assert_eq!(info.target, "aarch64-apple-darwin");
    assert_eq!(
        info.download_url,
        "/releases/download/example-cli/1.2.3/aarch64-apple-darwin"
    );
}

#[test]
fn uses_archive_digest_as_sha256_metadata() {
    let asset = GitHubAsset {
        name: "juno-1.2.3-aarch64-apple-darwin.tar.gz".to_owned(),
        url: "https://example.com/juno".to_owned(),
        digest: Some(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_owned(),
        ),
    };

    let info = assets::legacy_asset_info_from_asset(&asset, "1.2.3")
        .expect("asset info should parse")
        .expect("archive asset should produce metadata");

    assert_eq!(
        info.sha256,
        Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_owned())
    );
}

#[test]
fn rejects_invalid_archive_digest_metadata() {
    let asset = GitHubAsset {
        name: "juno-1.2.3-aarch64-apple-darwin.tar.gz".to_owned(),
        url: "https://example.com/juno".to_owned(),
        digest: Some("md5:deadbeef".to_owned()),
    };

    let error =
        assets::legacy_asset_info_from_asset(&asset, "1.2.3").expect_err("digest should fail");

    assert!(matches!(
        error,
        Error::InvalidAssetDigest { asset_name, digest }
            if asset_name == "juno-1.2.3-aarch64-apple-darwin.tar.gz"
                && digest == "md5:deadbeef"
    ));
}
