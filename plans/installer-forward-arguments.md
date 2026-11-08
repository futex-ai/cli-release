# Run the installed binary with script arguments

Status: Active. On branch `conductor/installer-forward-arguments`, no pull request yet. The completing pull request deletes this plan.

The user requested this implementation on 2026-11-08.
The branch starts at current `origin/main`, commit `e354934`.
It does not include the pre-release channel branch of PR #7.

The user supplied ciproxy design call 30, dated 2026-11-04.
After installing, a generated script with arguments runs the installed binary once with exactly those arguments.
The call uses quoted `"$@"` without splitting, re-quoting, or logging the values.
The script returns the binary's exit status and removes its temporary directory.
With no arguments, the script retains its existing behavior.
The rule applies to every configured binary, with no product default.
No merge, release, or Clippy dependency fix is authorized.

Contract owners:

- [Installer contract](../docs/protocol/cli-releases.md#installer-contract).
- [Server README](../crates/cli-release-server/README.md).

Agent call, 2026-11-08: append a conditional child-process invocation after existing install messages.
Keep the shell process and its existing EXIT trap so cleanup still runs.
Use the existing shell error handling to preserve a failing binary's status.

## Milestone 1: Installer documentation

Depends on: none

- [x] Add the forwarding, no-argument, exit-status, cleanup, and argument-privacy requirements to the installer contract.
- [x] Update the server README with the rule and a generic usage example.

Evidence: `.context/installer-forward-arguments/reports/docs.md`.

## Milestone 2: Forward arguments and prove the behavior

Depends on: 1

- [x] Add focused regression tests in `src/_tests_/install_tests.rs` without losing any existing assertion.
- [x] Prove the new tests fail with the old script.
- [x] Add the generic post-install call in `src/install.rs`.
- [x] Run a real generated-script shell smoke with a stub binary.
  Cover spaces, quotes, `--`, no arguments, a nonzero status, one invocation, privacy, and cleanup.
- [x] Record test costs and focused checks under `.context/installer-forward-arguments/`.

Evidence: `.context/installer-forward-arguments/reports/implementation.md` and `logs/smoke-green.log`.

## Milestone 3: Checks, review, and ready pull request

Depends on: 1, 2

- [x] Run the complete `AGENTS.md` gate and report the known Clippy failure without fixing it.
- [x] Run the build and relevant release-script, Helm, and image checks.

The Rust 1.99 gate passes all 47 tests, then stops at the known `async-trait` Clippy failure.
The same complete gate passes with Rust 1.89, matching the Dockerfile.
Existing [fix PR #6](https://github.com/futex-ai/cli-release/pull/6) owns the macro dependency fix.
Evidence: `.context/installer-forward-arguments/reports/verification.md`.

On 2026-11-08, the user reported that PR #6 merged as `4a57677`.
The user authorized integration of current `origin/main` before the final checks.
Merge `647df05` takes main's lockfile without conflicts and retains every installer change.
Its remerge audit has exactly two parents and no conflict-resolution diff.
The final default-toolchain gate must pass on this combined tree before the ready pull request.
Integration evidence: `.context/installer-forward-arguments/logs/main-merge-audit.log`.

During review, PR #7 merged into main as `bc59b05`.
Merge `3ee412c` integrates that current-main change without conflicts or a remerge diff.
The feature diff against main still contains only installer work.
The combined protocol now exceeds its length target, so the installer contract moves to a linked protocol page without losing requirements.

- [x] Split the installer protocol section and check every moved requirement and live link.

Evidence: `.context/installer-forward-arguments/reports/docs-main-integration.md`.
- [ ] Commit with Conventional Commits and push the new branch.
- [ ] Run a fresh review against `origin/main` with the `implementation-review` skill after the push.
  Apply confirmed findings and re-review any fixes.
- [ ] Delete this plan and preserve its design record in the pull request.
- [ ] Open a ready pull request against `main` and watch its CI checks.
