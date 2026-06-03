use serde::Deserialize;

use super::{GitHubReleaseProvider, client::get_json};
#[cfg(test)]
use crate::Error;
use crate::Result;

#[derive(Clone, Debug, Deserialize)]
pub(super) struct GitHubRelease {
    pub(super) tag_name: String,
    pub(super) assets: Vec<GitHubAsset>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct GitHubAsset {
    pub(super) name: String,
    pub(super) url: String,
    pub(super) digest: Option<String>,
}

pub(super) async fn latest_release(provider: &GitHubReleaseProvider) -> Result<GitHubRelease> {
    let url = format!(
        "{}/repos/{}/releases/latest",
        provider.api_url.trim_end_matches('/'),
        provider.repository
    );
    get_json(provider, &url).await
}

pub(super) async fn release_for_tag(
    provider: &GitHubReleaseProvider,
    tag: &str,
) -> Result<GitHubRelease> {
    let url = format!(
        "{}/repos/{}/releases/tags/{}",
        provider.api_url.trim_end_matches('/'),
        provider.repository,
        encode_path_segment(tag)
    );
    get_json(provider, &url).await
}

fn encode_path_segment(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
                return [byte as char, '\0', '\0'];
            }
            const HEX: &[u8; 16] = b"0123456789ABCDEF";
            [
                '%',
                HEX[(byte >> 4) as usize] as char,
                HEX[(byte & 0x0f) as usize] as char,
            ]
        })
        .filter(|ch| *ch != '\0')
        .collect()
}

#[cfg(test)]
pub(super) fn version_from_tag(tag: &str) -> Result<String> {
    tag.strip_prefix('v')
        .filter(|version| !version.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| Error::InvalidReleaseTag {
            tag: tag.to_owned(),
        })
}
