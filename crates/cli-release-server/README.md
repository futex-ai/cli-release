# cli-release-server

`cli-release-server` serves install scripts, latest release metadata, and
proxied archive downloads for CLI binaries published as GitHub Release assets.

## Responsibilities

- Serve latest-release metadata for configured CLI binaries.
- Proxy archive downloads from private GitHub Releases.
- Render install scripts without exposing GitHub credentials to clients.
- Keep product-specific repositories, domains, binaries, tag prefixes, and
  asset naming in `CLI_RELEASE_*` configuration instead of code.

## What This Crate Does

The server sits in front of private GitHub Releases so installed CLIs can
discover and download updates without shipping GitHub credentials. The shared
JSON contract for those endpoints lives in `cli-release-interface`; explicit
upgrade clients can use `cli-updater`.

## Quick Start

```bash
CLI_RELEASE_GITHUB_REPOSITORY=owner/repo \
CLI_RELEASE_GITHUB_TOKEN=ghp_example \
CLI_RELEASE_PUBLIC_RELEASE_SERVER_URL=https://releases.example.com \
CLI_RELEASE_INSTALL_HOSTS=install.example.com \
CLI_RELEASE_DEFAULT_BINARY=example-cli \
CLI_RELEASE_BINARIES=example-cli,example-admin \
cargo run -p cli-release-server -- serve --bind 127.0.0.1:8080
```

```bash
curl http://127.0.0.1:8080/
curl -H 'Host: install.example.com' http://127.0.0.1:8080/
```

The service accepts `--log-level`, `--log-format`,
`CLI_RELEASE_LOG_LEVEL`, and `CLI_RELEASE_LOG_FORMAT`. Local runs default to
`info` and `pretty`; production deploys normally set JSON lines explicitly.

## Configuration

Set provider credentials:

- `CLI_RELEASE_GITHUB_REPOSITORY`, for example `owner/repo`.
- `CLI_RELEASE_GITHUB_TOKEN`, a token that can read private GitHub Release
  assets.

Set both values for GitHub-backed deployments, or leave both unset for a server
that only renders configured install-script routes. Partial GitHub
configuration and empty GitHub binary allowlists fail during startup.

Common optional values:

- `CLI_RELEASE_GITHUB_API_URL`, defaults to `https://api.github.com`.
- `CLI_RELEASE_SERVER_BIND`, defaults to `0.0.0.0:8080`.
- `CLI_RELEASE_PUBLIC_RELEASE_SERVER_URL`, embedded in install scripts.
- `CLI_RELEASE_INSTALL_HOSTS`, comma-separated install-script hostnames.
- `CLI_RELEASE_DEFAULT_BINARY`, binary installed from `/` on install hosts.
- `CLI_RELEASE_BINARIES`, comma-separated release and install-script binary
  allowlist.
- `CLI_RELEASE_TAG_PREFIX`, defaults to `v`.
- `CLI_RELEASE_ASSET_TEMPLATE`, defaults to
  `{binary}-{version}-{target}.tar.gz`.
- `CLI_RELEASE_CLIENT_SERVER_URL_ENV`, defaults to `CLI_RELEASE_SERVER_URL`.
- `CLI_RELEASE_INSTALL_DIR_ENV`, defaults to `CLI_RELEASE_INSTALL_DIR`.

When `CLI_RELEASE_INSTALL_HOSTS` is set, `CLI_RELEASE_DEFAULT_BINARY` is
required and must be included in `CLI_RELEASE_BINARIES`. Configured binary
names use the same path-component contract as release routes: ASCII
alphanumeric characters plus `.`, `_`, and `-`.

## Runtime

The service defaults to `0.0.0.0:8080` and serves:

```text
GET /
GET /{binary}
GET /releases/latest
GET /releases/latest/{binary}/{target}
GET /releases/download/{binary}/{version}/{target}
```

When GitHub release access is configured, the server reads the latest release,
derives per-binary metadata, and proxies matching archives. Metadata includes a
SHA-256 digest when the release has a matching `.sha256` sidecar or GitHub
asset digest.
GitHub API and asset requests use explicit request and connect timeouts so
stalled upstream calls do not hang route handlers indefinitely.

Generated install scripts verify the archive checksum, reject unsafe archive
member paths, and extract only the selected binary into a temporary file before
copying it into the install directory.

Release metadata and download routes are scoped to `CLI_RELEASE_BINARIES`.
The index route skips unconfigured GitHub assets before checksum resolution, so
metadata problems on assets outside the deployment allowlist do not break
configured binaries.
Versioned download URLs fetch the GitHub release tag for that version, so a
newer latest release does not invalidate metadata that was already returned.

Container images set `CLI_RELEASE_BUILD_COMMIT` in the runtime stage so
`GET /` reports the deployed revision.

## Development

```bash
cargo test -p cli-release-server
cargo clippy -p cli-release-server --all-targets --all-features -- -D warnings
cargo xtask check
```

Keep endpoint behavior aligned with the CLI release protocol docs.

### Key Code

- `src/main.rs` - CLI entrypoint and bind-address parsing.
- `src/config.rs` - `CLI_RELEASE_*` configuration parsing and validation.
- `src/lib.rs` - public release-provider traits and shared error types.
- `src/routes/` - Axum route wiring for root, install-script, and release
  endpoints.
- `src/github/` - GitHub Releases client, asset template matching, and release
  metadata assembly.
- `src/install.rs` - configured install script rendering.

### Related Docs

- [`../cli-release-interface/README.md`](../cli-release-interface/README.md)
- [`../cli-updater/README.md`](../cli-updater/README.md)
- [`../../docs/protocol/cli-releases.md`](../../docs/protocol/cli-releases.md)
