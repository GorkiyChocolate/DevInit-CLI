# DevInit CLI — Configuration Reference

**Version:** `1.0.0-rc.2` development snapshot.

**Input:** `compile.yaml` or an alternative file selected
with `-f` / `--file`.

**Machine-readable schema:**
[compile.schema.json](../schemas/compile.schema.json)

DevInit validates infrastructure declarations and generates
configuration files. It does not deploy infrastructure.

The JSON Schema provides structural checks and editor
assistance. Semantic validation, service registry resolution,
dependency checks, and generated-file conflict detection
are performed by the DevInit CLI.

## 1. Top-level structure

| Key | Type | Purpose |
| --- | --- | --- |
| `services` | sequence | Custom services, registry references, Dockerfile discovery, or Compose presets |
| `k8s` | mapping | Kubernetes templates, raw manifests, or presets |
| `cicd` | mapping | GitHub Actions or GitLab CI/CD pipeline, or preset |

The keys are individually optional. **Unknown top-level keys are rejected** in the reviewed implementation. `docker_discovery: true` at top level is invalid; use the service-list declaration below.

```yaml
services:
  - name: redis
    version: "7"
k8s:
  redis-helper:
    template: deployment
    name: redis-helper
    image: nginx:alpine
    replicas: 1
    port: 80
cicd:
  provider: github
  jobs:
    - name: validate
      steps:
        - name: Run checks
          action: run
          command: cargo test
```

This is a syntax example, not a recommended deployment architecture.

## 2. Services

### 2.1 Built-in registry shorthand

```yaml
services:
  - name: postgresql
    version: "17"
  - name: redis
    version: "7"
```

The `version` field is interpreted by the registry resolver; the resolved service is converted to a full recipe before generation. Omitting `version` selects the packaged default. `postgresql` currently defaults to `16`, while supported versions include `15`, `16`, and `17` in the audited registry. Other versions must be checked with the packaged service definitions; arbitrary versions are not guaranteed to work.

Do not combine `image` with `version` for the same service. Other supported fields may be used as registry overrides if accepted by the resolver and validators.

Packaged service names are `postgresql`, `mysql`, `mariadb`, `mongodb`, `redis`, `rabbitmq`, `kafka`, `elasticsearch`, `prometheus`, `grafana`, `s3` (MinIO), and `ec2` (LocalStack). `devinit list` displays defaults for the installed version.

### 2.2 Explicit custom services

```yaml
services:
  - name: backend
    description: Example web service
    image: nginx:alpine
    ports: ["8080:80"]
    environment:
      - APP_ENV=development
    depends_on: [redis]
    restart: unless-stopped
  - name: redis
    version: "7"
```

The resolved `RecipeCompose` Rust model supports:

| Field | Value type | Notes |
| --- | --- | --- |
| `name` | string | Required service name; validated for safe naming |
| `description` | optional string | Descriptive metadata; may become a Compose label |
| `image` | string | Container image in an explicit service recipe |
| `ports` | optional list of strings | Published/container port mappings |
| `environment` | optional list of strings | Compose environment entries |
| `volumes` | optional list of strings | Bind and named mounts |
| `networks` | optional list of strings | Network attachment names |
| `depends_on` | optional list of strings | Other declared service names |
| `restart` | optional string | Restart policy |
| `command` | optional list of strings | Container command and arguments |
| `files` | optional file object or list | Additional generated files; see below |
| `env` | optional list of strings | Environment-file-related recipe entries |
| `notes` | optional list of strings | Human-readable notes |

**Note:** This table describes the resolved recipe representation. Public shorthand inputs additionally support `version` and preset fields before resolution; not all properties are valid for every shorthand or override context. Unsupported combinations are rejected by the resolver or validators.

A `files` entry contains `path` and `content`:

```yaml
services:
  - name: backend
    image: nginx:alpine
    files:
      - path: config/app.yaml
        content: |
          mode: development
```

The source supports either a single `{path, content}` object or a list of objects; generated paths are subject to safety checks. Some bundled recipes generate service-specific directories and files (for example Prometheus and Grafana configuration).

### 2.3 Dockerfile discovery

