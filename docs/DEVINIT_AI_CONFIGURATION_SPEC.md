# DevInit `compile.yaml` — AI Generation Contract

**Document purpose:** deterministic instructions for a local language model generating valid configuration files for the DevInit CLI. This is a **configuration-generation contract**, not an invitation to invent a new schema.  
**Target:** DevInit CLI as tested in the v0.0.2 development line (v0.0.3 candidate), October 2026.  
**Evidence:** demonstrated CLI behavior, known Rust data models/registry definitions and the 30 examples in `generated.zip`. Where a feature has not been verified, this document explicitly says so.

> **Golden rule:** Output YAML that DevInit actually accepts. Never substitute a familiar Docker Compose / Kubernetes / Terraform format for the DevInit input format.

## 1. Mandatory rules for an AI generator

1. Generate **one YAML document** for `compile.yaml`, not Docker Compose YAML, not an explanation and not Markdown. For inference-only responses, use YAML text without code fences, unless the caller explicitly asks for a fenced block.
2. For ordinary Compose-only cases, the **entire input can be just** `services: [...]`. Do not add unnecessary sections.
3. **Never emit** `project`, `compose`, `type`, `enabled`, or `schema_version` in a normal `compile.yaml`. These belong to a fabricated schema or, in the case of `schema_version`, to **registry recipe definitions**, not project inputs.
4. A registry service reference uses the **registry key in `name`**: `postgresql`, not `postgres`; `redis`, not `cache`; `s3`, not `minio`; `ec2`, not `localstack`.
5. **Do not put `type` beside `name`.** There is no `{name: alias, type: recipe}` syntax in the demonstrated project-input schema.
6. The value of `version` is a **quoted string**, and it must be one of that service's **supported version keys**. Never infer version keys from popular Docker tags.
7. If a service is not in the registry or properly installed project cache, reference it only as a **fully specified custom service** with an `image`, or report that its recipe is unavailable. `name` plus `version` does not magically create a recipe.
8. `kubernetes` and `cicd` are **structured configuration objects or omitted/null**, not Boolean feature toggles. Do not use `kubernetes: {enabled: true}` or `cicd: {enabled: false}`.
9. Do not invent a `compose.enabled` switch. For tested service-only input, DevInit creates `docker-compose.yaml` from the `services` list.
10. Respect the requested test polarity: a **positive** example must be plausibly valid; a **negative** example should contain **one deliberate violation** with a stated expected failure. Do not label a valid file as invalid merely in a comment.
11. Do not fabricate credentials, hardcode real passwords, or write secrets into `compile.yaml`. Standard service recipes may require environment variables at Docker Compose runtime.
12. If unsure about an advanced field or exact CI/CD/Kubernetes schema, **omit that feature or ask for the actual schema/template**. Do not hallucinate field names.

## 2. Supported high-level input shape

At the top level, the demonstrated config is equivalent to:

```yaml
services: []
# Optional: kubernetes: <a valid Deployment object, or null>
# Optional: cicd: <a valid Pipeline object, or null>
```

- `services`: sequence of service references and/or complete custom service recipes. In the Rust model it defaults to an empty sequence when omitted, but use `services: []` for clarity.
- `kubernetes`: optional Kubernetes **Deployment object**, **not** an enabled flag. There are also Kubernetes manifest/preset features elsewhere in the codebase; do not assume their shape without the corresponding schema.
- `cicd`: optional CI/CD **Pipeline object**, **not** an enabled flag.
- Unknown top-level keys may not all be rejected by the current deserializer; **that does not make them implemented features**. Generate only documented keys.

### Service reference syntaxes

**Shorthand, default version:**

```yaml
services:
  - postgresql
  - redis
```

**Named reference with supported version:**

```yaml
services:
  - name: postgresql
    version: "16"
  - name: redis
    version: "7"
```

**Custom service with explicit image** (no registry lookup required):

```yaml
services:
  - name: my-api
    image: ghcr.io/example/my-api:1.0
    ports:
      - "127.0.0.1:8080:8080"
    restart: unless-stopped
```

For custom recipes the known `RecipeCompose` model includes: `name`, `image`, and optional `description`, `ports`, `environment`, `volumes`, `networks`, `depends_on`, `restart`, `command`, `files`, `env`, `notes`. Use their established YAML value shapes. Only add fields necessary for the test. `files` is handled as either a single `{path, content}` object or a list of such objects. The generator rejects unsafe relative output paths and duplicates.

**Not supported as a way to select a registry service:**

```yaml
services:
  - name: postgres
    type: postgresql
    version: "16"
```

Use `name: postgresql`, omit `type`. A custom alias can be modeled only by a complete custom recipe (with an explicit `image`) or by supported override behavior; do not invent a `type` pointer.

