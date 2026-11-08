# DevInit CLI

**Define once. Validate safely. Generate infrastructure configurations with confidence.**

DevInit is a Rust command-line tool for declarative infrastructure configuration generation. Describe the infrastructure you need in a `compile.yaml` file, validate it, and generate configuration files for **Docker Compose, Kubernetes, GitHub Actions, and GitLab CI/CD**.

DevInit follows a simple philosophy: **minimum configuration, maximum automation**. It favors sensible defaults, optional advanced settings, offline-first service recipes, safe file handling, and actionable errors.

> **Current version:** `1.0.0-rc.2` (release candidate, not yet published)
> **Platforms:** Linux x86-64 (GNU/glibc), Windows x86-64
> **License:** Apache-2.0 — see [LICENSE](LICENSE)
> **Release status:** Packaging and installation are being validated before publication.

## What DevInit does

- **Declarative generation:** Generate supported Docker Compose, Kubernetes, GitHub Actions, and GitLab CI/CD files from one `compile.yaml`.
- **Dockerfile discovery:** Opt in to detecting Dockerfiles in your project and adding their build contexts to the generated Compose configuration.
- **Offline-first registry:** Use 12 packaged service recipes and supported versions without relying on a web backend.
- **Reusable presets:** Define global service, Kubernetes, and CI/CD presets and apply controlled project-level overrides.
- **Safe generation:** Validate with `check`, preview with `compile --dry-run`, and avoid overwriting files unless `--force` is explicitly supplied.
- **Resource downloads:** Download approved Docker images and, on supported Linux platforms, verified `kubectl` and Helm binaries.

DevInit **generates configuration files**. `compile` does not build images, start containers, deploy workloads, install dependencies, or provision cloud resources. Its validation is not a replacement for Docker Compose, Kubernetes API, or CI provider validation.

## Quick start

Create `compile.yaml` in your project directory:

```yaml
services:
  - name: backend
    image: nginx:alpine
    ports: ["8080:80"]
  - name: postgresql
    version: "17"
  - name: redis
    version: "7"
```

Run these commands from that directory:

```bash
devinit check
devinit compile --dry-run
devinit compile
```

The example generates `docker-compose.yaml`. The first compile creates the file; subsequent compiles refuse to overwrite it by default. Use `devinit compile --dry-run --force` to preview a replacement and `devinit compile --force` only when you intend to overwrite existing generated files.

> The packaged PostgreSQL recipe requires a `POSTGRES_PASSWORD` environment value when Docker Compose resolves the generated file. Set credentials securely; do not commit production secrets.

### Discover existing Dockerfiles

Given `backend/Dockerfile` and `frontend/Dockerfile` in your project, use:

```yaml
services:
  - docker: true
  - name: redis
    version: "7"
```

DevInit discovers the Dockerfiles and creates Compose build services with appropriate contexts, alongside Redis. The discovery directive is **`- docker: true` inside `services`**. A top-level `docker_discovery: true` is not valid.

### Kubernetes and CI/CD

Use `k8s` for Kubernetes templates or raw resources and `cicd` for supported pipeline definitions:

```yaml
k8s:
  backend:
    template: deployment
    name: backend
    image: nginx:alpine
    replicas: 2
    port: 80
  backend-service:
    template: service
    name: backend
    target: backend
    port: 80
    targetPort: 80
cicd:
  provider: github
  triggers:
    - type: push
      branches: [main]
  jobs:
    - name: build
      steps:
        - name: Checkout
          action: checkout
        - name: Build
          action: run
          command: cargo build
```

This creates Kubernetes files under `k8s/` and a GitHub Actions workflow at `.github/workflows/devinit.yml`. The CI job above is illustrative for a Rust project; choose commands appropriate for your own codebase. See [Configuration Reference](docs/CONFIG_REFERENCE.md) for supported syntax.


## Installation

DevInit provides prebuilt packages for Linux and Windows.
Rust and Cargo are not required to run the packaged CLI.

> **Release status:** Version `1.0.0-rc.2` is under
> pre-release validation. Download links and automated
> installation become available after the release assets
> are published on GitHub.

### Linux x86-64

