# DevInit CLI

**Define once. Validate safely. Generate infrastructure configurations effortlessly.**

DevInit is an open-source, Rust-based command-line tool designed to simplify infrastructure configuration management through a declarative approach.

Instead of manually maintaining multiple configuration formats, DevInit allows developers to describe their infrastructure requirements in a single `compile.yaml` file and generate configurations for **Docker Compose, Kubernetes, GitHub Actions, and GitLab CI/CD**.

DevInit focuses on predictable generation, reusable infrastructure components, offline-first functionality, and protecting existing project files.

> **Project Status:** Active development  
> **Current Version:** v0.0.2  
> **Platform Support:** Linux and Windows (x64)

---

## Features

### Declarative Configuration

Define infrastructure requirements using a single YAML configuration file.

DevInit handles validation, dependency resolution, and generation of supported infrastructure resources.

### Multi-Platform Generation

Generate configuration files for:

- **Docker Compose** — containerized services and their dependencies.
- **Kubernetes** — Deployments, Services, and supported custom manifests.
- **GitHub Actions** — automated workflows.
- **GitLab CI/CD** — pipeline configuration.

### Built-in Service Registry

DevInit provides reusable service recipes for common infrastructure components, including:

- PostgreSQL
- MySQL
- MariaDB
- MongoDB
- Redis
- RabbitMQ
- Apache Kafka
- Elasticsearch
- Prometheus
- Grafana

Supported versions and available recipes depend on the installed DevInit version.

### Reusable Presets

Create reusable configuration presets for Docker Compose, Kubernetes, and CI/CD workflows.

Presets can be stored in the global DevInit configuration and customized using project-level overrides.

### Configuration Validation

DevInit validates supported configuration rules before generating files, including:

- Service declarations and required fields.
- Dependency references and circular dependencies.
- Docker Compose port mappings and selected port conflicts.
- Kubernetes manifest structure.
- CI/CD job dependencies and dependency cycles.
- Output paths and existing-file conflicts.

Validation does not replace full validation against Docker, Kubernetes, or CI/CD platform specifications.

### Safe File Generation

DevInit is designed to preserve existing project files.

- Existing files are not overwritten by default.
- `--force` explicitly enables overwriting.
- `check` validates without generating files.
- `--dry-run` previews planned operations.
- Generated output paths are checked for unsafe filesystem operations.

### Offline-First Operation

Built-in recipes, locally cached resources, and local generation workflows can operate without an active connection to the DevInit backend.

Remote configuration distribution and account-based functionality are being developed separately.

---

## Installation

### Windows

Download the Windows installer from the repository's [Releases](https://github.com/GorkiyChocolate/DevInit-CLI/releases) page when a release is available.

Run:

`DevInit-Setup-0.0.2.exe`

Follow the installation wizard, then reopen PowerShell and verify the installation:

```powershell
devinit --version
devinit --help
```

For a portable installation, download the Windows ZIP archive, extract `devinit.exe`, and run it directly or add its directory to your `PATH`.

### Linux

Download the Linux x64 archive from the Releases page when available, extract it, and install the executable:

```bash
tar -xzf devinit-linux-x64.tar.gz
chmod +x devinit
sudo install -m 755 devinit /usr/local/bin/devinit
```

Verify:

```bash
devinit --version
devinit --help
```

### Build from Source

**Requirements:**

- Rust toolchain with support for edition 2024.
- Cargo.
- Git.

Clone the repository:

```bash
git clone https://github.com/GorkiyChocolate/DevInit-CLI.git
cd DevInit-CLI
```

Build the executable:

```bash
cargo build --release
```

The executable will be available at:

```text
target/release/devinit
```

On Windows:

```text
target\release\devinit.exe
```

Docker and Kubernetes are not required simply to generate configuration files.

---

## Quick Start

### 1. Create a configuration file

Create `compile.yaml` in your project directory:

```yaml
services:
  - name: backend
    image: backend:latest
    ports:
      - "8080:8080"

  - name: postgresql
    version: "17"

  - name: redis
    version: "7"
```

This example describes a backend application, PostgreSQL, and Redis.

### 2. Validate the configuration

```bash
devinit check
```

DevInit validates the configuration and prepares the generation plan without creating output files.

### 3. Preview generation

```bash
devinit compile --dry-run
```

Review the planned files and any output conflicts.

### 4. Generate configurations

```bash
devinit compile
```

The generated files depend on the sections declared in `compile.yaml`.

For the example above, DevInit generates:

```text
my-project/
├── compile.yaml
└── docker-compose.yaml
```

If generated output already exists, DevInit refuses to overwrite it by default.

To explicitly allow overwriting:

```bash
devinit compile --force
```

**Warning:** `--force` can replace existing configuration content. Review changes before using it.

---

## Kubernetes Configuration

DevInit supports Kubernetes configuration generation using simplified templates and raw manifests.

Example:

```yaml
k8s:
  backend:
    apiVersion: apps/v1
    kind: Deployment
    metadata:
      name: backend
    spec:
      replicas: 2
      selector:
        matchLabels:
          app: backend
      template:
        metadata:
          labels:
            app: backend
        spec:
          containers:
            - name: backend
              image: backend:latest
              ports:
                - containerPort: 8080

  backend-service:
    apiVersion: v1
    kind: Service
    metadata:
      name: backend
    spec:
      selector:
        app: backend
      ports:
        - port: 80
          targetPort: 8080
```

