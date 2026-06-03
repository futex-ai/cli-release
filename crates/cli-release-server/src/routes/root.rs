use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;

use super::{HttpError, HttpResult, state::ReleaseRouteState};
use crate::{Error, Result, install};

const SERVICE_NAME: &str = "cli-release-server";
const UNKNOWN_COMMIT: &str = "unknown";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct RootServiceInfo {
    name: &'static str,
    version: &'static str,
    commit: String,
}

async fn root_service_info() -> Json<RootServiceInfo> {
    Json(RootServiceInfo {
        name: SERVICE_NAME,
        version: env!("CARGO_PKG_VERSION"),
        commit: build_commit(),
    })
}

fn build_commit() -> String {
    build_commit_from(
        std::env::var("CLI_RELEASE_BUILD_COMMIT").ok(),
        option_env!("CLI_RELEASE_BUILD_COMMIT"),
        option_env!("GITHUB_SHA"),
    )
}

pub(super) fn build_commit_from(
    runtime_commit: Option<String>,
    compiled_commit: Option<&'static str>,
    github_sha: Option<&'static str>,
) -> String {
    runtime_commit
        .filter(|value| !value.is_empty())
        .or_else(|| {
            compiled_commit
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        })
        .or_else(|| {
            github_sha
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| UNKNOWN_COMMIT.to_owned())
}

pub(super) async fn root(State(state): State<ReleaseRouteState>, headers: HeaderMap) -> Response {
    if install::is_install_host(&state.install, headers.get(header::HOST)) {
        let default_binary = match state.install.default_binary() {
            Ok(binary) => binary,
            Err(error) => return HttpError(error).into_response(),
        };
        return match install::script_for(&state.install, default_binary) {
            Ok(script) => shell_script_response(default_binary, script),
            Err(error) => HttpError(error).into_response(),
        };
    }
    root_service_info().await.into_response()
}

pub(super) async fn install_script(
    State(state): State<ReleaseRouteState>,
    Path(binary): Path<String>,
) -> HttpResult<Response> {
    validate_path_component(&binary)?;
    Ok(shell_script_response(
        &binary,
        install::script_for(&state.install, &binary)?,
    ))
}

fn shell_script_response(binary: &str, script: String) -> Response {
    let mut response = Body::from(script).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/x-shellscript; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Ok(value) =
        HeaderValue::from_str(&format!("attachment; filename=\"install-{binary}.sh\""))
    {
        response
            .headers_mut()
            .insert(header::CONTENT_DISPOSITION, value);
    }
    response
}

pub(super) fn validate_path_component(value: &str) -> Result<()> {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    {
        return Ok(());
    }
    Err(Error::InvalidPathComponent {
        value: value.to_owned(),
    })
}