## 3. Registry catalog and version discipline

The development project has **12 installed YAML service definitions**. The keys below are confirmed from the team's YAML migration and tests. Defaults/versions shown are **historically observed development values**, not a promise that every future installation keeps them. The installed `resources/services/<name>.yaml` is authoritative.

| Registry key | Observed default | Observed available keys / notes |
|---|---|---|
| `postgresql` | `16` | `15`, `16`, `17` |
| `redis` | `7` | `6`, `7` |
| `mysql` | `8.4` | `8.4`, `9`, `26` |
| `mariadb` | `11.4` | `10.11`, `11.4`, `11.8` |
| `mongodb` | `7` | `7`, `8`, `8.0` (verify actual version-key set in installed YAML) |
| `rabbitmq` | `4.1` | `3.13`, `4.0`, `4.1` |
| `kafka` | `4.3.1` | `4.2.2`, `4.3.1`, `4.4.0`; KRaft recipe |
| `grafana` | `12.4.1` | `12.3.2`, `12.3.3`, `12.4.1` |
| `prometheus` | `3.13.3` | `3.13.3` verified |
| `elasticsearch` | `9.5.5` | `9.5.3`, `9.5.4`, `9.5.5` |
| `s3` | `2025-09-07` | MinIO-backed recipe. Quay image pull returned 401 in testing; generated Compose succeeded, container startup not verified. |
| `ec2` | `2026.10.4` | LocalStack-backed recipe; authorization token required; generated Compose succeeded, container startup not verified. |

**Special warning:** In the 30 bad examples, `kafka` `version: "3.8"` recurs; that key is **not** in the migrated Kafka definition described above. Do not generate it just because it is a real Kafka release family. Always use *registry keys*, not upstream product-version intuition.

**Registry vs project file:** `resources/services/kafka.yaml` contains service-definition fields such as `schema_version`, `default_version`, `versions`, `defaults`. Those **must not** be copied into ordinary project `compile.yaml` service references.

## 4. Output generation and runtime are separate stages

A successful `devinit compile` means that an output configuration was generated; **it does not prove Docker images are downloadable or containers are healthy**.

Normal verification sequence:

```text
devinit check
devinit compile --dry-run
devinit compile
```

For infrastructure execution (only if Docker is installed/running):

```text
docker compose config -q
docker compose up -d
docker compose ps
```

- `check` builds and validates the generation plan without writing output files.
- `--dry-run` previews CREATE/CONFLICT/OVERWRITE without writing.
- `compile` writes planned outputs, and protects existing files unless force is explicitly supplied.
- Standard service recipes may generate additional files (Prometheus generated `prometheus/prometheus.yml` and `prometheus/alerts.yml` in addition to Compose).
- Do not assume Compose output means the Docker Engine is running. On Windows, Docker Desktop Linux Engine must be available for `docker compose up`.
- Docker Compose may demand environment variables such as `POSTGRES_PASSWORD`, `MINIO_ROOT_USER`, `MINIO_ROOT_PASSWORD`, and `LOCALSTACK_AUTH_TOKEN`. This is a **runtime configuration requirement**, not permission to invent credentials.
- Authenticated registry downloads and a successful YAML compilation are separate concerns.

## 5. Known-valid examples (positive tests)

### P01 — Minimal valid input / empty service list

```yaml
services: []
```

Valid project input, **but do not assert that a file will necessarily be produced**. An empty plan may contain zero outputs depending on active generation features.

### P02 — PostgreSQL and Redis

```yaml
services:
  - name: postgresql
    version: "16"
  - name: redis
    version: "7"
```

This exact pattern generated `docker-compose.yaml` successfully on Windows in a FastAPI project. Docker startup subsequently required `POSTGRES_PASSWORD`, which is expected.

### P03 — Two default-version services in shorthand

```yaml
services:
  - postgresql
  - redis
```

Supported by integration tests validating backwards compatibility.

### P04 — Kafka and RabbitMQ with verified registry version keys

```yaml
services:
  - name: kafka
    version: "4.3.1"
  - name: rabbitmq
    version: "3.13"
```

Both keys are defined in the observed YAML Registry. Actual Kafka startup additionally requires an appropriate `KAFKA_CLUSTER_ID` and runtime environment; do not add arbitrary Kubernetes flags.

### P05 — Prometheus with additional generated files

```yaml
services:
  - name: prometheus
    version: "3.13.3"
```

Observed generation: `docker-compose.yaml`, `prometheus/prometheus.yml`, `prometheus/alerts.yml`. Prometheus readiness and self-scraping were verified.

### P06 — Custom HTTP service (no registry entry needed)

```yaml
services:
  - name: demo-http
    image: nginx:stable
    ports:
      - "127.0.0.1:8080:80"
```