```yaml
services:
  - docker: true
  - name: redis
    version: "7"
```

The discovery directive must contain **only** `docker: true`, and may occur **once**. It searches for supported project Dockerfiles and adds Compose build services with an appropriate `build.context` and `build.dockerfile`. It fails if explicitly requested but no Dockerfiles are found, or if a discovered name conflicts with a declared service. Generated Compose files are not automatically built or run.

### 2.4 Dependencies, ports, and versions

- Referenced dependencies must exist. Self-dependencies, duplicates, and cycles are rejected.
- Supported port examples include `8080:80`, `127.0.0.1:8080:80`, `8080:80/udp`, and container-only `80`.
- Published host port conflicts are checked for overlapping host interfaces and matching protocols. Advanced IPv6 and port-range validation is not complete.
- Service versions are limited to packaged or otherwise configured and validated definitions.


## 3. Kubernetes (`k8s`)

### Kubernetes configuration compatibility

DevInit supports two Kubernetes configuration formats:

- `kubernetes`: the original structured Deployment format.
  It generates a Deployment and, when applicable, a Service.
- `k8s`: the recommended format for new projects. It supports
  Deployment and Service templates, raw Kubernetes manifests,
  and reusable presets.

Both formats remain supported. They can be used together,
provided they do not generate conflicting output paths.

The `kubernetes` field is retained for backward compatibility
and is not formally deprecated in `1.0.0-rc.3`.

Neither format provides complete Kubernetes API validation.
Validate generated manifests with appropriate Kubernetes
tools before deploying to production.

### 3.1 Deployment and Service templates

### 3.1 Deployment and Service templates

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
    type: ClusterIP
```

Confirmed deployment template fields: `template`, `name`, `image`, `replicas`, `port`. The verified implementation accepts replicas in `0..=10000`; port `1..=65535`, with a template default of `8080`.

Confirmed Service template fields: `template`, `name`, `target`, `port`, `targetPort`, `type`. Supported `type` values are `ClusterIP`, `NodePort`, and `LoadBalancer`; ports must be in `1..=65535`.

Output files follow `k8s/<entry-key>.yaml`; in this example, `k8s/backend.yaml` and `k8s/backend-service.yaml`.

### 3.2 Raw manifests

```yaml
k8s:
  app-settings:
    apiVersion: v1
    kind: ConfigMap
    metadata:
      name: app-settings
    data:
      MODE: development
```

Raw mappings require basic Kubernetes identity fields (`apiVersion`, `kind`, `metadata.name`). Selected Deployment and Service structures receive extra checks. Other resource kinds are not exhaustively validated against the Kubernetes API; use `kubectl` or a cluster-side schema validator for production use.

### 3.3 Kubernetes presets

```yaml
k8s:
  use:
    - name: backend
      overrides:
        deployment:
          replicas: 4
```

`k8s.use: [backend]` is also supported. Referenced presets must exist in global `devinit.yaml`; overrides may only target known manifests in the named group. The expanded output names include the preset and manifest names.

### Service-specific configuration directories

Built-in service recipes may provide configuration files
that can be relocated within the project.

Example:

```yaml
services:
  - name: prometheus
    directories:
      - dir: monitoring
        files:
          - prometheus.yml
          - alerts.yml

  - name: grafana
    directories:
      - dir: observability/grafana
        files:
          - default.yml
```

DevInit reuses the built-in file contents and adjusts
the associated mounts where applicable.

The directory must be a safe relative path.
File selections must reference existing recipe files.
Required recipe files cannot be omitted.

Do not combine `directories` with `files` or `volumes`
on the same registry service declaration.

Run `devinit check` before generating output.

## 4. CI/CD (`cicd`)

### 4.1 GitHub Actions

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

Produces `.github/workflows/devinit.yml`.

### 4.2 GitLab CI/CD

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

Produces `.gitlab-ci.yml`.

### 4.3 Confirmed values

| Property | Supported values |
| --- | --- |
| `provider` | `github`, `gitlab` |
| Trigger `type` | `push`, `pull_request`, `manual` |
| Trigger `branches` | Optional list of strings for `push` / `pull_request` |
| Job `runner` | `ubuntu-latest` (default), `windows-latest`, `macos-latest` |
| Step `action` | `checkout`, `run`, `docker_build`, `docker_push` |

A job has `name`, optional `runner`, optional `needs` (list of other job names), and `steps` (ordered list). A step has `name`, `action`, and optional `env` (string-to-string mapping). `run` requires `command`; `docker_build` requires `dockerfile` and `image`; `docker_push` requires `image`.

DevInit checks supported dependency constraints, including missing references and cycles. Successful rendering does not guarantee the remote CI pipeline will execute successfully.

### 4.4 CI/CD preset

```yaml
cicd:
  use: rust-tests
  overrides:
    jobs:
      - name: test
        steps:
          - name: Run release tests
            action: run
            command: cargo test --release
