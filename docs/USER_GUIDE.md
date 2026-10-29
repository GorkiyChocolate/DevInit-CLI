# DevInit CLI v0.0.2 — User Guide

> Documentation draft based on tested DevInit CLI behavior. Verify the installation steps against release assets before publishing.

DevInit is a CLI-oriented infrastructure configuration generator. It reads `compile.yaml`, validates the project, resolves supported service recipes and presets, and generates infrastructure configuration files.

## Quick start

1. Install a DevInit CLI binary appropriate for your operating system (release installers and archives are planned; do not assume they are published yet).
2. Create `compile.yaml` in a project directory.
3. Run `devinit check` to validate without writing generated files.
4. Run `devinit compile --dry-run` to preview files and conflicts.
5. Run `devinit compile` to generate files.

Example using built-in service recipes:

```yaml
services:
  - name: postgresql
    version: "17"
  - name: redis
    version: "7"
```

The generated Docker Compose file is `docker-compose.yaml`. The actual images and defaults are resolved from the installed registry recipes.

## Commands

| Command | Behavior |
| --- | --- |
| `devinit check` | Loads and validates the project and builds the generation plan without writing files. |
| `devinit compile --dry-run` | Previews `CREATE` or `CONFLICT` for output files; writes nothing. |
| `devinit compile --dry-run --force` | Previews `CREATE` or `OVERWRITE`; writes nothing. |
| `devinit compile` | Generates files; refuses to overwrite existing files. |
| `devinit compile --force` | Allows overwriting existing regular generated files (safety checks still apply). |
| `devinit add <service>` | Adds a recipe using built-in, cached, or configured remote sources; exact option flags should be checked with `--help`. |
| `devinit --help` | Shows installed CLI commands and options. |

The tested CLI currently looks for `compile.yaml` in the current working directory. `check` validates DevInit-supported fields, not live Docker/Kubernetes deployment health.

## Project configuration examples

### Docker Compose services with overrides

```yaml
services:
  - name: postgresql
    version: "17"
    ports:
      - "5433:5432"
  - name: redis
    version: "7"
    ports:
      - "6380:6379"
```

Host-port mappings are subject to DevInit validation. Basic supported forms include `8080:80`, `127.0.0.1:8080:80`, `8080:80/udp`, and container-only `80`. Advanced IPv6 and range syntax is not fully verified by DevInit's built-in port checker.

### Kubernetes manifest templates

```yaml
k8s:
  backend:
    template: deployment
    name: backend
    image: backend:v1
    replicas: 2
    port: 8080
  backend-service:
    template: service
    name: backend
    target: backend
    port: 80
    targetPort: 8080
```

This produces files under `k8s/`. Template kinds currently confirmed: `deployment`, `service`. For other Kubernetes kinds, use a raw Kubernetes resource document as a value under `k8s`:

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

### GitHub Actions

```yaml
cicd:
  provider: github
  triggers:
    - type: push
      branches: [main]
  jobs:
    - name: test
      steps:
        - name: Run tests
          action: run
          command: cargo test
```

Confirmed generated path: `.github/workflows/devinit.yml`. GitLab CI generation is also implemented, but check the provider's exact reference schema before publishing additional examples.

## Global presets and project overrides

The confirmed Linux global configuration file is `~/.config/devinit/devinit.yaml` (or `$XDG_CONFIG_HOME/devinit/devinit.yaml`). Windows behavior must be verified against the Windows build before documenting it as shipped.

Global configuration example:

```yaml
docker_compose:
  db:
    - name: postgresql
      version: "17"
      ports: ["5433:5432"]
    - name: redis
      version: "7"
      ports: ["6380:6379"]
k8s:
  backend:
    deployment:
      template: deployment
      name: backend
      image: backend:latest
      replicas: 3
      port: 8080
    service:
      template: service
      name: backend
      target: backend
      port: 80
      targetPort: 8080
cicd:
  rust-tests:
    provider: github
    triggers:
      - type: push
        branches: [main]
    jobs:
      - name: test
        steps:
          - name: Run tests
            action: run
            command: cargo test
```

Project `compile.yaml` consuming the presets:

```yaml
services:
  - use: db
    overrides:
      postgresql:
        ports: ["5435:5432"]
k8s:
  use:
    - name: backend
      overrides:
        deployment:
          image: backend:v4
          replicas: 4
        service:
          targetPort: 8080
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

**Important:** presets are user-specific. Another developer must have corresponding global presets, or you should use explicit definitions in `compile.yaml`. A proposed `devinit copy` command to export a fully resolved specification is **not implemented in v0.0.2**.

## Safety and validation

- Invalid configurations fail before writing generated files.
- Generated output paths are checked for duplicates and unsafe path traversal.
- Existing output files are protected by default; use `--force` only when replacement is intended.
- `--dry-run` does not modify files, including with `--force`.
- Output generation is **not** a multi-file transaction; an I/O failure during writing can leave some files written.
- Kubernetes raw manifest validation is intentionally partial, not a substitute for Kubernetes API/schema validation.

## Troubleshooting

- `Refusing to overwrite ...`: a generated destination already exists. Run `devinit compile --dry-run`, inspect changes, or explicitly use `--force`.
- Unknown preset: create the named global preset or replace it with explicit definitions.
- Port conflict: assign distinct published host ports or protocols.
- Invalid Kubernetes Deployment selector: ensure `spec.selector.matchLabels` matches the Pod template labels.
- CI/CD circular dependency: remove cyclic `needs` references.

## Publishing checklist

Before releasing this guide, validate `--help`, Windows paths, CLI binary naming, package names, and all example files against the tagged `v0.0.2` release.
