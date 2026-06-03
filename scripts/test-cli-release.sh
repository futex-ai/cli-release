#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
release_script="$script_dir/cli-release.sh"
repo_root="$(cd "$script_dir/.." && pwd)"
release_config="$repo_root/release-plz.toml"
release_workflow="$repo_root/.github/workflows/release-plz.yml"
dockerignore="$repo_root/Dockerfile.dockerignore"
tmp_dir="$(mktemp -d)"

cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

assert_line() {
  file="$1"
  expected="$2"

  if ! grep -Fx "$expected" "$file" >/dev/null; then
    printf 'missing expected line: %s\n' "$expected" >&2
    printf 'actual output:\n' >&2
    sed -n '1,120p' "$file" >&2
    exit 1
  fi
}

assert_fails() {
  if "$@" >/dev/null 2>&1; then
    printf 'expected command to fail: %s\n' "$*" >&2
    exit 1
  fi
}

link_tool() {
  tool="$1"
  tool_dir="$2"

  tool_path="$(command -v "$tool")"
  ln -s "$tool_path" "$tool_dir/$tool"
}

workspace_version() {
  cargo metadata \
    --no-deps \
    --format-version=1 \
    --manifest-path "$repo_root/Cargo.toml" \
    | jq -r '.packages[] | select(.name == "cli-release-server") | .version'
}

bash -n "$release_script"

version="$(workspace_version)"

"$release_script" resolve-workspace-tag "v${version}" "$repo_root/Cargo.toml" >"$tmp_dir/release.out"
assert_line "$tmp_dir/release.out" "version=$version"
assert_line "$tmp_dir/release.out" "tag=v$version"
assert_line "$tmp_dir/release.out" "release_title=CLI Release v$version"

"$release_script" resolve-workspace-tag "refs/tags/v${version}" "$repo_root/Cargo.toml" >"$tmp_dir/ref.out"
assert_line "$tmp_dir/ref.out" "tag=v$version"

"$release_script" verify-package-manifests "$repo_root/Cargo.toml" >"$tmp_dir/manifests.out"
assert_line "$tmp_dir/manifests.out" "package_manifests=ok"

"$release_script" write-release-pr-config "$release_config" "$tmp_dir/release-plz-pr.toml" >"$tmp_dir/release-pr-config.out"
assert_line "$tmp_dir/release-pr-config.out" "release_pr_config=$tmp_dir/release-plz-pr.toml"
grep -F 'git_only = false' "$tmp_dir/release-plz-pr.toml" >/dev/null
grep -F 'release = true' "$tmp_dir/release-plz-pr.toml" >/dev/null

fixture="$tmp_dir/version-fixture"
mkdir -p "$fixture/crates/app/src" "$fixture/crates/internal-lib/src"
cat >"$fixture/Cargo.toml" <<'TOML'
[workspace]
members = [
    "crates/app",
    "crates/internal-lib",
]
resolver = "2"

[workspace.dependencies]
internal-lib = { path = "crates/internal-lib" }
TOML
cat >"$fixture/crates/app/Cargo.toml" <<'TOML'
[package]
name = "app"
version = "0.2.0"
edition = "2024"

[dependencies]
internal-lib = { workspace = true }
TOML
cat >"$fixture/crates/app/src/lib.rs" <<'RS'
pub fn app_value() -> u8 {
    internal_lib::value()
}
RS
cat >"$fixture/crates/internal-lib/Cargo.toml" <<'TOML'
[package]
name = "internal-lib"
version = "0.1.0"
edition = "2024"
TOML
cat >"$fixture/crates/internal-lib/src/lib.rs" <<'RS'
pub fn value() -> u8 {
    1
}
RS

"$release_script" normalize-workspace-dependency-versions "$fixture/Cargo.toml" >"$tmp_dir/versions.out"
assert_line "$tmp_dir/versions.out" "workspace_dependency_versions=ok"
grep -F 'internal-lib = { version = "0.1.0", path = "crates/internal-lib" }' "$fixture/Cargo.toml" >/dev/null

cat >"$tmp_dir/binaries.json" <<'JSON'
{
  "binaries": [
    { "name": "example-cli", "package": "example-cli-bin" },
    { "name": "example-admin", "package": "example-admin-bin" }
  ]
}
JSON

