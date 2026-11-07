# DevInit CLI

**Define once. Validate safely. Generate infrastructure configurations with confidence.**

DevInit is a Rust command-line tool for declarative infrastructure configuration generation. Describe the infrastructure you need in a `compile.yaml` file, validate it, and generate configuration files for **Docker Compose, Kubernetes, GitHub Actions, and GitLab CI/CD**.

DevInit follows a simple philosophy: **minimum configuration, maximum automation**. It favors sensible defaults, optional advanced settings, offline-first service recipes, safe file handling, and actionable errors.

> **Status:** Active development; preparing a v1.0.0 release candidate.  
> **Current source version:** `0.0.4` (not yet v1.0.0).  
> **Packaged CLI platforms:** Linux x64 and Windows x64.  
> **License:** No license file has been provided yet. See [License](#license).

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

Installable release assets will be linked from [GitHub Releases](https://github.com/GorkiyChocolate/DevInit-CLI/releases) when published. A successful CI run does **not** imply a public release has been published.

### Linux x64 — portable, no sudo

Download `devinit-linux-x64.tar.gz` when it is available as a release asset, then:

```bash
mkdir -p ~/.local/opt/devinit
tar -xzf devinit-linux-x64.tar.gz -C ~/.local/opt/devinit
~/.local/opt/devinit/devinit --version
~/.local/opt/devinit/devinit list
```

Keep the executable and bundled service definitions together:

```text
~/.local/opt/devinit/
├── devinit
└── resources/
    └── services/
        └── *.yaml  (12 bundled recipes)
```

Optionally add the executable to your command path using a symlink:

```bash
mkdir -p ~/.local/bin
ln -s ~/.local/opt/devinit/devinit ~/.local/bin/devinit
```

Make sure `~/.local/bin` is on your `PATH`. This direct and symlink layout has been tested on Linux. **Do not install only the binary** while discarding `resources/services`: registry commands require those files. For a system-wide layout, install `devinit` to `/usr/local/bin/` and service definitions to `/usr/local/share/devinit/services/`.

### Windows x64

When released, choose the Windows ZIP (portable) or the `DevInit-Setup-<version>.exe` installer. Keep `devinit.exe` together with its `resources\services\` directory in a portable installation. The installer targets the current user's application directory and does not request administrator privileges.

To verify, open PowerShell in the folder containing `devinit.exe` and run:

```powershell
.\devinit.exe --version
.\devinit.exe list
```

The current installer script does not establish that `devinit` is automatically added to `PATH`. If Windows does not recognize `devinit` as a global command, invoke the executable using its full path or configure `PATH` yourself. Windows packaging has passed CI; interactive installer behavior should be verified on an actual Windows machine before final release claims.

### Build from source

Requires Git, Cargo, and a Rust toolchain supporting edition 2024.

```bash
git clone https://github.com/GorkiyChocolate/DevInit-CLI.git
cd DevInit-CLI
cargo build --release --locked
./target/release/devinit --version
```

For local development, ensure `resources/services/` remains available alongside the project. On Windows the built executable is `target\release\devinit.exe`.

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

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo build --release --locked
```

The audited `0.0.4` development baseline passed **228 automated tests (201 unit and 27 integration)**. This is a tested snapshot, not a promise that the count will never change.

For bugs and proposals, see [GitHub Issues](https://github.com/GorkiyChocolate/DevInit-CLI/issues). The DevInit CLI is part of an Astana IT University diploma project focused on declarative infrastructure configuration management, versioning, distribution, and CLI-oriented workflows.

## License

**No open-source license has yet been confirmed or included in the reviewed repository.** Public visibility of source code does not itself grant permission to copy, modify, or redistribute it. A license must be selected and added before advertising DevInit as open-source or releasing it under specific reuse terms.
