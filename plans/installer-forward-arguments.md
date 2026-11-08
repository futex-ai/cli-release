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

- [ ] Add the forwarding, no-argument, exit-status, cleanup, and argument-privacy requirements to the installer contract.
- [ ] Update the server README with the rule and a generic usage example.

## Milestone 2: Forward arguments and prove the behavior

Depends on: 1

- [ ] Add focused regression tests in `src/_tests_/install_tests.rs` without losing any existing assertion.
- [ ] Prove the new tests fail with the old script.
- [ ] Add the generic post-install call in `src/install.rs`.
- [ ] Run a real generated-script shell smoke with a stub binary.
  Cover spaces, quotes, `--`, no arguments, a nonzero status, one invocation, privacy, and cleanup.
- [ ] Record test costs and focused checks under `.context/installer-forward-arguments/`.

## Milestone 3: Checks, review, and ready pull request

Depends on: 1, 2

- [ ] Run the complete `AGENTS.md` gate and report the known Clippy failure without fixing it.
- [ ] Run the build and relevant release-script, Helm, and image checks.
- [ ] Commit with Conventional Commits and push the new branch.
- [ ] Run a fresh review against `origin/main` with the `implementation-review` skill after the push.
  Apply confirmed findings and re-review any fixes.
- [ ] Delete this plan and preserve its design record in the pull request.
- [ ] Open a ready pull request against `main` and watch its CI checks.
