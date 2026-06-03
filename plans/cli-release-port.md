# CLI Release Port

## Summary

Port the CLI release crates and related release infrastructure from
`/Users/calummoore/projects/futex/juno` into this workspace, then make the
release server and updater generic enough that each CLI repository can deploy
the same code with environment-driven configuration.

## Investigation Findings

- The current workspace only contains repository instructions and a minimal
  README.
- The source repo has the three requested crates:
  `crates/cli-release-interface`, `crates/cli-updator`, and
  `crates/cli-release-server`. Port `cli-updator` as `cli-updater` so the new
  workspace uses the standard spelling before it has public consumers.
- `cli-release-server` depends on `juno-bin-logging`; the port must either copy
  and rename that logging helper or replace it with generic tracing bootstrap
  code.
- The source server hardcodes Juno install and release domains in
  `src/install.rs`, hardcodes supported binaries there, and reads GitHub
  repository/token from `JUNO_RELEASE_*` env vars.
- The source updater hardcodes `https://releases.juno.futex.ai` and
  `JUNO_RELEASE_SERVER_URL`.
- The GitHub provider assumes `v<version>` tags and
  `<binary>-<version>-<target>.tar.gz` asset names.
- Related source content includes `release-plz.toml`, `.github/workflows/release.yml`,
  `scripts/cli-release.sh`, `scripts/test-cli-release.sh`, Dockerfile and Bake
  release-server targets, `helm/cli-release-server`, production GCP
  release-host variables/docs, and CLI release documentation.

## Implementation Decisions

- Use `CLI_RELEASE_*` as the only release-server and updater environment
  variable prefix. Do not keep `JUNO_*` compatibility aliases.
- Support one GitHub repository per release-server deployment, with one or more
  configured CLI binaries served from that repository.
- Rename the copied updater crate to `cli-updater`. Rust callers import it as
  `cli_updater`, which is the conventional module path for a package named
  `cli-updater`.
- Actual CLI binary releases are owned by the app-specific repositories. This
  repository may provide reusable scripts/templates, but it must not run the
  app-specific release-plz flow or publish app CLI assets from this repo.
- Add release-plz in this repository for versioning and releasing the shared
  crates and shared release-server package/image surface owned by this
  workspace.

## Milestone 14: Pull Request Workflow Checks

Summary: implement the user-selected review fix so pull requests run the shared
check suite without running release-plz publishing jobs.

- [x] Add a failing regression showing the release-plz workflow has a
  `pull_request` trigger and skips release jobs on pull requests.
- [x] Confirm the regression fails against the current workflow.
- [x] Add the `pull_request` workflow trigger.
- [x] Guard release-plz publishing jobs so they do not run on pull request
  events.
- [x] Update docs for the pull request workflow behavior.
- [x] Run focused release script workflow tests.
- [x] Run relevant checks.
- [x] Run `cargo xtask check`.
- [x] Run `cargo xtask review` after `cargo xtask check`.
- [x] Report `cargo xtask review` findings without automatically fixing them.

## Milestone 1: Bootstrap The Standalone Workspace

Summary: copy the Rust crates and enough repository scaffolding for the
workspace to compile and run tests without relying on the Futex source tree.

- [x] Copy `crates/cli-release-interface`, source `crates/cli-updator` into
  `crates/cli-updater`, and `crates/cli-release-server`, excluding `.DS_Store`
  files.
- [x] Add a root `Cargo.toml` with the copied crates as workspace members and
  workspace dependency entries.
- [x] Decide the logging strategy for `cli-release-server`: copy and rename
  `juno-bin-logging`, or replace it with generic in-crate tracing bootstrap.
- [x] Add or generate `Cargo.lock`.
- [x] Add `.cargo/config.toml` with the `xtask` alias if an xtask crate is
  copied or created.
- [x] Add a minimal `xtask` crate or port the source `xtask` checks needed for
  `cargo xtask check` and `cargo xtask review`.
- [x] Run `cargo fmt --all -- --check`; if it fails, format and rerun.
- [x] Run focused tests for the copied crates.
- [x] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.