Expected generated files:

```text
k8s/
├── backend.yaml
└── backend-service.yaml
```

DevInit performs supported structural checks before generation.

---

## CLI Commands

| Command | Description |
|---|---|
| `devinit --help` | Display available commands and options. |
| `devinit --version` | Display the installed version. |
| `devinit check` | Validate configuration without writing files. |
| `devinit compile` | Generate infrastructure configuration files. |
| `devinit compile --dry-run` | Preview generation without modifying files. |
| `devinit compile --force` | Explicitly allow overwriting existing output files. |
| `devinit add` | Access the service recipe addition workflow. |
| `devinit list` | List services through the available CLI interface. |
| `devinit get` | Access the remote service retrieval interface. |
| `devinit login` | Authentication command; functionality is under development. |

Some commands are experimental or require backend integration. Use `devinit <command> --help` to inspect the currently supported arguments.

---

## How DevInit Works

DevInit uses a structured configuration processing pipeline:

```text
compile.yaml
     |
     v
Configuration Parsing
     |
     v
Preset & Service Resolution
     |
     v
Configuration Validation
     |
     v
Intermediate Representation (IR)
     |
     v
Generation Plan
     |
     v
Filesystem Safety Checks
     |
     v
Generated Infrastructure Files
```

This architecture separates configuration resolution, validation, and generation.

The generation plan allows DevInit to inspect intended output operations before writing files.

---

## Project Structure

```text
DevInit-CLI/
├── src/
│   ├── api/              # API integration
│   ├── cli/              # CLI commands and execution logic
│   ├── config/           # Global presets
│   ├── core/             # IR, resolver, service registry
│   ├── docker/           # Docker-related utilities
│   ├── file_config/      # Configuration parsing and file handling
│   ├── generation/       # Generation plans and Kubernetes manifests
│   ├── generator/        # Infrastructure generators
│   ├── models/           # Configuration data models
│   ├── resolved_spec/    # Specification resolution
│   ├── validator/        # Configuration validation
│   └── tests/            # Unit tests
├── tests/                # Integration tests
├── docs/                 # Technical and user documentation
├── Cargo.toml
├── Cargo.lock
└── README.md
```

---

## Documentation

The repository includes documentation for both human developers and AI-assisted configuration generation.

| Document | Purpose |
|---|---|
| `docs/USER_GUIDE.md` | Installation, usage, and configuration examples. |
| `docs/CONFIG_REFERENCE.md` | Supported configuration structures and parameters. |
| `docs/AI_INSTRUCTIONS.md` | Instructions for AI models generating DevInit configurations. |
| `docs/PROJECT_DOCUMENTATION.md` | Additional project documentation. |

### AI-Assisted Configuration Generation

DevInit is intended to work with AI-assisted development workflows.

Developers can provide `docs/AI_INSTRUCTIONS.md` and `docs/CONFIG_REFERENCE.md` to an AI coding assistant and request a DevInit configuration.

For example:

> Generate a DevInit `compile.yaml` for a Go backend using PostgreSQL 17 and Redis 7, with Docker Compose and Kubernetes configurations.

The resulting YAML should be validated using:

```bash
devinit check
```

AI-generated configurations must be reviewed before use.

---

## Current Limitations

DevInit is under active development.

- Dockerfile discovery is not yet integrated into Docker Compose build generation.
- Configuration validation covers supported rules rather than complete external platform schemas.
- The CLI does not automatically execute deployments or build Docker images.
- Generation of multiple files is not guaranteed to be fully atomic.
- Authentication and remote configuration features are still evolving.
- Built-in service versions and preset availability are limited to supported definitions.

---

## Roadmap

Planned improvements include:

- **Dockerfile Discovery Integration:** automatically associate discovered Dockerfiles with Compose services.
- **Advanced File Protection:** strengthen safe writes, recovery, and configuration integrity.
- **CLI Authentication:** complete secure authentication and session management through `devinit login`.
- **Expanded Service Registry:** add more built-in service recipes, versions, and reusable presets.
- **Configuration Schema:** provide a machine-readable JSON Schema for `compile.yaml`.
- **Configuration Versioning and Distribution:** improve sharing and management of reusable infrastructure configurations.
- **Cross-Platform Improvements:** expand compatibility and installation support.

These features are planned and should not be considered part of the current release until implemented.

---

## Development

Run the following checks before submitting changes:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

The v0.0.2 development baseline includes 137 automated tests: 121 unit tests and 16 integration tests.

Contributions should preserve existing behavior and include tests for new functionality.

---

## Contributing

Contributions, bug reports, and feature requests are welcome.

To contribute:

1. Fork the repository.
2. Create a dedicated feature or bug-fix branch.
3. Implement your changes with appropriate tests.
4. Run the project validation commands.
5. Open a Pull Request describing your changes.

For feature requests and bug reports, visit [GitHub Issues](https://github.com/GorkiyChocolate/DevInit-CLI/issues).

## License

License information will be provided in the repository. Until then, no open-source license or reuse permissions should be assumed.

## Project Background

DevInit CLI is part of the **DevInit** diploma project developed at Astana IT University.

The academic project explores declarative infrastructure configuration generation, versioning, distribution, and CLI-oriented developer workflows.

The CLI is implemented in Rust and is designed to function independently of the project's optional web platform.

---

**DevInit — Define infrastructure once. Generate with confidence.**
