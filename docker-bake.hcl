variable "CLI_RELEASE_BUILD_COMMIT" {
  default = "unknown"
}

variable "RELEASE_SERVER_IMAGE_REPOSITORY" {
  default = "cli-release-server"
}

group "default" {
  targets = [
    "cli-release-server",
  ]
}

target "cli-release-server" {
  context = "."
  dockerfile = "Dockerfile"
  target = "cli-release-server-runtime"
  args = {
    CLI_RELEASE_BUILD_COMMIT = "${CLI_RELEASE_BUILD_COMMIT}"
  }
  tags = [
    "${RELEASE_SERVER_IMAGE_REPOSITORY}:${CLI_RELEASE_BUILD_COMMIT}",
    "${RELEASE_SERVER_IMAGE_REPOSITORY}:latest",
  ]
  cache-from = [
    "type=gha,scope=cli-release-server",
  ]
  cache-to = [
    "type=gha,mode=max,scope=cli-release-server",
  ]
  attest = [
    "type=provenance,mode=min,inline-only=true",
  ]
}
