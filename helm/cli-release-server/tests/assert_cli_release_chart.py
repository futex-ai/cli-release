#!/usr/bin/env python3
"""Render assertions for the cli-release-server Helm chart."""

from __future__ import annotations

import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
CHART = ROOT / "helm" / "cli-release-server"


def render(release_channel: str | None = None) -> str:
    command = [
        "helm",
        "template",
        "cli-release-server",
        str(CHART),
        "--set",
        "config.releaseGithubRepository=owner/repo",
        "--set",
        "secret.values.releaseGithubToken=dummy-token",
    ]
    if release_channel is not None:
        command.extend(["--set", f"config.releaseChannel={release_channel}"])
    result = subprocess.run(
        command,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    )
    return result.stdout


def render_missing_token_source() -> str:
    result = subprocess.run(
        [
            "helm",
            "template",
            "cli-release-server",
            str(CHART),
            "--set",
            "config.releaseGithubRepository=owner/repo",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode == 0:
        raise AssertionError("render unexpectedly succeeded without a GitHub token source")
    return result.stdout + result.stderr


def render_install_script_only() -> str:
    result = subprocess.run(
        [
            "helm",
            "template",
            "cli-release-server",
            str(CHART),
            "--set",
            "secret.create=false",
        ],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    )
    return result.stdout


def render_server_bind_override() -> str:
    result = subprocess.run(
        [
            "helm",
            "template",
            "cli-release-server",
            str(CHART),
            "--set",
            "config.serverBind=0.0.0.0:9090",
        ],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode == 0:
        raise AssertionError("render unexpectedly allowed config.serverBind override")
    return result.stdout + result.stderr


def assert_contains(output: str, expected: str) -> None:
    if expected not in output:
        raise AssertionError(f"missing rendered content: {expected}")


def assert_not_contains(output: str, unexpected: str) -> None:
    if unexpected in output:
        raise AssertionError(f"unexpected rendered content: {unexpected}")


def main() -> None:
    output = render()
    for expected in [
        "name: CLI_RELEASE_LOG_LEVEL",
        "value: \"0.0.0.0:8080\"",
        "name: CLI_RELEASE_GITHUB_REPOSITORY",
        "name: CLI_RELEASE_PUBLIC_RELEASE_SERVER_URL",
        "name: CLI_RELEASE_INSTALL_HOSTS",
        "name: CLI_RELEASE_DEFAULT_BINARY",
        "name: CLI_RELEASE_BINARIES",
        "name: CLI_RELEASE_ASSET_TEMPLATE",
        "key: CLI_RELEASE_GITHUB_TOKEN",
        "host: \"releases.example.com\"",
        "host: \"install.example.com\"",
    ]:
        assert_contains(output, expected)
    assert_not_contains(output, "JUNO_RELEASE")
    assert_not_contains(output, "juno.futex.ai")
    for channel in [None, "prerelease", "future-channel"]:
        channel_output = output if channel is None else render(channel)
        expected = channel or "stable"
        assert_contains(
            channel_output,
            f'name: CLI_RELEASE_CHANNEL\n              value: "{expected}"',
        )
    assert_contains(
        render_missing_token_source(),
        "config.releaseGithubRepository requires secret.values.releaseGithubToken",
    )
    install_script_only = render_install_script_only()
    assert_not_contains(install_script_only, "name: CLI_RELEASE_GITHUB_TOKEN")
    assert_not_contains(install_script_only, "key: CLI_RELEASE_GITHUB_TOKEN")
    assert_contains(
        render_server_bind_override(),
        "config.serverBind is not configurable in the Helm chart",
    )


if __name__ == "__main__":
    main()
