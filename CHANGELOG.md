# Changelog

All notable changes to DevInit CLI are documented here.

This project follows [Semantic Versioning](https://semver.org/) for published releases.


## [Unreleased]

### Added
- GitHub Actions CI for pushes to `development` and pull requests targeting `development` or `master`.
- JSON output for actual file generation using `devinit compile --json`.
- Supported version lists for all packaged services in `devinit list --json`.

### Improved
- Structured reporting of created and overwritten files after successful compilation.
- Preservation of existing `compile --dry-run --json` behavior.
- Automated Rust formatting, Clippy, unit, integration, and Python schema contract checks.

### Testing
- 244 Rust tests passing locally.
- 15 Python schema contract tests passing locally.
- JSON compilation tests covering file creation, explicit overwriting, and refusal to overwrite without `--force`.

### Known limitations
- Multi-file generation is not transactional. A failed write may leave previously generated files on disk.
- The successful-write JSON report does not provide a complete list of partially written files when compilation fails.
- Kubernetes validation covers supported structures, not the complete Kubernetes API specification.

## [1.0.0-rc.2] - 2026-11-08

### Added
- Linux and Windows release-candidate packaging and installation verification.

### Improved
- Release distribution and installation documentation.

## [1.0.0-rc.1] - 2026-11-08

### Added
- Declarative `compile.yaml` workflows for Docker Compose, Kubernetes manifests, GitHub Actions, and GitLab CI/CD.
- Dockerfile discovery enabled by `services: - docker: true`, with generated Compose build contexts.
- Offline built-in registry with 12 service definitions: PostgreSQL, MySQL, MariaDB, MongoDB, Redis, RabbitMQ, Kafka, Elasticsearch, Prometheus, Grafana, S3-compatible MinIO, and LocalStack (EC2 development emulation).
- Global reusable presets and project-level overrides for Compose, Kubernetes, and CI/CD.
- `devinit check`, `devinit compile --dry-run`, and explicit `--force` overwrites.
- `devinit list`, `devinit add`, and supported cached/remote `devinit get` workflows.
- Approved-source downloads of supported Docker images and Linux CLI tools (`kubectl`, `helm`), including checksum verification for downloaded CLI binaries.
- Linux x64 portable package and Windows x64 portable/installer packaging workflows.

### Improved
- Validation of dependencies, host-port conflicts, Kubernetes structures, CI/CD job dependencies, and unsafe output paths.
- Protection of existing generated files by default.
- Explicit failures for unimplemented `test`, `license`, and `login` CLI commands.
- Rejection of unknown top-level `compile.yaml` fields.
- Documentation for installation, configuration, and AI-assisted YAML generation.

### Testing
- 201 unit tests and 28 integration tests passing in the local development branch.
- Portable Linux installation tested with bundled service definitions and a symlink entry point.

### Known limitations
- The CLI does not deploy infrastructure, start containers during compilation, or fully validate external platform schemas.
- Multi-file output generation is not transactional.
- Authentication and some remote configuration workflows are incomplete.
- CLI binary downloads are currently Linux-only; support varies by architecture.