```

`rust-tests` must be defined in the global `cicd` preset mapping. The override merge process recursively merges mappings and replaces sequences; it does not append list items.

## 5. Global presets and backend URL

Global `devinit.yaml` paths:

| System | Path |
| --- | --- |
| Linux | `~/.config/devinit/devinit.yaml`, or `$XDG_CONFIG_HOME/devinit/devinit.yaml` |
| Windows | `%APPDATA%\DevInit\devinit.yaml` |

Example global presets:

```yaml
default_url: http://localhost:5167
docker_compose:
  db:
    - name: postgresql
      version: "17"
    - name: redis
      version: "7"
k8s:
  backend:
    deployment:
      template: deployment
      name: backend
      image: nginx:alpine
      replicas: 2
      port: 80
cicd:
  rust-tests:
    provider: github
    jobs:
      - name: test
        steps:
          - name: Run tests
            action: run
            command: cargo test
```

Use a Compose preset from your project:

```yaml
services:
  - use: db
    overrides:
      postgresql:
        ports: ["5435:5432"]
```

A Compose preset may be referenced using `use` and optional `overrides`; only services defined within the preset may be overridden, and their `name` or `use` identities may not be changed. Nested preset references are not supported. Presets are user-specific: do not assume they exist on another machine.

The backend URL is selected in priority order: `DEVINIT_API_URL`, global `default_url`, or `http://localhost:5167`. Local generation does not require a remote backend. The URL affects features that consult the backend, not Docker/Kubernetes deployments.

## 6. Output paths and file safety

| Configuration | Output |
| --- | --- |
| `services` / Docker discovery | `docker-compose.yaml` |
| `k8s.<name>` | `k8s/<name>.yaml` |
| `cicd.provider: github` | `.github/workflows/devinit.yml` |
| `cicd.provider: gitlab` | `.gitlab-ci.yml` |
| Recipe `files` | Declared relative file paths |

`devinit check` validates and computes a plan but writes no generated output. `devinit compile --dry-run` previews CREATE/CONFLICT, and `--dry-run --force` previews OVERWRITE without writing. `devinit compile` refuses to overwrite existing files; `--force` is explicit and is still subject to path/symlink safety restrictions. Output generation is not promised to be transactional across multiple files.

## 7. Registry and downloads

- `devinit list` lists 12 bundled recipes and their default versions.
- `devinit add <service> [--version <version>]` caches selected recipes; it does not edit `compile.yaml` automatically.
- `devinit download <name>` accepts a restricted set of approved Docker image names (`redis`, `postgresql`, `kafka`, `prometheus`, `grafana`) or Linux CLI tools (`kubectl`, `helm`).
- Binary tools are stored in `~/.local/share/devinit/tools/`, verified with SHA-256 and do not automatically change `PATH`.
- Downloaded CLI tools currently support Linux x86-64 and ARM64; image downloads require Docker Engine.

## 8. Scope and compatibility notes

This reference documents confirmed user-facing behavior from the audited Rust models, CLI tests, and manual integration checks. It is not a machine-readable schema and does not enumerate all private resolver internals or every possible value accepted by upstream Docker, Kubernetes, or CI systems.

`devinit get` is experimental and backend-dependent in some situations. `login`, `license`, and `test` are registered but not yet implemented and return errors. There is no supported `devinit copy` command or guaranteed lockfile-based reproduction in this reviewed version. Always test generated configuration files with their intended external tools before production use.