"$release_script" resolve-binary-package example-cli "$tmp_dir/binaries.json" >"$tmp_dir/binary.out"
assert_line "$tmp_dir/binary.out" "binary=example-cli"
assert_line "$tmp_dir/binary.out" "package=example-cli-bin"
assert_fails "$release_script" resolve-binary-package missing-cli "$tmp_dir/binaries.json"

assert_fails "$release_script" resolve-workspace-tag "app-v${version}" "$repo_root/Cargo.toml"
assert_fails "$release_script" resolve-workspace-tag "v999.999.999" "$repo_root/Cargo.toml"
assert_fails "$release_script" package bad/name 1.2.3 x86_64-apple-darwin /tmp/missing "$tmp_dir/dist"
assert_fails "$release_script" package example-cli v1.2.3 x86_64-apple-darwin /tmp/missing "$tmp_dir/dist"

fake_binary="$tmp_dir/example-cli"
printf '#!/usr/bin/env bash\nexit 0\n' >"$fake_binary"
chmod 755 "$fake_binary"

"$release_script" package \
  example-cli \
  1.2.3 \
  x86_64-apple-darwin \
  "$fake_binary" \
  "$tmp_dir/dist" >"$tmp_dir/package.out"

archive="$tmp_dir/dist/example-cli-1.2.3-x86_64-apple-darwin.tar.gz"

if [ ! -f "$archive" ]; then
  printf 'expected archive to exist: %s\n' "$archive" >&2
  exit 1
fi

assert_line "$tmp_dir/package.out" "asset_name=example-cli-1.2.3-x86_64-apple-darwin.tar.gz"
assert_line "$tmp_dir/package.out" "asset_path=$archive"
assert_line "$tmp_dir/package.out" "checksum_name=example-cli-1.2.3-x86_64-apple-darwin.tar.gz.sha256"
assert_line "$tmp_dir/package.out" "checksum_path=$archive.sha256"

tar -tzf "$archive" | grep -Fx "example-cli-1.2.3-x86_64-apple-darwin/example-cli" >/dev/null

test -f "$archive.sha256"
test "$(wc -c <"$archive.sha256" | tr -d ' ')" -eq 65

sha256sum_only_path="$tmp_dir/sha256sum-only-path"
mkdir -p "$sha256sum_only_path"
for tool in bash mktemp mkdir rm cp chmod tar awk gzip; do
  link_tool "$tool" "$sha256sum_only_path"
done
cat >"$sha256sum_only_path/sha256sum" <<'SH'
#!/usr/bin/env bash
printf 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  %s\n' "$1"
SH
chmod 755 "$sha256sum_only_path/sha256sum"

PATH="$sha256sum_only_path" "$release_script" package \
  example-cli \
  1.2.4 \
  x86_64-unknown-linux-gnu \
  "$fake_binary" \
  "$tmp_dir/sha256sum-dist" >"$tmp_dir/sha256sum-package.out"

sha256sum_archive="$tmp_dir/sha256sum-dist/example-cli-1.2.4-x86_64-unknown-linux-gnu.tar.gz"
assert_line "$tmp_dir/sha256sum-package.out" "checksum_path=$sha256sum_archive.sha256"
test "$(cat "$sha256sum_archive.sha256")" = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

grep -F 'command: release' "$release_workflow" >/dev/null
grep -F 'command: release-pr' "$release_workflow" >/dev/null
grep -F '  pull_request:' "$release_workflow" >/dev/null
test "$(grep -Fc "if: github.event_name != 'pull_request'" "$release_workflow")" -eq 2
grep -F 'name: Checks' "$release_workflow" >/dev/null
grep -F 'run: cargo xtask check' "$release_workflow" >/dev/null
grep -F 'run: scripts/test-cli-release.sh' "$release_workflow" >/dev/null
grep -F 'run: python3 helm/cli-release-server/tests/assert_cli_release_chart.py' "$release_workflow" >/dev/null
grep -F 'run: docker build --target cli-release-server-runtime -t cli-release-server:ci .' "$release_workflow" >/dev/null
grep -F 'needs: checks' "$release_workflow" >/dev/null
grep -F 'release-plz/action@v0.5' "$release_workflow" >/dev/null
assert_fails grep -F 'gh release upload' "$release_workflow"
assert_fails grep -F 'cargo build --release --package' "$release_workflow"

grep -Fx '!crates/**' "$dockerignore" >/dev/null
grep -Fx '!xtask/**' "$dockerignore" >/dev/null