The Linux installer is designed to download the official
release, verify its SHA-256 checksum, and install DevInit
into a directory already available through PATH.

The intended installation command for stable releases is:

```bash
curl -fsSL https://raw.githubusercontent.com/GorkiyChocolate/DevInit-CLI/development/scripts/install.sh -o install.sh
bash install.sh
```

Review the installation script before executing it.
Until a stable release is published, use an explicitly
selected version:

```bash
DEVINIT_VERSION=v1.0.0-rc.2 bash install.sh
```

The installer prefers `~/.local/bin` when it is already
in PATH. Otherwise, it can use `/usr/local/bin` with sudo.
It does not require users to manually edit shell profiles.

Linux support currently targets x86-64 systems using glibc.
Alpine Linux and other architectures are not covered by
this release package.

### Windows x86-64

Download `DevInit-Setup-<version>.exe` from
[GitHub Releases](https://github.com/GorkiyChocolate/DevInit-CLI/releases)
when available.

The installer targets the current user's application
directory and is designed to register DevInit in the
user's PATH without administrator privileges.

After installation, open a new PowerShell terminal:

```powershell
devinit --version
devinit list
```

Automatic PATH registration and uninstall behavior are
implemented but still awaiting Windows integration testing.

A portable ZIP package is also planned.

### Build from source

Requires Git and a Rust toolchain supporting edition 2024.

```bash
git clone https://github.com/GorkiyChocolate/DevInit-CLI.git
cd DevInit-CLI
git switch development
cargo build --release --locked
./target/release/devinit --version
```

On Windows, the executable is
`target\release\devinit.exe`.

### Verify installation

```bash
devinit --version
devinit list
```

The 12 built-in service definitions are embedded in
the compiled executable. They work without an external
service registry or a running DevInit backend.

## Built-in service registry

`devinit list` shows the packaged recipes and their default versions. There are currently **12**:

| Recipe | Description |
| --- | --- |
| `postgresql` | PostgreSQL relational database |
| `mysql` | MySQL relational database |
| `mariadb` | MariaDB relational database |
| `mongodb` | MongoDB document database |
| `redis` | Redis in-memory store |
| `rabbitmq` | RabbitMQ message broker |
| `kafka` | Apache Kafka |
| `elasticsearch` | Elasticsearch search and analytics |
| `prometheus` | Prometheus monitoring |
| `grafana` | Grafana visualization |
| `s3` | Local S3-compatible storage (MinIO-based recipe) |
| `ec2` | Local AWS emulation (LocalStack-based recipe) |

Available versions depend on the bundled registry. For example, PostgreSQL 16 is currently the default while `version: "17"` is also supported.

```bash
devinit list
devinit add redis
devinit add postgresql --version 17
```

`add` writes recipe data to a local DevInit cache. It does **not** automatically insert a service into `compile.yaml` or start a container. Use `devinit add --help` for current options.

## Verified downloads

```bash
devinit download redis
devinit download kubectl
devinit download helm
```

Approved Docker image download names are `redis`, `postgresql`, `kafka`, `prometheus`, and `grafana`. Docker image downloads require a working Docker Engine and use approved repositories. The CLI rejects arbitrary image references or unapproved download names.

`kubectl` and Helm downloads are supported on **Linux x86-64 and ARM64**, not through the Windows binary. DevInit resolves a supported release, verifies SHA-256, and saves the tool to `~/.local/share/devinit/tools/`. It does not automatically add downloaded tools to `PATH`. Binary downloads require network access; Linux supports resumable transfers.

## Commands

| Command | Status and purpose |
| --- | --- |
| `devinit check` | Validate the project and generation plan without creating output files. |
| `devinit compile` | Generate declared infrastructure files; protect existing outputs. |
| `devinit compile --dry-run` | Preview file actions without writing. |
| `devinit compile --force` | Allow explicit overwriting (filesystem safeguards still apply). |
| `devinit list` | Show packaged service recipes. |
| `devinit add <service>` | Add a recipe to the local cache. |
| `devinit download <name>` | Download an approved Docker image or supported CLI tool. |
| `devinit get <source>` | Experimental retrieval; behavior depends on local cache and backend availability. |
| `devinit login <url>` | Reserved; authentication is not implemented. Returns an error. |
| `devinit license <type>` | Reserved; not implemented. Returns an error. |
| `devinit test` | Reserved; not implemented. Returns an error. |


### Custom configuration files

DevInit uses `compile.yaml` by default. To select another
configuration file, use `-f` or `--file`:

```bash
devinit check -f configs/staging.yaml
devinit compile -f configs/staging.yaml --dry-run
devinit compile --file configs/production.yaml
```

Relative paths are resolved from the working directory.
Paths to configuration files may contain spaces.

### Structured JSON output

Machine-readable JSON output is available for supported
read-only and preview operations:

```bash
devinit list --json
devinit check --json
devinit check -f configs/staging.yaml --json
devinit compile --dry-run --json
```

JSON output is useful for CI pipelines, scripts, editors,
and external automation tools.

A validation failure returns a non-zero exit status.
The JSON response provides structured status and error
information.

Regular `compile` continues to generate files using
its normal human-readable output.

Use `devinit <command> --help` for arguments accepted by your installed version. Registered commands are not necessarily fully implemented.

## Configuration, presets, and output files

Project configuration is `compile.yaml`. Supported top-level keys are `services`, `k8s`, and `cicd`; unknown top-level fields are rejected. Presets are optional and stored separately in a global `devinit.yaml`:

- Linux: `~/.config/devinit/devinit.yaml` or `$XDG_CONFIG_HOME/devinit/devinit.yaml`.
- Windows: `%APPDATA%\DevInit\devinit.yaml`.

Preset definitions can include `docker_compose`, `k8s`, `cicd`, and `default_url`; project declarations may select known presets using `use` with supported `overrides`.

The default backend URL is resolved in this order: `DEVINIT_API_URL`, global `default_url`, then `http://localhost:5167`. The local built-in registry and configuration generators do not require a running DevInit backend.

| Input | Typical output |
| --- | --- |
| `services` / Dockerfile discovery | `docker-compose.yaml` |
| `k8s` manifests and templates | `k8s/<name>.yaml` |
| GitHub CI/CD | `.github/workflows/devinit.yml` |
| GitLab CI/CD | `.gitlab-ci.yml` |
| Service recipe `files` | Additional declared paths |

## Safety and limitations

- No automatic overwrite without `--force`; `--dry-run` writes no generated files, including with `--force`.
- Validates dependencies, supported port rules, CI job graphs, selected Kubernetes structures, and generated file paths.
- Raw Kubernetes documents and CI pipelines must still be validated against their target environments.
- Generation of multiple files is **not fully transactional**; an unexpected I/O error can leave some outputs written.
- Local cache, packaged recipe versions, remote integration, and supported platforms have defined limits.
- DevInit does not generate application code or automatically perform deployments.

## Documentation

- [User Guide](docs/USER_GUIDE.md) — workflows, installation, examples, and troubleshooting.
- [Configuration Reference](docs/CONFIG_REFERENCE.md) — input fields, templates, and output paths.
- [AI Instructions](docs/AI_INSTRUCTIONS.md) — conservative rules for generating `compile.yaml` with an AI assistant.
- [AI Configuration Specification](docs/DEVINIT_AI_CONFIGURATION_SPEC.md) and [Project Documentation](docs/PROJECT_DOCUMENTATION.md) — additional project materials; verify their version-specific claims against current code.
- [JSON Schema](schemas/compile.schema.json) —
  Draft 2020-12 schema for compile configuration.
  It supports editor completion and structural validation.
  DevInit's own validator remains authoritative for
  registry-dependent and semantic checks.


## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo build --release --locked
python tests/schema_validation.py
```

The `1.0.0-rc.2` development snapshot passed
239 Rust tests and 15 Python JSON Schema contract tests
(254 checks in total), along with Clippy and rustfmt
verification. Test counts may change as development continues.

For bugs and proposals, see [GitHub Issues](https://github.com/GorkiyChocolate/DevInit-CLI/issues). The DevInit CLI is part of an Astana IT University diploma project focused on declarative infrastructure configuration management, versioning, distribution, and CLI-oriented workflows.

## License

DevInit is licensed under the Apache License, Version 2.0.

See [LICENSE](LICENSE) for the full license text.
