use super::*;

#[test]
fn serializes_release_asset_info_contract() {
    let info = ReleaseAssetInfo {
        binary: "juno".to_owned(),
        version: "0.2.0".to_owned(),
        target: "aarch64-apple-darwin".to_owned(),
        asset_name: "juno-0.2.0-aarch64-apple-darwin.tar.gz".to_owned(),
        download_url: "/releases/download/juno/0.2.0/aarch64-apple-darwin".to_owned(),
        sha256: Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_owned()),
    };

    let value = serde_json::to_value(&info).expect("release asset should serialize");

    assert_eq!(value["binary"], "juno");
    assert_eq!(value["version"], "0.2.0");
    assert_eq!(value["target"], "aarch64-apple-darwin");
    assert_eq!(
        value["asset_name"],
        "juno-0.2.0-aarch64-apple-darwin.tar.gz"
    );
    assert_eq!(
        value["download_url"],
        "/releases/download/juno/0.2.0/aarch64-apple-darwin"
    );
    assert_eq!(
        value["sha256"],
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
}

#[test]
fn deserializes_release_index_contract() {
    let json = serde_json::json!({
        "releases": [{
            "binary": "juno-host",
            "version": "0.2.0",
            "target": "x86_64-unknown-linux-gnu",
            "asset_name": "juno-host-0.2.0-x86_64-unknown-linux-gnu.tar.gz",
            "download_url": "/releases/download/juno-host/0.2.0/x86_64-unknown-linux-gnu",
            "sha256": null
        }]
    });

    let index: ReleaseIndex =
        serde_json::from_value(json).expect("release index should deserialize");

    assert_eq!(index.releases.len(), 1);
    assert_eq!(index.releases[0].binary, "juno-host");
    assert_eq!(index.releases[0].sha256, None);
}
