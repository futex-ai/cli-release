use axum::http::header;
use serde::Deserialize;

use super::{GitHubAsset, GitHubReleaseProvider};
use crate::{Error, Result};

pub(super) async fn get_json<T: for<'de> Deserialize<'de>>(
    provider: &GitHubReleaseProvider,
    url: &str,
) -> Result<T> {
    let response = provider
        .client
        .get(url)
        .header(header::ACCEPT, "application/vnd.github+json")
        .header(header::USER_AGENT, "cli-release-server")
        .bearer_auth(&provider.token)
        .send()
        .await
        .map_err(|source| Error::GitHubRequest {
            url: url.to_owned(),
            source,
        })?;
    let status = response.status();
    if !status.is_success() {
        return Err(Error::GitHubStatus {
            url: url.to_owned(),
            status: status.as_u16(),
        });
    }
    response
        .json::<T>()
        .await
        .map_err(|source| Error::GitHubDecode {
            url: url.to_owned(),
            source,
        })
}

pub(super) async fn download_asset(
    provider: &GitHubReleaseProvider,
    asset: &GitHubAsset,
) -> Result<Vec<u8>> {
    let response = provider
        .client
        .get(&asset.url)
        .header(header::ACCEPT, "application/octet-stream")
        .header(header::USER_AGENT, "cli-release-server")
        .bearer_auth(&provider.token)
        .send()
        .await
        .map_err(|source| Error::GitHubRequest {
            url: asset.url.clone(),
            source,
        })?;
    let status = response.status();
    if !status.is_success() {
        return Err(Error::GitHubStatus {
            url: asset.url.clone(),
            status: status.as_u16(),
        });
    }
    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|source| Error::GitHubDecode {
            url: asset.url.clone(),
            source,
        })
}