## Milestone 2: Make Release Configuration Generic

Summary: replace Juno-specific defaults with a typed configuration model that
is read at the server composition boundary and passed into routes, install
script rendering, and GitHub release lookup.

- [x] Add failing tests that prove install hosts, release URLs, default binary,
  supported binaries, tag prefix, and asset template are not hardcoded.
- [x] Add `cli-release-server` config types for the env vars documented in
  `docs/protocol/cli-releases.md`.
- [x] Update `routes::app_from_env` to build a provider and installer routes
  from typed config instead of direct ambient env reads.
- [x] Update install-script rendering to use configured binary metadata, public
  release URL, install directory env var, and client release-server override
  env var.
- [x] Update GitHub release lookup to use configured repository, API URL,
  token, tag prefix, and asset template.
- [x] Reject invalid configuration with explicit `thiserror` enum variants and
  no `unwrap` or `expect` in production code.
- [x] Update `cli-updater` so consuming binaries pass a default release-server
  URL and override env var name instead of using Juno-specific constants.
- [x] Update all crate README files and protocol docs to describe the generic
  configuration contract.

## Milestone 3: Port Shared Release Tooling And Docker Support

Summary: bring over reusable release packaging and image build support without
turning this repository into the publisher for app-specific CLI binaries.

- [x] Copy `scripts/cli-release.sh` and `scripts/test-cli-release.sh`.
- [x] Refactor release script binary metadata so binary/package mappings come
  from config rather than a shell `case` with Juno binaries.
- [x] Do not copy the source `.github/workflows/release.yml` as an active
  workflow that publishes Juno/app CLI assets from this repository.
- [x] Extract only reusable workflow guidance or a reusable workflow template
  that app repositories can call with their own binary/package/target matrix.
- [x] Add `release-plz.toml` for this workspace's shared crates, with
  `cli-release-interface`, `cli-updater`, and `cli-release-server` included in
  the versioning/changelog policy.
- [x] Add an in-repo release-plz workflow that creates release PRs/tags for
  this workspace's shared crates only.
- [x] Ensure the release-plz workflow does not build or upload app-specific CLI
  binary assets.
- [x] Document that app repositories run release-plz for their own CLI crates,
  create their own release tags, and upload their own CLI binary assets.
- [x] Copy the Dockerfile release-server runtime target or create a narrower
  Dockerfile for this workspace.
- [x] Copy or recreate `docker-bake.hcl` with a release-server target only.
- [x] Copy Docker ignore rules needed for stable release-server build context.
- [x] Run `scripts/test-cli-release.sh`.
- [x] Build the release-server image target and smoke-test the
  `cli-release-server --help` command inside the image.

## Milestone 4: Port Helm, Terraform, And Deployment Docs

Summary: provide deployment assets that can deploy the generic release server
for any CLI repository by changing values and secrets.

- [x] Copy `helm/cli-release-server`.
- [x] Replace Juno domains, secret names, image repositories, and env names in
  chart values with generic defaults and overridable values.
- [x] Add Helm template tests for rendered env vars, install/release hosts,
  ExternalSecret key mapping, and ingress TLS hosts.
- [x] Decide whether Terraform belongs in this repository as reusable examples
  or only as documentation for consumer repos.
- [x] Do not copy provider-specific Terraform into this workspace; document
  that app repositories own DNS, secret-manager, and chart-value wiring.
- [x] Update deployment docs so they describe per-CLI-repo deployments without
  hardcoded Juno infrastructure.

## Milestone 5: Documentation And Consumer Integration

Summary: make the copied code understandable as a standalone project and
document how external CLI repositories consume it.

- [x] Update the root README with features, user-facing endpoints, developer
  setup, key code links, and links to protocol docs and plans.
- [x] Ensure every copied Rust crate README has the required sections in the
  required order.
- [x] Add docs for release metadata, install scripts, updater integration,
  release workflow usage, Docker image build, Helm deployment, and smoke tests.
