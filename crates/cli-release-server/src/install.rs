//! Shell install script generation for released CLI binaries.

use axum::http::HeaderValue;

use crate::config::InstallConfig;
use crate::{Error, Result};

/// Returns whether the Host header is one configured installer host.
pub fn is_install_host(config: &InstallConfig, host: Option<&HeaderValue>) -> bool {
    let Some(host) = host.and_then(|value| value.to_str().ok()) else {
        return false;
    };
    host.split(':').next().is_some_and(|name| {
        config
            .install_hosts
            .iter()
            .any(|host| host.eq_ignore_ascii_case(name))
    })
}

/// Returns an install script for a supported binary.
pub fn script_for(config: &InstallConfig, binary: &str) -> Result<String> {
    if !config.supports_binary(binary) {
        return Err(Error::UnsupportedInstallBinary {
            binary: binary.to_owned(),
        });
    }
    let release_server_url = config.public_release_server_url()?;
    let client_server_url_env = &config.client_server_url_env;
    let install_dir_env = &config.install_dir_env;
    let binary_literal = shell_single_quote(binary);
    let release_server_url_literal = shell_single_quote(release_server_url);

    Ok(format!(
        r#"#!/bin/sh
set -eu

binary={binary_literal}

if [ -n "${{{client_server_url_env}:-}}" ]; then
  release_server="${{{client_server_url_env}}}"
else
  release_server={release_server_url_literal}
fi

need() {{
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: $1 is required" >&2
    exit 1
  fi
}}

need curl
need sed
need tar
need uname

os="$(uname -s)"
arch="$(uname -m)"

case "$os:$arch" in
  Linux:x86_64|Linux:amd64)
    target="x86_64-unknown-linux-gnu"
    ;;
  Linux:aarch64|Linux:arm64)
    target="aarch64-unknown-linux-gnu"
    ;;
  Darwin:x86_64|Darwin:amd64)
    target="x86_64-apple-darwin"
    ;;
  Darwin:aarch64|Darwin:arm64)
    target="aarch64-apple-darwin"
    ;;
  *)
    echo "error: unsupported platform $os/$arch" >&2
    exit 1
    ;;
esac

metadata_url="${{release_server%/}}/releases/latest/$binary/$target"
metadata="$(curl -fsSL "$metadata_url")"

field() {{
  printf '%s' "$metadata" | sed -n "s/.*\"$1\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\".*/\1/p"
}}

version="$(field version)"
asset_name="$(field asset_name)"
download_url="$(field download_url)"
sha256="$(field sha256)"

if [ -z "$version" ] || [ -z "$asset_name" ] || [ -z "$download_url" ]; then
  echo "error: release metadata from $metadata_url was incomplete" >&2
  exit 1
fi

if [ -z "$sha256" ]; then
  echo "error: release metadata from $metadata_url did not include a checksum" >&2
  exit 1
fi

case "$download_url" in
  http://*|https://*)
    archive_url="$download_url"
    ;;
  /*)
    archive_url="${{release_server%/}}$download_url"
    ;;
  *)
    archive_url="${{release_server%/}}/$download_url"
    ;;
esac

tmp_dir="$(mktemp -d 2>/dev/null || mktemp -d -t cli-release-install)"
trap 'rm -rf "$tmp_dir"' EXIT HUP INT TERM

archive_name="${{asset_name##*/}}"

if [ -z "$archive_name" ]; then
  echo "error: release metadata from $metadata_url had an invalid asset name" >&2
  exit 1
fi

archive="$tmp_dir/$archive_name"
curl -fsSL "$archive_url" -o "$archive"

if command -v sha256sum >/dev/null 2>&1; then
  printf '%s  %s\n' "$sha256" "$archive" | sha256sum -c - >/dev/null
elif command -v shasum >/dev/null 2>&1; then
  printf '%s  %s\n' "$sha256" "$archive" | shasum -a 256 -c - >/dev/null
else
  echo "error: sha256sum or shasum is required" >&2
  exit 1
fi

is_unsafe_archive_member() {{
  case "$1" in
    ""|/*|-*|..|../*|*/../*|*/..)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}}

archive_members="$(tar -tzf "$archive")" || {{
  echo "error: archive could not be listed" >&2
  exit 1
}}
binary_member=""

while IFS= read -r member; do
  if is_unsafe_archive_member "$member"; then
    echo "error: archive contained unsafe path $member" >&2
    exit 1
  fi
  member_name="${{member##*/}}"
  if [ "$member_name" = "$binary" ]; then
    if [ -n "$binary_member" ]; then
      echo "error: archive contained multiple $binary files" >&2
      exit 1
    fi
    binary_member="$member"
  fi
done <<EOF
$archive_members
EOF

if [ -z "$binary_member" ]; then
  echo "error: archive did not contain $binary" >&2
  exit 1
fi

binary_path="$tmp_dir/$binary"
if ! tar -xOf "$archive" -- "$binary_member" >"$binary_path"; then
  echo "error: failed to extract $binary from archive" >&2
  exit 1
fi

if [ -n "${{{install_dir_env}:-}}" ]; then
  install_dir="${{{install_dir_env}}}"
elif [ -d /usr/local/bin ] && [ -w /usr/local/bin ]; then
  install_dir="/usr/local/bin"
else
  if [ -z "${{HOME:-}}" ]; then
    echo "error: HOME is required when {install_dir_env} is not set" >&2
    exit 1
  fi
  install_dir="$HOME/.local/bin"
fi

mkdir -p "$install_dir"

if [ ! -w "$install_dir" ]; then
  echo "error: $install_dir is not writable; set {install_dir_env} to a writable directory" >&2
  exit 1
fi

cp "$binary_path" "$install_dir/$binary"
chmod 755 "$install_dir/$binary"

echo "installed $binary $version to $install_dir/$binary"

case ":${{PATH:-}}:" in
  *":$install_dir:"*)
    ;;
  *)
    echo "warning: $install_dir is not on PATH" >&2
    ;;
esac
"#
    ))
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

#[cfg(test)]
#[path = "_tests_/install_tests.rs"]
mod install_tests;
