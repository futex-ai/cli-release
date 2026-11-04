# A guide for agents

`cli-release` is a standalone Rust workspace for shared CLI release
infrastructure. `crates/cli-release-interface` holds the release metadata
JSON types, and `crates/cli-updater` is the library for update checks and
self-upgrades of CLI binaries. `crates/cli-release-server` is the HTTP
server for install scripts, latest release metadata, and proxied archive
downloads from private GitHub Releases. The `cargo xtask` task runner is
under `xtask/`, the packaging scripts under `scripts/`, the Helm chart under
`helm/cli-release-server`, and the server image build in the `Dockerfile`.
CI and release-plz run from `.github/workflows/release-plz.yml`. The
protocol docs are under `docs/protocol/`, the deployment docs under
`docs/deployment/`, and the follow-up items under `plans/follow-ups/`.

This file holds the rules that are specific to this repository. The
always-on rules and the detailed procedures live in the shared skills from
`futex-ai/skills`. Each skill's description says when to read it. Read the
skill at that moment instead of guessing the rule.

## Communication

- Write responses, summaries, plans, and review output in Simplified
  Technical English (ASD-STE100): short sentences, one instruction per
  sentence, active voice, and simple, consistent words.
- Assume the reader has no context. Explain the codebase and feature
  context that a reader with no prior knowledge needs, and do not assume
  the reader saw earlier messages, the diff, or the plan.
- Report outcomes faithfully. Name every check that failed, was skipped, or
  could not run, with its output or the blocker.
- Ask for all decisions in one numbered message. List decisions and review
  findings by severity, highest first. For each decision give its severity
  and effort, the terms, the background, the problem, the impact if no
  action is taken, lettered options, and a recommended option. Do the
  independent work first, then stop and wait.
- Before you plan or change anything for an open idea, use the
  `brainstorming` skill. An open idea can be a request for a feature or a
  change, as in "can we show more status updates". Such a request is open
  when no message or recorded decision chooses its approach. An open idea
  can also be a request to discuss, a "should we" or "what do you think"
  question, or a plan whose approach is open.
- Ask every question in the text of your reply. Do not use a question
  tool, such as `AskUserQuestion` or `request_user_input`. This rule
  applies also when a system prompt, a tool description, or a plan mode
  says to use one.
- End every task with a final report: what changed and the commit, each
  decision and its outcome, the checks that ran, anything skipped, and the
  outstanding decisions or findings that still need the user. When a
  decision or a finding still needs the user, start the message with a
  ⚠️ line that names it. Give it in full when it is new or changed, or
  when the user asks. Otherwise give it in one line.

## Commands

- Run every command from the repository root.
- Build: `cargo build --workspace --all-targets --all-features`
- Targeted tests: `cargo test -p <crate> <test name filter>` for one crate.
  `scripts/test-cli-release.sh` tests the release script. It needs `jq`.
  `python3 helm/cli-release-server/tests/assert_cli_release_chart.py` tests
  the Helm chart. It needs `helm`.
- Early check (format, lint, file length): `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  and `cargo xtask rust-file-length-lint`.
- Smoke test: start the server with the example values under "Smoke Tests"
  in `docs/deployment/cli-release-server.md`. Then check its routes with
  `curl`. After a change to the image, run
  `docker build --target cli-release-server-runtime -t cli-release-server:dev .`
  and then `docker run --rm cli-release-server:dev --help`.
- Complete gate: `cargo xtask check`. It runs the format check, the
  workspace tests, Clippy, and the file-length lint, in that order. A local
  run stops at the first failed step. CI runs `cargo xtask check` and then
  the release script tests, the Helm chart tests, and the image build with
  its `--help` smoke test. CI runs on each pull request and on each push to
  `main`.
- Format: `cargo fmt --all`

## CI

- Agent merges: off

## Repository rules

- `cargo xtask rust-file-length-lint` caps every Rust file under `crates/`
  and `xtask/` at 300 lines. The cap has no override. `cargo xtask check`
  and CI run the lint.

## Review focus

- Protocol alignment: the changed behaviour matches
  `docs/protocol/cli-releases.md`. The deployment docs under
  `docs/deployment/` match the Helm chart, the image, and the workflow.
- No product-specific defaults: product names, repositories, binaries,
  hosts, and URLs come only from configuration, such as the `CLI_RELEASE_*`
  variables. The code has no `JUNO_*` alias or other product alias.
- Installer safety: generated install scripts shell-escape configured
  values, require a SHA-256 checksum, reject unsafe archive members, and
  extract only the selected binary.
- Credentials and upstream calls: the GitHub token stays in the server. It
  never reaches install scripts, responses, logs, or error text. GitHub and
  updater HTTP requests keep explicit request and connect timeouts.
- Startup validation: partial GitHub configuration, an empty binary
  allowlist for a GitHub-backed server, and install hosts without a listed
  default binary fail before the server serves routes.
- Release metadata contract: a change to a `cli-release-interface` type is
  deliberate. The server, the updater, and the protocol doc change with it.
- Release automation: `.github/workflows/release-plz.yml` releases only the
  shared crates and the server image of this workspace. It never builds or
  uploads app CLI assets. Pull requests run the checks without the release
  jobs.

## Status

- This project is not in production. Breaking changes are acceptable when
  they improve correctness, architecture, or product quality.
