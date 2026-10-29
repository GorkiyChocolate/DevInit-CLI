# DevInit v0.0.2 — Instructions for AI Configuration Generators

You are generating a **DevInit `compile.yaml`** file, **not** a Docker Compose file or Kubernetes manifest bundle. DevInit transforms this declarative input into Docker Compose, Kubernetes and CI/CD outputs.

## Primary objective

Given a user's description of an application's services and intended infrastructure, produce one valid `compile.yaml` that uses **only documented DevInit features**. Do not hallucinate keys, templates, available registry recipes or commands.

## Non-negotiable rules

1. Output YAML only when the user asks for a ready-to-use configuration; optionally add a concise explanation separately if requested.
2. Top-level sections currently confirmed: `services`, `k8s`, `cicd`. Include only sections required by the request.
3. A service may be a concrete definition (`name`, `image`, optional supported fields) or a registry recipe with `name` + `version`; do not specify `image` and `version` together.
4. Built-in recipe versions demonstrated and tested: `postgresql` version `"17"`, `redis` version `"7"`. Do not assume arbitrary service names or versions are installed.
5. Preset syntax `use` requires a matching preset in the user's **global** `devinit.yaml`. If preset availability is unknown, default to explicit definitions or ask for the preset names; do not fabricate a preset.
6. Kubernetes template names demonstrated: `deployment`, `service`. To describe other resource kinds, use a raw Kubernetes resource mapping under `k8s` with `apiVersion`, `kind`, `metadata.name` and the applicable structure.
7. CI/CD provider `github` with `push` trigger and `run` step is confirmed. Avoid inventing other provider-specific trigger/step variants without consulting `CONFIG_REFERENCE.md` or the project's Rust model types.
8. Use valid, non-conflicting ports in `1..=65535`; never duplicate published host ports on overlapping interfaces for the same protocol.
9. All referenced Compose dependencies and CI/CD `needs` jobs must exist. Dependencies must not form cycles.
10. Never include real credentials, access tokens, private keys or hardcoded production passwords. Use environment references only when their support is confirmed by the project schema.
11. Do not promise that a valid `compile.yaml` guarantees runtime deployment, schema compatibility or security.
12. When the user's request requires a feature not confirmed by DevInit documentation, state that limitation rather than inventing a configuration field.

## Verified examples

### Explicit Docker Compose recipe services

```yaml
services:
  - name: postgresql
    version: "17"
    ports: ["5433:5432"]
  - name: redis
    version: "7"
    ports: ["6380:6379"]
```

### Explicit custom service

```yaml
services:
  - name: backend
    image: backend:latest
    ports: ["8080:8080"]
```

### Kubernetes Deployment and Service templates

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

### GitHub Actions workflow

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

### Presets when names and definitions are known

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
          replicas: 4
cicd:
  use: rust-tests
```

This final example depends on a global `db`, `backend` and `rust-tests` preset; **do not select it as a standalone configuration** when global presets are unknown.

## Generation checklist (perform before answering)

- Is the YAML syntactically correct?
- Are all keys confirmed by the reference or project source?
- Do service names and CI/CD job names refer to existing declarations?
- Are port values, dependency graphs and resource identifiers internally consistent?
- Does every template use exactly its documented keys?
- Is the configuration independent of user-global presets unless explicitly requested?
- Is there any sensitive data embedded in the YAML?

## Suggested user handoff

After giving a config, tell the user to run, in this order:

```text
devinit check
devinit compile --dry-run
devinit compile
```

DevInit refuses to overwrite generated files by default. `--force` is opt-in. `devinit copy` and lockfile-based reproduction are future features, not part of the confirmed v0.0.2 CLI.

## Important authority rule

This is a **conservative generative guide**, not a complete formal schema. When a complete field/variant reference becomes available, treat `CONFIG_REFERENCE.md` and the exact Rust models/validators for the tagged release as authoritative. Never infer unlisted YAML keys from Docker Compose/Kubernetes standards.
