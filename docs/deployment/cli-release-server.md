# CLI Release Server Deployment

`cli-release-server` is a shared service image that each app-specific CLI
repository can deploy with its own release configuration.

## Required Configuration

```bash
CLI_RELEASE_GITHUB_REPOSITORY=owner/app-repo
CLI_RELEASE_GITHUB_TOKEN=<token that can read private release assets>
CLI_RELEASE_PUBLIC_RELEASE_SERVER_URL=https://releases.example.com
CLI_RELEASE_INSTALL_HOSTS=install.example.com
CLI_RELEASE_DEFAULT_BINARY=example-cli
CLI_RELEASE_BINARIES=example-cli,example-admin
```

GitHub-backed deployments fail during startup when `CLI_RELEASE_BINARIES` is
empty. The allowlist controls which assets are indexed, downloaded, and exposed
through install-script routes.

Optional values:

```bash
CLI_RELEASE_GITHUB_API_URL=https://api.github.com
CLI_RELEASE_SERVER_BIND=0.0.0.0:8080
CLI_RELEASE_TAG_PREFIX=v
CLI_RELEASE_ASSET_TEMPLATE={binary}-{version}-{target}.tar.gz
CLI_RELEASE_CLIENT_SERVER_URL_ENV=CLI_RELEASE_SERVER_URL
CLI_RELEASE_INSTALL_DIR_ENV=CLI_RELEASE_INSTALL_DIR
CLI_RELEASE_LOG_LEVEL=info
CLI_RELEASE_LOG_FORMAT=json
```

## Docker

Build the shared image:

```bash
docker build --target cli-release-server-runtime -t cli-release-server:dev .
docker run --rm cli-release-server:dev --help
```

Bake uses the same runtime target:

```bash
docker buildx bake cli-release-server
```

## Helm

Render the reusable chart:

```bash
helm template cli-release-server helm/cli-release-server \
  --set config.releaseGithubRepository=owner/app-repo \
  --set secret.values.releaseGithubToken=dummy-token
```

Validate the chart:

```bash
python3 helm/cli-release-server/tests/assert_cli_release_chart.py
```

Each app repo should override image, hosts, binaries, repository, and secret
source values in its own infrastructure configuration.
The Helm chart keeps the server process bound to `0.0.0.0:8080` so the
Deployment's named container port, probes, and Service target stay aligned.
Use `service.port` and ingress values to change the Kubernetes-facing port or
hostnames; do not set `config.serverBind` in chart values.
When `config.releaseGithubRepository` is set, the chart requires one GitHub
token source: `secret.values.releaseGithubToken`, `secret.existingSecret`, or
`secret.externalSecret.enabled`.
When `config.releaseGithubRepository` is empty, the chart omits
`CLI_RELEASE_GITHUB_REPOSITORY` and `CLI_RELEASE_GITHUB_TOKEN` so the deployment
can serve configured install scripts without a GitHub-backed release provider.

Terraform is not copied into this workspace as a provider-specific deployment
example. App repositories should manage DNS records, Secret Manager entries,
and chart value wiring in their own infrastructure stacks while keeping the
Kubernetes secret key names aligned with the Helm chart.

## Smoke Tests

Run locally with configured example values:

```bash
CLI_RELEASE_PUBLIC_RELEASE_SERVER_URL=http://127.0.0.1:8080 \
CLI_RELEASE_INSTALL_HOSTS=install.example.com \
CLI_RELEASE_DEFAULT_BINARY=example-cli \
CLI_RELEASE_BINARIES=example-cli \
cargo run -p cli-release-server -- serve --bind 127.0.0.1:8080
```

Then check service and installer routes:

```bash
curl http://127.0.0.1:8080/
curl -H 'Host: install.example.com' http://127.0.0.1:8080/
curl http://127.0.0.1:8080/example-cli
```

Release metadata routes require a configured GitHub repository/token and a
published release asset matching `CLI_RELEASE_ASSET_TEMPLATE`.
Install-host routing requires `CLI_RELEASE_DEFAULT_BINARY`, and that binary
must be included in `CLI_RELEASE_BINARIES`.
Configured binary names may contain only ASCII alphanumeric characters, `.`,
`_`, and `-`, matching the release route path-component contract.
Generated install scripts validate archive member paths and extract only the
selected binary, so app release archives should contain a regular file whose
basename matches the configured binary.
