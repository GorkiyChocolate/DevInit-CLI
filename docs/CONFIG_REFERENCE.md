# DevInit CLI v0.0.2 — Configuration Reference (Verified Subset)

> **Status: draft.** This document lists confirmed syntax from code excerpts and successful CLI runs. It is not a complete schema. Complete it against `cli/src/models/*`, `cli/src/validator/*`, preset expansion and generator code before tagging a release.

## `compile.yaml` top-level document

| Key | Kind | Purpose |
| --- | --- | --- |
| `services` | sequence | Concrete services, registry recipes, or global Docker Compose preset references |
| `k8s` | mapping | Raw Kubernetes resources, short templates or Kubernetes preset references |
| `cicd` | mapping | CI/CD pipeline definition or preset reference |

All three are optional individually in observed tests.

### Services

Confirmed registry recipe syntax:

```yaml
services:
  - name: postgresql
    version: "17"
  - name: redis
    version: "7"
```

Confirmed custom service syntax:

```yaml
services:
  - name: backend
    image: backend:latest
    ports: ["8080:8080"]
```

Do not combine `image` and `version` on the same service. `depends_on` is a list of other declared service names; duplicate, unknown, self and cyclic dependencies are rejected. Basic published host-port collisions are rejected.

Confirmed preset reference syntax:

```yaml
services:
  - use: db
    overrides:
      postgresql:
        ports: ["5435:5432"]
```

The matching global config section is `docker_compose.db`, a list of service definitions. `overrides` may reference only services defined by that preset; a service cannot be renamed through overrides.

### Kubernetes short templates

```yaml
k8s:
  backend:
    template: deployment
    name: backend
    image: backend:latest
    replicas: 3
    port: 8080
  backend-service:
    template: service
    name: backend
    target: backend
    port: 80
    targetPort: 8080
    type: ClusterIP
```

Confirmed allowed `deployment` template fields: `template`, `name`, `image`, `replicas`, `port`. Replicas: 0–10000. Port: 1–65535; default 8080. `service` template fields: `template`, `name`, `target`, `port`, `targetPort`, `type`. Types confirmed by template implementation: `ClusterIP`, `NodePort`, `LoadBalancer`. Ports 1–65535.

Raw Kubernetes objects are supported as mappings with `apiVersion`, `kind`, and `metadata.name`. Deployment and Service raw objects receive additional structural checks; other kinds remain mostly opaque except required metadata and duplicate identity checks. Do not interpret successful DevInit validation as validation against a live Kubernetes API server.

Confirmed Kubernetes preset reference:

```yaml
k8s:
  use:
    - name: backend
      overrides:
        deployment:
          replicas: 5
        service:
          targetPort: 3000
```

`k8s.use: [backend]` is also supported. The preset must exist in the global `k8s` mapping. Overrides apply to named manifests in that preset.

### CI/CD

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

`cicd.use: rust-tests` (and `cicd.overrides`) refers to a named pipeline in the global `cicd` mapping:

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

CI/CD jobs may declare `needs` references; unknown and cyclic references are rejected. Note: the complete provider/runner/trigger/step enum set has **not yet been extracted** from the Rust models, so this reference does not claim to enumerate all possible variants.

### Global configuration

Verified Linux path: `~/.config/devinit/devinit.yaml` or `$XDG_CONFIG_HOME/devinit/devinit.yaml`. Expected top-level sections from tested presets: `default_url`, `docker_compose`, `k8s`, `cicd`. Windows path resolution should be documented only after Windows build verification.

### Output paths

| Source | Confirmed output |
| --- | --- |
| `services` | `docker-compose.yaml` |
| Raw and templated `k8s.<key>` | `k8s/<key>.yaml` |
| `cicd` with GitHub provider | `.github/workflows/devinit.yml` |
| GitLab CI | `.gitlab-ci.yml` |

### Validation boundaries

DevInit validates supported declaration rules, registry versions, dependencies, selected port formats, Kubernetes template structure, and output path conflicts. It does not fully validate Docker Compose or Kubernetes schemas. Existing files are not overwritten without `--force`; no multi-file transactional rollback is promised.

## Fields to extract from source before publishing as complete reference

- All supported `CompileService`/`RecipeCompose` fields, types, defaults and constraints.
- All supported CI providers, triggers, runners and step actions.
- Legacy Kubernetes deployment model fields and generators.
- Preset resolution precedence and override merge semantics.
- Exact global config path logic on Windows.
- CLI flags and positional arguments for `add`, `get`, `list`, `login`.

A robust approach for future AI tools is a **versioned machine-readable JSON Schema for `compile.yaml`** generated or maintained alongside this reference. Do not claim one exists until implemented and tested.