A complete custom service must include an actual image. Whether Docker can pull the image is outside DevInit's compile validation.

### P07 — Explicitly omit Kubernetes/CI

```yaml
services:
  - redis
```

To avoid Kubernetes and CI/CD output, omit those sections. Do **not** write `kubernetes: {enabled: false}` or `cicd: {enabled: false}`. An explicit `kubernetes: null` / `cicd: null` is also consistent with the optional data model, but unnecessary.

## 6. Known-invalid examples (negative tests)

Each test below deliberately introduces a single flaw. Do not append multiple unrelated errors unless testing error aggregation specifically.

### N01 — Unknown service

```yaml
services:
  - name: service-that-is-not-installed
    version: "1.0"
```

Expected: service-not-found, **provided the service is absent from both installed YAML Registry and project cache**. Do not assume this is guaranteed invalid in an arbitrary developer environment.

### N02 — Unsupported PostgreSQL version

```yaml
services:
  - name: postgresql
    version: "999"
```

Expected: unsupported-version error. Installed definition has priority; DevInit must not fall back to stale cache for an unsupported installed-service version.

### N03 — Invalid `type` key

```yaml
services:
  - name: postgresql
    type: database
    version: "16"
```

Observed error: `services[0]: unknown field 'type'`.

### N04 — Invalid service reference / alias mistake

```yaml
services:
  - name: postgres
    version: "16"
```

Expected: unknown service unless a recipe named `postgres` was explicitly installed/cached. The built-in key is `postgresql`.

### N05 — Duplicate service key

```yaml
services:
  - redis
  - redis
```

Expected: duplicate service validation failure. Tests cover duplicate service names.

### N06 — Invalid version type

```yaml
services:
  - name: redis
    version: 7
```

`version` is modeled as an optional string. Do not emit numeric scalars. Depending on YAML deserializer behavior, the observed failure message may vary; assert failure semantics, not a fabricated exact string.

### N07 — Invalid port

```yaml
services:
  - name: api
    image: example/api:1.0
    ports:
      - "70000:80"
```

Expected: port validation error (host port outside 1–65535). The image is a placeholder for configuration testing, not a claim that it exists remotely.

### N08 — Missing image in an explicit custom service

```yaml
services:
  - name: mystery-api
    ports:
      - "8080:8080"
```

Expected: an unresolved/invalid service, **provided** `mystery-api` is neither installed nor cached. To specifically test the custom-image validator, use the exact custom-service representation and fixture expected by the corresponding Rust validator test, rather than relying on unknown-service resolution.

### N09 — Circular dependencies in custom services

```yaml
services:
  - name: api-a
    image: example/api-a:1.0
    depends_on:
      - api-b
  - name: api-b
    image: example/api-b:1.0
    depends_on:
      - api-a
```

Expected: circular dependency error, if these names are accepted as custom service declarations. This pattern is consistent with dependency-graph unit tests.

### N10 — Unsupported Boolean feature switch

```yaml
services:
  - redis
kubernetes:
  enabled: true
```

This is an **invalid intended feature configuration**: `kubernetes` is not a Boolean enablement wrapper. Exact failure may be a deserialization error or missing Deployment fields.

## 7. Why the 30 uploaded examples failed as a dataset

The archive's examples use the same fabricated template repeatedly:

```yaml
project:
  name: some-project
services:
  - name: postgres
    type: postgresql
    version: "16"
compose:
  enabled: true
kubernetes:
  enabled: false
cicd:
  enabled: false
```

This mixes four unsupported assumptions in a single supposedly valid sample. If the model is trained or few-shot prompted with it, it will learn the **wrong grammar**.

Recurring failure categories observed in `test-001.yaml` through `test-030.yaml`:

| Failure pattern | Representative files | Correction |
|---|---|---|
| Invented `project.name` / `compose.enabled` | 001, 003, 021, 023 | Omit these top-level fields. |
| Invented `services[].type` | 002–010, 012–020, 022–024, 026, 028–030 | Remove `type`; use exact registry `name`. |
| Registry key mistaken for instance alias (`postgres`) | 003, 009, 013, 019, 023, 029 | Use `postgresql`. |
| Nonexistent Kafka registry version `3.8` | 004, 014, 024 | Use a version key confirmed in installed Kafka YAML. |
| Kubernetes/CI as Boolean flags | nearly all files | Omit optional sections, or supply a full supported object. |
| Fake “invalid” tests containing comments rather than invalid data | 007, 017, 027 | Introduce a real schema/validation violation; test actual CLI outcome. |
| Unsupported CI provider `actions` / unverified provider wrapper | 026 | Do not invent CI schema or provider spelling. |
| Generated custom service with `type`/`version` but no image | 002, 012, 022 | Supply an `image` or a known installed/cached recipe. |
| Duplicate examples repeatedly cloned | 003/023, 004/014/024, 009/019 | Diversify by **validated** features rather than template noise. |