- [x] Add an example consumer configuration showing one repository with several
  CLI binaries and one release-server deployment.
- [x] Search copied docs for stale Juno domains, binary names, and
  repository-specific claims; keep them only in clearly labeled examples.

## Milestone 6: Final Validation And Review

Summary: verify the full port, run the required workspace checks, and capture
review findings for user decision.

- [x] Run `cargo fmt --all -- --check`.
- [x] Run all relevant Rust tests with 100% pass rate.
- [x] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [x] Run release script tests.
- [x] Run Helm template tests.
- [x] Run Docker build smoke tests for the release-server image.
- [x] Start `cli-release-server` locally with fixture or test configuration and
  smoke-test `/`, install-script routes, and release metadata routes.
- [x] Run `cargo xtask check`.
- [x] Run `cargo xtask review` after `cargo xtask check`.
- [x] Do not automatically fix `cargo xtask review` findings. Number each
  finding in the final response with context, solution options, and a clear
  recommendation.

## Milestone 7: Review Follow-Ups

Summary: apply the selected review findings and chart layout rename without
changing the completed port milestones.

- [x] Add regression coverage for partial GitHub repository/token
  configuration and archive-layout-agnostic install scripts.
- [x] Update install scripts to save downloaded archives by basename and
  discover the binary inside the extracted archive.
- [x] Fail server startup when exactly one of `CLI_RELEASE_GITHUB_REPOSITORY`
  and `CLI_RELEASE_GITHUB_TOKEN` is configured.
- [x] Add Helm validation that a configured GitHub repository has a token
  source.
- [x] Move the reusable chart to `helm/cli-release-server`.
- [x] Update docs, scripts, and workflows for the new chart path and GitHub
  config validation contract.
- [x] Run focused Rust and Helm tests for the review follow-ups.
- [x] Run `cargo fmt --all -- --check`.
- [x] Run all relevant Rust tests with 100% pass rate.
- [x] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [x] Run release script tests and Helm chart tests.
- [x] Run release-server image and local server smoke tests.
- [x] Run `cargo xtask check`.
- [x] Run `cargo xtask review` after `cargo xtask check`.
- [x] Report `cargo xtask review` findings without automatically fixing them.

## Milestone 13: GitHub Binary Allowlist Enforcement

Summary: implement the user-selected review fixes for release-index allowlist
filtering and GitHub-backed startup validation.

- [x] Add a failing regression showing `/releases/latest` ignores malformed
  checksum metadata for unconfigured binaries before checksum work.
- [x] Add a failing regression showing GitHub-backed config requires a non-empty
  binary allowlist.
- [x] Confirm both regressions fail against the current implementation.
- [x] Pass the configured binary allowlist into release-index assembly and skip
  unconfigured assets before checksum resolution.
- [x] Fail configuration when GitHub is configured and no release binaries are
  configured.
- [x] Update docs and README content for the GitHub binary allowlist contract.
- [x] Run focused config/GitHub/route tests.
- [x] Run Helm chart and release script tests.
- [x] Run relevant Rust checks.
- [x] Run `cargo xtask check`.
- [x] Run `cargo xtask review` after `cargo xtask check`.
- [x] Report `cargo xtask review` findings without automatically fixing them.

## Milestone 12: Helm Fixed Server Port

Summary: implement the user-selected review fix to keep the reusable Helm chart
server bind port fixed at `8080`.

- [x] Add a failing Helm regression for `config.serverBind` overriding the
  chart-managed server port.
- [x] Confirm the Helm regression fails against the current chart.
- [x] Remove or validate the Helm `config.serverBind` override while keeping the
  server process bound to `0.0.0.0:8080`.
- [x] Update deployment docs for the fixed Helm bind port.
- [x] Run Helm chart tests.
- [x] Run relevant Rust checks.
- [x] Run `cargo xtask check`.
- [x] Run `cargo xtask review` after `cargo xtask check`.
- [x] Report `cargo xtask review` findings without automatically fixing them.

## Milestone 11: Portable Checksums And Protocol Cleanup

