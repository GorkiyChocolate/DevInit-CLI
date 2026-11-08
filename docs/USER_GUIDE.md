# DevInit CLI — User Guide

**Version context:** `1.0.0-rc.2` development snapshot. Release packaging and installation are undergoing validation.

DevInit is a CLI-first generator for infrastructure configuration files. It reads `compile.yaml`, resolves built-in recipes and optional presets, checks supported rules, then writes Docker Compose, Kubernetes, and CI/CD configuration files. It does not deploy infrastructure.


## 1. Install

DevInit `1.0.0-rc.2` provides Linux and Windows
release packaging. Automated installers are currently
undergoing pre-release validation.

For installation instructions and platform limitations,
see [Installation](../README.md#installation).

After installation, verify:

```bash
devinit --version
devinit list
```

The 12 built-in service recipes are embedded in the
executable. Copying external registry YAML files is
not required for normal CLI usage.

### Working with another configuration file

The default input is `compile.yaml`, but another
file can be supplied explicitly:

```bash
devinit check --file config/dev.yaml
devinit compile -f config/dev.yaml --dry-run
devinit compile -f config/dev.yaml
```

### Using JSON output

```bash
devinit list --json
devinit check --json
devinit compile --dry-run --json
```

Use JSON output for automated validation and integrations.
A failed validation returns a non-zero exit code.

### Safe generation

```bash
devinit check
devinit compile --dry-run
devinit compile
```

DevInit does not overwrite existing output files by
default. Use `--force` only when replacement is intended.


## 2. First project

Create a `compile.yaml`:

```yaml
services:
  - name: backend
    image: nginx:alpine
    ports: ["8080:80"]
  - name: redis
    version: "7"
```

From the same folder:

```bash
devinit check
devinit compile --dry-run
devinit compile
```

The generated `docker-compose.yaml` contains the declared services and their resolved settings. Running `devinit compile` again will refuse to overwrite it. To preview changes, use `devinit compile --dry-run --force`; overwrite only deliberately with `devinit compile --force`.

### PostgreSQL credentials

```yaml
services:
  - name: postgresql
    version: "17"
```

The packaged recipe expects `POSTGRES_PASSWORD` when Compose resolves the output. Provide it via your local environment or a properly protected secrets workflow; never commit live credentials.

## 3. Existing project: Dockerfile discovery

Given:

```text
my-project/
├── backend/Dockerfile
├── frontend/Dockerfile
└── compile.yaml
```

Use:

```yaml
services:
  - docker: true
  - name: redis
    version: "7"
```

Running `devinit compile` creates Compose services for `backend`, `frontend`, and `redis`, including relative build contexts. The directive must appear as its own list item and may appear only once. It is not a top-level `docker_discovery` key. DevInit discovers Dockerfiles but does not build or execute their containers.

## 4. Kubernetes templates

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
```

This generates `k8s/backend.yaml` and `k8s/backend-service.yaml`. For other kinds, provide raw Kubernetes mappings with `apiVersion`, `kind`, and `metadata.name`. DevInit checks supported structural rules; it does not replace validation against a real cluster or Kubernetes schema.

## 5. CI/CD generation

GitHub Actions:

```yaml
cicd:
  provider: github
  triggers:
    - type: push
      branches: [main]
  jobs:
    - name: test
      runner: ubuntu-latest
      steps:
        - name: Checkout
          action: checkout
        - name: Run tests
          action: run
          command: cargo test
```

GitLab CI/CD:

```yaml
cicd:
  provider: gitlab
  jobs:
    - name: build
      steps:
        - name: Build
          action: run
          command: cargo build
    - name: test
      needs: [build]
      steps:
        - name: Test
          action: run
          command: cargo test
```

Generated files are `.github/workflows/devinit.yml` or `.gitlab-ci.yml`. Example commands above assume a Rust project; replace them for other stacks. Supported providers, triggers, runners, and step types are listed in the [Configuration Reference](CONFIG_REFERENCE.md).

## 6. Built-in registry and adding recipes

```bash
devinit list
devinit add redis
devinit add postgresql --version 17
```

There are 12 bundled service definitions, including relational databases, Redis, Kafka, Elasticsearch, Prometheus, Grafana, RabbitMQ, MinIO (`s3`), and LocalStack (`ec2`). `add` saves a recipe to DevInit's local cache. It does not edit your `compile.yaml` or start a container. Unsupported versions and unsafe service names are rejected.

## 7. Approved downloads

```bash
devinit download redis
devinit download kubectl
devinit download helm
```

Image downloads are restricted to approved Docker images (Redis, PostgreSQL, Kafka, Prometheus, Grafana) and require Docker Engine. `kubectl` and Helm binary downloads are supported on Linux x86-64/ARM64. DevInit verifies SHA-256 and saves the tools to `~/.local/share/devinit/tools/`; it does not install them system-wide or add them to `PATH`. Network access is required for fresh downloads.

## 8. Global presets

Global configuration locations:

- Linux: `~/.config/devinit/devinit.yaml` or `$XDG_CONFIG_HOME/devinit/devinit.yaml`.
- Windows: `%APPDATA%\DevInit\devinit.yaml`.

Example:

```yaml
docker_compose:
  db:
    - name: postgresql
      version: "17"
    - name: redis
      version: "7"
```

The project can reference it:

```yaml
services:
  - use: db
    overrides:
      postgresql:
        ports: ["5435:5432"]
```

The preset must already be defined locally. Presets can also be used for Kubernetes and CI/CD. Overrides cannot rename preset services or target nonexistent definitions; nested preset references are unsupported.

## 9. Troubleshooting

| Symptom | What to do |
| --- | --- |
| `Refusing to overwrite` or output `CONFLICT` | Review `devinit compile --dry-run`; only use `--force` when replacement is intended. |
| `Service directory not found` | Restore `resources/services` alongside the executable, use a supported system layout, or configure `DEVINIT_SERVICES_DIR`. |
| `unknown field docker_discovery` | Use `services:\n  - docker: true` instead of a top-level key. |
| Unsupported service version | Check available bundled recipes; do not assume any image tag is accepted. |
| Port conflict | Assign distinct published ports or use compatible protocol/bind interfaces. |
| Unknown preset | Create the named global preset or replace it with explicit declarations. |
| Missing dependency or cycle | Check `depends_on` / CI `needs` names and break cycles. |
| `login`, `license`, or `test` returns an error | These commands are reserved and not implemented in the reviewed version. |

## 10. Reliability boundaries

DevInit is **safe by default**: `check` and `dry-run` do not write generated output, existing files are protected, and file paths are validated. However, multi-file generation is not guaranteed to roll back all writes after an I/O failure. Validate output with Docker Compose, Kubernetes, or CI-specific validators before use in production. DevInit does not execute deployment operations during compilation.

For field-level details, refer to [Configuration Reference](CONFIG_REFERENCE.md). For AI-assisted YAML authoring, see [AI Instructions](AI_INSTRUCTIONS.md).