**Important nuance:** examples with extra unknown top-level fields are not automatically proven rejected. The observed runtime rejected `services[].type`, while extra top-level keys may be ignored unless a strict top-level validator is enabled. They are *unsupported/unreliable*, not necessarily guaranteed parser failures.

## 8. Positive/negative dataset creation policy

For a dataset of `compile.yaml` files, record metadata **outside** `compile.yaml` (e.g., `manifest.jsonl`):

```json
{"file":"positive/pg-redis.yaml","expected":"valid","purpose":"two versioned registry services"}
{"file":"negative/unsupported-version.yaml","expected":"invalid","purpose":"unsupported PostgreSQL version"}
```

- Do not add `expected`, `scenario`, `valid`, or `test_type` keys to the configuration file itself.
- Use separate `positive/` and `negative/` directories.
- Maintain a roughly balanced but **non-duplicative** set of scenarios.
- Each positive sample must pass `devinit check` in a **clean fixture environment** with a known registry and cache state.
- Each negative sample must fail for the **intended reason**, not an earlier accidental `type` error.
- Run `devinit compile --dry-run` for valid cases to validate the output plan without writing.
- Keep actual generated outputs and errors in the dataset manifest or test reports, not as invented comments in YAML.
- For docker runtime tests, clearly distinguish `compile_valid` from `container_started` and `service_healthy`.
- Do not test `--force` by destroying user files; use throwaway test directories only.
- Quote version strings and port mapping strings.
- Do not invent build tools, Docker images, credentials, GitHub Actions schemas, or cloud resources.

## 9. Prompt template to give the local model

**System / highest-priority instructions:**

> You generate `compile.yaml` inputs for the DevInit CLI, **not Docker Compose files**. Follow the accompanying `DEVINIT_AI_CONFIGURATION_SPEC.md` exactly. Generate only documented keys. For a normal service-only file, use just `services`. Registry services have `name` and optionally a quoted `version`; never use `type`. `project`, `compose.enabled`, `kubernetes.enabled`, and `cicd.enabled` are not part of the documented input schema. Use registry names exactly (`postgresql`, not `postgres`) and only versions listed in the installed registry YAML. Custom services require an explicit `image`. Do not invent CI/CD or Kubernetes object fields. For valid examples emit YAML only, without Markdown fences or prose. If asked for intentionally invalid YAML, violate exactly one documented constraint and identify the intended failure **outside the YAML**. If required details are unknown, prefer a minimal correct configuration rather than guessing.

**Example user instruction:**

> Generate five DIFFERENT **valid** `compile.yaml` files for the current installed registry. Use PostgreSQL, Redis, Kafka, Prometheus and one custom HTTP service across the five cases. Each file should have a different purpose. Include no project/type/enabled fields. Use only documented version keys. Output five separate named YAML blocks.

**Required model self-check before answering:**

```text
[ ] Is this DevInit input YAML, not generated Docker Compose?
[ ] Is `services` a YAML sequence?
[ ] Does every registry reference use the correct installed key?
[ ] Is every `version` quoted and listed in installed registry YAML?
[ ] Does every custom non-registry service have `image`?
[ ] Did I remove `project`, `compose`, `type`, and `enabled` flags?
[ ] Did I omit Kubernetes/CI unless I know their exact full schema?
[ ] Did I avoid invented env credentials and unsupported fields?
[ ] For negative samples, is exactly one intentional error present?
[ ] Are any assumed runtime-success claims explicitly separated from compile success?
```

## 10. Validation gate before accepting model output

The model's output is only a **candidate**. The final authority is the actual DevInit CLI and installed Registry:

```text
1. Parse YAML syntax.
2. Run `devinit check` in an isolated temp project.
3. Compare actual exit code and error category with intended expectation.
4. For passing candidates, run `devinit compile --dry-run`.
5. Record planned files and errors; reject mislabeled samples.
6. Deduplicate equivalent configurations.
7. Manually inspect any Kubernetes, CI/CD, security-sensitive or advanced overrides.
```

**Do not promote a sample into a trusted training set solely because it looks plausible.** The original 30 samples are useful as examples of common model mistakes, not as gold-standard valid DevInit YAML.

---

### Scope boundaries / current unknowns

This specification is grounded in observed tests and previously shared Rust types, **not a fresh read of the entire repository**. It does not claim to exhaustively document Kubernetes manifest presets, CI/CD job YAML, advanced registry overrides, remote-backend authentication, or all `ServiceDefinition` fields. Their precise external YAML schema should be extracted from the current source and validated before inclusion in a future expanded revision. Existing cache state and the set of installed definitions can also change how an unknown service is resolved.
