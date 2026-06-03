# App CLI Release Workflow

App-specific repositories own actual CLI binary releases. This workspace owns
shared tooling and the generic release-server image, but app repositories run
their own release-plz workflow and upload their own CLI archives.

## App Repository Responsibilities

- Run release-plz for the app's CLI crates and version policy.
- Create app release tags such as `v1.2.3`.
- Build each CLI binary for each supported target.
- Package archives with `scripts/cli-release.sh package` or an equivalent
  vendored script.
- Upload `<binary>-<version>-<target>.tar.gz` and `.sha256` files to the app
  repository's GitHub Release.
- Deploy `cli-release-server` with
  `CLI_RELEASE_GITHUB_REPOSITORY=owner/app-repo`.

## Binary Matrix Example

```json
{
  "binaries": [
    { "name": "example-cli", "package": "example-cli-bin" },
    { "name": "example-admin", "package": "example-admin-bin" }
  ]
}
```

Resolve a package from that config:

```bash
scripts/cli-release.sh resolve-binary-package example-cli release-binaries.json
```

Package a built binary:

```bash
scripts/cli-release.sh package \
  example-cli \
  1.2.3 \
  aarch64-apple-darwin \
  target/aarch64-apple-darwin/release/example-cli \
  dist
```

## Reusable Workflow Shape

An app repository workflow should:

1. Run the app workspace checks before release-plz can tag or open release PRs.
2. Run release-plz `release` and `release-pr` for the app workspace.
3. On a created release, resolve the release tag with
   `scripts/cli-release.sh resolve-workspace-tag`.
4. Build the app's binary/package matrix for each supported target.
5. Smoke-test each built binary with `--help`.
6. Package each binary with `scripts/cli-release.sh package`.
7. Upload archives and checksums to the app repository's GitHub Release.

This workspace's `.github/workflows/release-plz.yml` intentionally does not
build or upload app CLI assets. It only versions shared crates in this
workspace. Pull requests run the shared check suite, while release-plz publishing
jobs only run on `main` pushes or manual dispatch after the check suite passes.

## Updater Integration

App CLI crates should depend on `cli-updater` and pass their own default
release-server URL:

```rust
let server = cli_updater::resolve_release_server_url(
    release_server_flag,
    cli_updater::DEFAULT_RELEASE_SERVER_URL_ENV,
    "https://releases.example.com",
);
```

The app crate owns the command-line flag, default URL, and user-facing upgrade
command text.
