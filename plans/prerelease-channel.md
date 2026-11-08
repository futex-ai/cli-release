# Release-server pre-release channel

Status: Active. On branch `conductor/release-server-pre-release-channel-ciproxy-item-103-b`, no pull request yet. The completing pull request deletes this plan.

The user requested implementation, complete checks, a commit, a push, and a ready pull request on 2026-11-08.
The server needs a deployment channel for preview installers and self-updates.
The chosen approach adds `stable` and `prerelease` channels to the existing GitHub provider.
The user authorized this approach and delegated the choice of release order.
No merge or release publication is authorized.

Contract owners:

- [CLI release protocol](../docs/protocol/cli-releases.md).
- [Server README](../crates/cli-release-server/README.md).
- [Deployment configuration](../docs/deployment/cli-release-server.md).

## Design calls

- Agent call, 2026-11-08: use `CLI_RELEASE_CHANNEL` and Helm `config.releaseChannel`.
  These names follow the existing deployment configuration.
- Agent call, 2026-11-08: keep GitHub's latest full-release endpoint for `stable`.
  This preserves the production default and its tag-prefix validation.
- Agent call, 2026-11-08: select the first matching non-draft release in GitHub release-list order for `prerelease`.
  Read later pages when necessary.
  This follows the current GitHub selection model and supports the existing tag suffix contract without semantic-version restrictions.
  Full releases also qualify.
- Agent call, 2026-11-08: warn and use `stable` for an unknown channel value.
  This follows the repository's graceful handling rule and keeps preview releases opt-in.
- Agent call, 2026-11-08: return HTTP 404 when no release matches.
  This gives the preview channel an explicit empty-result response.
  Stable keeps its existing upstream error mapping.
- Agent call, 2026-11-08: retain exact tag lookups for versioned downloads.
  The installer and updater receive channel selection through the existing latest metadata route.
  The updater keeps its semantic-version comparison and requires force for downgrades.

## Milestone 1: Documentation and protocol

Depends on: none

- [ ] Describe the channel, release order, empty result, and exact downloads in the protocol.
- [ ] Update the root README, server README, and deployment configuration docs.

## Milestone 2: Channel selection and regression tests

Depends on: 1

- [ ] Add the channel configuration and GitHub selection with pagination.
- [ ] Expose the setting in Helm without adding a validation gate.
- [ ] Add focused tests for both channels, drafts, prefixes, empty results, pagination, and exact downloads.
- [ ] Prove each new test fails with the old behavior and record its runtime.
- [ ] Run every new asynchronous test ten times.

Evidence lives under `.context/prerelease-channel/`.

## Milestone 3: Verification, review, and pull request

Depends on: 1, 2

- [ ] Run the early checks and a server smoke test with installer and updater coverage.
- [ ] Run `cargo xtask check`, the workspace build, release script tests, Helm chart tests, and image checks when available.
- [ ] Commit with Conventional Commits and push the current branch.
- [ ] Review the complete diff against `origin/main` with the `implementation-review` skill after the push.
  Apply the finding rule and re-review fixes once.
- [ ] Delete this plan under the `plans` skill and preserve its dated record in the pull request.
- [ ] Open a ready pull request against `main` and wait for its CI checks.
