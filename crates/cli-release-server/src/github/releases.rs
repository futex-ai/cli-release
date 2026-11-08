//! GitHub release lookup and channel selection.

use serde::Deserialize;

use crate::channel::ReleaseChannel;
use crate::{Error, Result};

use super::{GitHubReleaseProvider, client::get_json};

const RELEASES_PER_PAGE: usize = 100;

#[derive(Clone, Debug, Deserialize)]
pub(super) struct GitHubRelease {
    pub(super) tag_name: String,
    #[serde(default)]
    pub(super) draft: bool,
    pub(super) assets: Vec<GitHubAsset>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct GitHubAsset {
    pub(super) name: String,
    pub(super) url: String,
    pub(super) digest: Option<String>,
}

pub(super) async fn latest_release(provider: &GitHubReleaseProvider) -> Result<GitHubRelease> {
    if provider.channel == ReleaseChannel::Prerelease {
        return first_matching_release(provider).await;
    }
    let url = format!(
        "{}/repos/{}/releases/latest",
        provider.api_url.trim_end_matches('/'),
        provider.repository
    );
    get_json(provider, &url).await
}

async fn first_matching_release(provider: &GitHubReleaseProvider) -> Result<GitHubRelease> {
    let mut page = 1_u64;
    loop {
        let url = format!(
            "{}/repos/{}/releases?per_page={RELEASES_PER_PAGE}&page={page}",
            provider.api_url.trim_end_matches('/'),
            provider.repository
        );
        let releases: Vec<GitHubRelease> = get_json(provider, &url).await?;
        let last_page = releases.len() < RELEASES_PER_PAGE;
        if let Some(release) = releases
            .into_iter()
            .find(|release| !release.draft && provider.version_from_tag(&release.tag_name).is_ok())
        {
            return Ok(release);
        }
        if last_page {
            return Err(Error::NoMatchingRelease);
        }
        page += 1;
    }
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
