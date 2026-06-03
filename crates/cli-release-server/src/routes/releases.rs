use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
};

use super::{HttpResult, validate_path_component};
use crate::Error;
use crate::routes::state::ReleaseRouteState;

pub(super) async fn index(
    State(state): State<ReleaseRouteState>,
) -> HttpResult<Json<cli_release_interface::ReleaseIndex>> {
    let mut index = state.provider()?.index(&state.install.binaries).await?;
    index
        .releases
        .retain(|release| state.install.supports_binary(&release.binary));
    Ok(Json(index))
}

pub(super) async fn latest(
    State(state): State<ReleaseRouteState>,
    Path((binary, target)): Path<(String, String)>,
) -> HttpResult<Json<cli_release_interface::ReleaseAssetInfo>> {
    validate_path_component(&binary)?;
    validate_path_component(&target)?;
    validate_supported_binary(&state, &binary)?;
    Ok(Json(state.provider()?.latest(&binary, &target).await?))
}

pub(super) async fn download(
    State(state): State<ReleaseRouteState>,
    Path((binary, version, target)): Path<(String, String, String)>,
) -> HttpResult<Response> {
    validate_path_component(&binary)?;
    validate_path_component(&version)?;
    validate_path_component(&target)?;
    validate_supported_binary(&state, &binary)?;
    let download = state
        .provider()?
        .download(&binary, &version, &target)
        .await?;
    let mut response = Body::from(download.bytes).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/gzip"),
    );
    if let Ok(value) =
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", download.asset_name))
    {
        response
            .headers_mut()
            .insert(header::CONTENT_DISPOSITION, value);
    }
    Ok(response)
}

fn validate_supported_binary(state: &ReleaseRouteState, binary: &str) -> crate::Result<()> {
    if state.install.supports_binary(binary) {
        return Ok(());
    }
    Err(Error::UnsupportedInstallBinary {
        binary: binary.to_owned(),
    })
}