Summary: implement the user-selected review fixes for portable release
package checksums and standalone protocol documentation.

- [x] Add a failing regression for package checksum generation when only
  `sha256sum` is available.
- [x] Confirm the checksum regression fails against the current release script.
- [x] Update release packaging to prefer `sha256sum` and fall back to `shasum`.
- [x] Remove local source-tree provenance from the protocol spec.
- [x] Run the release script regression test.
- [x] Run relevant Rust/Helm checks.
- [x] Run `cargo xtask check`.
- [x] Run `cargo xtask review` after `cargo xtask check`.
- [x] Report `cargo xtask review` findings without automatically fixing them.

## Milestone 10: Shell Safety And HTTP Timeouts

Summary: implement the user-selected review fixes for safe installer script
configuration and bounded HTTP requests.

- [x] Add failing regressions for invalid install binary names and release URL
  shell substitution in generated installers.
- [x] Confirm installer safety regressions fail against the current
  implementation.
- [x] Add failing timeout regressions for updater and GitHub provider clients.
- [x] Confirm timeout regressions fail against the current implementation.
- [x] Validate configured install binaries with the release path-component
  contract.
- [x] Shell-escape literal values emitted into generated installer scripts.
- [x] Build updater and GitHub HTTP clients with explicit request/connect
  timeouts.
- [x] Update docs and READMEs for binary-name validation and HTTP timeout
  behavior.
- [x] Run focused installer/config/client timeout tests.
- [x] Run `cargo fmt --all -- --check`.
- [x] Run all relevant Rust tests with 100% pass rate.
- [x] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [x] Run release script tests and Helm chart tests.
- [x] Run release-server image and local server smoke tests.
- [x] Run `cargo xtask check`.
- [x] Run `cargo xtask review` after `cargo xtask check`.
- [x] Report `cargo xtask review` findings without automatically fixing them.

## Milestone 9: Installer Archive Safety

Summary: implement review finding 1 option A by validating archive member paths
and extracting only the selected binary from the generated install script.

- [x] Add a failing regression that exercises an archive containing a traversal
  member.
- [x] Confirm the regression fails against the current generated installer.
- [x] Update the generated installer to reject unsafe archive members before
  extraction.
- [x] Extract only the selected binary to a fixed temporary path.
- [x] Update installer protocol/docs for the archive safety contract.
- [x] Run focused installer tests.
- [x] Run `cargo fmt --all -- --check`.
- [x] Run all relevant Rust tests with 100% pass rate.
- [x] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [x] Run release script tests and Helm chart tests.
- [x] Run release-server image and local server smoke tests.
- [x] Run `cargo xtask check`.
- [x] Run `cargo xtask review` after `cargo xtask check`.
- [x] Report `cargo xtask review` findings without automatically fixing them.

## Milestone 8: Selected Review Fixes

Summary: implement the user-selected recommended fixes from the Milestone 7
review output.

- [x] Add failing regressions for Docker context recursion, install-script-only
  Helm rendering, relative updater download URLs, and missing install-host
  default binaries.
- [x] Confirm the new regressions fail against the current implementation.
- [x] Fix `Dockerfile.dockerignore` so nested crate and xtask files are
  included in the image build context.
- [x] Render `CLI_RELEASE_GITHUB_TOKEN` only when the Helm deployment has a
  configured GitHub repository/token source.
- [x] Normalize non-absolute updater `download_url` values before joining them
  with the release-server base URL.
- [x] Require a configured supported default binary whenever install hosts are
  configured.
- [x] Update docs for the corrected Helm and install-host configuration
  contracts.
- [x] Run focused tests for the four selected fixes.
- [x] Run `cargo fmt --all -- --check`.
- [x] Run all relevant Rust tests with 100% pass rate.
- [x] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [x] Run release script tests and Helm chart tests.
- [x] Run release-server image and local server smoke tests.
- [x] Run `cargo xtask check`.
- [x] Run `cargo xtask review` after `cargo xtask check`.
- [x] Report `cargo xtask review` findings without automatically fixing them.
