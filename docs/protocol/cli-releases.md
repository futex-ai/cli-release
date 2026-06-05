# CLI Releases

This document defines the target contract for the standalone `cli-release`
workspace. The shared server and updater code must stay free of
product-specific defaults.

## Goals

- Share one release metadata contract across CLI binaries, updater clients, and
  the release server.
- Allow one release-server deployment to serve one GitHub repository and one or
  more CLI binaries from that repository.
- Configure repository, hostnames, public release URL, binary allowlist, tag
  convention, and asset naming through environment-backed configuration.
- Keep GitHub credentials in the server deployment only. CLI users and updater
  clients must not need GitHub credentials.
- Keep deployment-specific choices in infrastructure values or environment
  variables, not in Rust code.

## Release Metadata

`cli-release-interface` owns the JSON response types.

`ReleaseAssetInfo` includes:

- `binary`: CLI executable name.
- `version`: release version without a tag prefix.
- `target`: release target triple.
- `asset_name`: archive asset name in the upstream release.
- `download_url`: absolute URL or release-server-relative archive URL. Updater
  clients accept server-relative paths with or without a leading `/`.
- `sha256`: optional archive SHA-256 digest.

`ReleaseIndex` wraps:

- `releases`: array of `ReleaseAssetInfo` records.

The server returns metadata as JSON. Archive downloads remain binary
`application/gzip` responses.

## Server Endpoints

The release server exposes:

```text
GET /
GET /{binary}
GET /releases/latest
GET /releases/latest/{binary}/{target}
GET /releases/download/{binary}/{version}/{target}
```

`GET /` returns service metadata unless the request host matches a configured
install host. On an install host, `GET /` returns the installer for the
configured default binary.

`GET /{binary}` returns an installer only for configured binaries. Unknown
binaries return `404`. Release metadata and download routes are also scoped to
`CLI_RELEASE_BINARIES`; `GET /releases/latest` filters out assets for
unconfigured binaries, and binary-specific release routes return `404` for
unconfigured binaries.

Release path components must be non-empty and limited to ASCII alphanumeric
characters plus `.`, `_`, and `-`.

## Server Configuration

The shared server must not hardcode product names, GitHub repositories, binary
names, install hosts, release hosts, or public release URLs.

Primary environment variables:

- `CLI_RELEASE_GITHUB_REPOSITORY`: GitHub repository, for example
  `owner/repo`.
- `CLI_RELEASE_GITHUB_TOKEN`: token that can read private GitHub Release
  assets.
- `CLI_RELEASE_GITHUB_API_URL`: optional GitHub API base URL. Defaults to
  `https://api.github.com`.
- `CLI_RELEASE_SERVER_BIND`: bind address. Defaults to `0.0.0.0:8080`.
- `CLI_RELEASE_PUBLIC_RELEASE_SERVER_URL`: URL embedded in install scripts.
- `CLI_RELEASE_INSTALL_HOSTS`: comma-separated hostnames that should serve
  install scripts from `/`.
- `CLI_RELEASE_DEFAULT_BINARY`: binary installed from `/` on install hosts.
  Required when `CLI_RELEASE_INSTALL_HOSTS` is configured.
- `CLI_RELEASE_BINARIES`: comma-separated binary allowlist. Required for
  GitHub-backed deployments.
- `CLI_RELEASE_TAG_PREFIX`: release tag prefix. Defaults to `v`.
- `CLI_RELEASE_ASSET_TEMPLATE`: archive naming template. Defaults to
  `{binary}-{version}-{target}.tar.gz`.
- `CLI_RELEASE_CLIENT_SERVER_URL_ENV`: environment variable name emitted in
  install scripts for client-side release-server overrides. Defaults to
  `CLI_RELEASE_SERVER_URL`.
- `CLI_RELEASE_LOG_LEVEL`: log level for the server process.
- `CLI_RELEASE_LOG_FORMAT`: log format for the server process.

