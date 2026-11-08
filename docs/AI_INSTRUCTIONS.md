# DevInit CLI — Instructions for AI Configuration Generators

**Target version:** DevInit `1.0.0-rc.2`.

**Schema:** [compile.schema.json](../schemas/compile.schema.json).

Generate DevInit `compile.yaml` files using only supported
syntax. Prefer the smallest valid configuration.

Validate generated YAML against the JSON Schema when
available, then run DevInit's own semantic validator:

```bash
devinit check -f compile.yaml --json
```

A schema-valid configuration may still be rejected by
the CLI if it references an unknown recipe, unsupported
service version, invalid dependency, or unavailable file.


### Additional generation rules

- Prefer built-in registry declarations over repeating
  Docker Compose defaults.
- Do not invent service-specific configuration files.
- When using `directories`, include all required files
  from the selected built-in recipe.
- Do not combine `directories` with `files` or `volumes`
  in the same registry declaration.
- Do not assume the JSON Schema can validate the
  existence of registry files or global presets.
- Prefer `check --json` to capture validation errors
  in automated workflows.
- Never use `--force` automatically without explicit
  user approval.
- Do not claim that generated Kubernetes manifests or
  CI pipelines have been deployed or executed.


## Task

Generate one **DevInit `compile.yaml`**, not raw Docker Compose or Kubernetes YAML, from a user's infrastructure requirements. Keep the configuration minimal, explicit when needed, and safe. Do not invent service names, versions, fields, commands, presets, or behavior.

## Core rules

1. Use only supported top-level keys: `services`, `k8s`, and `cicd`. Omit sections the user does not need. Unknown root fields are rejected.
2. Prefer packaged registry shorthand (`name` plus optional supported `version`) to duplicate defaults. Use an explicit custom service with `name` and `image` when appropriate. Do not combine `image` and `version` on one service.
3. For project Dockerfile discovery, use a **standalone list item** `- docker: true` within `services`. Do not write `docker_discovery: true` at top level. Enable discovery only when the user wants locally discovered applications included.
4. Built-in service names are `postgresql`, `mysql`, `mariadb`, `mongodb`, `redis`, `rabbitmq`, `kafka`, `elasticsearch`, `prometheus`, `grafana`, `s3`, and `ec2`. Do not assume arbitrary versions; the reviewed implementation confirms PostgreSQL `"17"` and Redis `"7"` among supported choices.
5. Use only confirmed Kubernetes templates `deployment` and `service`; for other resource kinds, use raw Kubernetes objects under `k8s` with valid API metadata, and state that external validation is needed.
6. CI/CD providers: `github`, `gitlab`. Triggers: `push`, `pull_request`, `manual`. Runners: `ubuntu-latest`, `windows-latest`, `macos-latest`. Steps: `checkout`, `run`, `docker_build`, `docker_push`. Use the field requirements in the Configuration Reference.
7. Never fabricate global presets. Use `use` only when the user's global `devinit.yaml` is known to contain a matching preset; otherwise generate a self-contained specification.
8. All `depends_on` services and CI `needs` jobs must exist; references must be acyclic. Avoid duplicate or conflicting published host ports.
9. Do not embed real passwords, API keys, private keys, or production tokens. Use environment-variable references only when known to be valid for the generated configuration.
10. Do not claim DevInit deploys services, builds Docker images when compiling, fully validates platform specifications, or makes multi-file writes atomic.
11. Prefer small, readable input with sensible defaults over unnecessary configuration. Explain unsupported requirements rather than silently inventing syntax.
12. If the user asks only for ready-to-use configuration, output YAML without commentary unless clarification of an unmet requirement is necessary.

## Verified patterns

### Registry-based Compose services

```yaml
services:
  - name: postgresql
    version: "17"
  - name: redis
    version: "7"
```

For PostgreSQL, downstream Docker Compose requires a `POSTGRES_PASSWORD` value; never embed actual credentials in YAML.

### Custom service and published port

```yaml
services:
  - name: backend
    image: nginx:alpine
    ports: ["8080:80"]
```

### Dockerfile discovery with a registry service

```yaml
services:
  - docker: true
  - name: redis
    version: "7"
```

Requires Dockerfiles in the project. DevInit generates build-context services but does not execute `docker build`.

### Kubernetes templates

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
        - name: Checkout
          action: checkout
        - name: Run tests
          action: run
          command: cargo test
```

Use `cargo test` only for an actual Rust project; substitute build/test commands based on the user's stack.

### GitLab CI/CD

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

### Preset (only if explicitly known to exist)

```yaml
services:
  - use: db
    overrides:
      postgresql:
        ports: ["5435:5432"]
```

This is not standalone: the global `docker_compose.db` preset must exist.

## Self-check before responding

- Does the YAML parse correctly and contain only documented root sections?
- Are all service names, versions, and action names supported?
- Does Docker discovery use `services: - docker: true` and correspond to the user's intent?
- Are dependencies declared, valid, and acyclic?
- Do all published ports avoid conflicts?
- Does every Kubernetes template use its supported fields?
- Are global presets guaranteed to exist if referenced?
- Is the content free of credentials and unsupported assumptions?
- Does the result avoid claiming anything about runtime deployment or guaranteed external compatibility?

## User handoff

Recommend the following workflow after producing a file:

```bash
devinit check
devinit compile --dry-run
devinit compile
```

Do not use `--force` by default; it explicitly permits overwriting generated files. For complex Kubernetes or CI/CD outputs, recommend external validation with the appropriate platform tools. `devinit login`, `devinit license`, and `devinit test` are not implemented in the audited baseline; `get` can depend on backend availability. Do not invent `devinit copy`, auto-deployment, or lockfile commands.