The shared implementation must use the `CLI_RELEASE_*` variables above. It must
not add product-specific compatibility aliases such as `JUNO_*`.
GitHub-backed deployments must set both `CLI_RELEASE_GITHUB_REPOSITORY` and
`CLI_RELEASE_GITHUB_TOKEN`, or neither. Partial GitHub configuration is invalid
so a misconfigured release server fails before serving routes. GitHub-backed
deployments must also set a non-empty `CLI_RELEASE_BINARIES` allowlist.
Install-host deployments must set `CLI_RELEASE_DEFAULT_BINARY` to a binary
listed in `CLI_RELEASE_BINARIES`; otherwise startup fails before serving
installer traffic. Configured binary names must be valid release path
components: non-empty ASCII alphanumeric strings plus `.`, `_`, and `-`.

## GitHub Release Contract

The GitHub provider reads the latest release from the configured repository.
The release tag must start with `CLI_RELEASE_TAG_PREFIX`; the remaining suffix
is the release version.

Archive assets are matched with `CLI_RELEASE_ASSET_TEMPLATE`. The default
template supports names such as:

```text
example-cli-1.2.3-aarch64-apple-darwin.tar.gz
```

Checksum metadata is resolved from the matching `.sha256` companion asset when
present. If no companion exists, the server may use GitHub's asset digest when
it is a valid `sha256:<hex>` value.
For release indexes, the provider skips assets whose binary is not in
`CLI_RELEASE_BINARIES` before checksum resolution. Invalid checksum metadata for
unconfigured binaries must not fail the index route.

Versioned download routes must fetch the GitHub release identified by
`CLI_RELEASE_TAG_PREFIX` plus the requested version, not the repository's
current latest release.
GitHub API and asset HTTP requests must use explicit request and connect
timeouts so stalled upstream calls cannot hang server route handlers
indefinitely.

## Installer Contract

Install scripts:

- Support Linux and macOS on `x86_64` and `aarch64`.
- Resolve the release server from the configured client override env var, then
  from `CLI_RELEASE_PUBLIC_RELEASE_SERVER_URL`.
- Fetch `/releases/latest/{binary}/{target}`.
- Download the archive from `download_url`.
- Require a SHA-256 checksum in metadata before installing.
- Reject archive members with absolute paths, parent-directory traversal, or
  option-like leading dashes before extraction.
- Extract only the selected binary member to a fixed temporary path instead of
  unpacking the full archive.
- Install into `CLI_RELEASE_INSTALL_DIR` when set, `/usr/local/bin` when
  writable, or `$HOME/.local/bin` otherwise.

Install scripts must not embed product-specific domains or binary names except
for values supplied by server configuration.

## Updater Contract

`cli-updater` provides trait-backed release checks and installation:

- `ReleaseClient` fetches metadata and archive bytes.
- `BinaryInstaller` verifies optional checksums and replaces the current
  executable.
- `Updater::status` compares semantic versions.
- `Updater::upgrade` installs only when a newer version exists unless forced.
- The production HTTP client uses explicit request and connect timeouts, with a
  constructor for callers that need custom timeout values.

The shared updater must not hardcode a product release-server URL. Consuming CLI
crates pass their preferred default URL and override env var name at their
composition boundary.

## Release Automation Ownership

App-specific repositories own CLI binary releases. They run release-plz, create
version tags, build their CLI binaries, upload archive assets to their GitHub
Release, and deploy the shared release server with `CLI_RELEASE_*` values that
point back to that app repository.

This repository owns reusable release tooling: packaging scripts, validation
tests, documentation, optional reusable workflow templates, the generic
release-server image, and deployment chart examples. It also runs release-plz
for versioning and releasing this workspace's shared crates, including
`cli-release-interface`, `cli-updater`, and `cli-release-server`.
On pushes to `main`, the shared release workflow publishes the generic
`cli-release-server` runtime image after workspace checks pass.

The in-repo release-plz workflow must only release shared workspace artifacts.
It must not copy an app repository's `.github/workflows/release.yml` as an
active workflow that publishes app CLI assets from this repository.

## Validation

Implementation changes must include focused tests for:

- server configuration parsing and validation;
- install-host routing and installer rendering;
- binary allowlist behavior;
- release tag prefix handling;
- asset template matching;
- checksum sidecar and GitHub digest handling;
- updater URL resolution and upgrade behavior.

Before completion, run relevant package tests, `cargo fmt --all -- --check`,
clippy, `cargo xtask check`, then `cargo xtask review`.
