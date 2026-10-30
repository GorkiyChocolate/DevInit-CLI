# DevInit: Detailed Documentation

This document describes the actual state of the `devinit_cli_2` project based on its source code. It is intended for users who want to run the CLI and developers who need to understand the purpose of each file, function, and processing stage.

## 1. Project Purpose

DevInit is a Rust CLI for preparing configuration files from a single YAML description. The main workflow is the `devinit compile` command:

```text
compile.yaml
    -> check that the file exists and is not empty
    -> read as UTF-8
    -> deserialize YAML into CompileSpec
    -> validate services, Kubernetes, and CI/CD
    -> check dependencies between services
    -> generate target YAML files
    -> atomically write the results
```

Generation is performed only after successful deserialization and complete validation. The project does not run Docker, Kubernetes, or CI/CD; it only creates configuration files.

## 2. Quick Start

Rust and Cargo must be installed.

```bash
cargo build
cargo test
cargo run -- compile
```

`compile` always looks for `compile.yaml` in the current working directory. To run an already-built binary:

```bash
./target/debug/devinit_cli_2 compile
```

Main help commands:

```bash
cargo run -- --help
cargo run -- compile --help
```

## 3. Project Structure

```text
Cargo.toml                         package manifest and dependencies
README.md                          brief project description
Instruction.md                     compile.yaml schema and examples
TODO.md                            implemented features and development plan
compile.yaml                       Compose + GitHub Actions example
compile12.yaml                     built-in services only example
compile43.yaml                     Compose + Kubernetes + GitLab CI example
resources/services.yaml            reference service list
src/lib.rs                         library module exports
src/main.rs                        binary entry point
src/errors.rs                      error types
src/api/                            HTTP client
src/cli/                            CLI command definitions and routing
src/file_config/                   configuration file reading and writing
src/generator/                     target file generation
src/models/                        YAML data models
src/validator/                     model validation
 tests/project_tests.rs             integration tests
```

## 4. Application and CLI Execution

### `src/main.rs`

This file is the binary application's entry point.

- `main()` calls `cli::cli_logic::cli_logic()`. If the function returns an error, the process exits with code `1`. The error text itself is printed inside the CLI logic for the relevant scenarios.

### `src/lib.rs`

This file turns the project into a library used by integration tests and exports these modules:

- `api` — HTTP requests;
- `cli` — command construction and execution;
- `file_config` — file operations;
- `models` — data structures;
- `validator` — validation;
- `errors` — shared errors;
- `generator` — result generation.

There are no functions in this file.

### `src/cli/commands.rs`

This module builds the command tree with `clap`.

- `build_cli() -> Command` creates the root `devinit` command, sets version `0.0.2`, author, description, and required subcommand, and registers all subcommands.
- `build_add_cli() -> Command` describes `add <service>` with `--port/-p` and `--version/-v` options. These options are parsed but are not yet used in the request.
- `build_list_cli() -> Command` describes `list` with `--type/-t` and `--page/-p` options.
- `build_get_cli() -> Command` describes `get <service_url>`.
- `build_license_cli() -> Command` describes `license <license_type>`.
- `build_login_cli() -> Command` describes `login <login_url>`.
- `build_compile_cli() -> Command` describes `compile` without arguments.
- `build_test_cli() -> Command` describes `test` with an optional `tester` argument.

The registered commands have different levels of readiness:

| Command | Actual behavior |
| --- | --- |
| `add service` | Requests a recipe from the local API and appends Compose/env examples |
| `get service_url` | Requests a list of configurations and appends a Compose example |
| `compile` | Reads, validates, and generates files |
| `list` | Only prints the supplied filters |
| `login` | Only prints the URL; the login API is not called |
| `license` | Registered, but has no separate handling |
| `test` | The handling branch is empty |

### `src/cli/cli_logic.rs`

This is the main command router and the public entry point of the program pipeline.

- `compile() -> Result<Vec<PathBuf>, DevinitError>` compiles the standard `compile.yaml` file into the current directory `.`. It is a convenience wrapper around `compile_at`.
- `compile_at(input_path, output_dir) -> Result<Vec<PathBuf>, DevinitError>` loads `CompileSpec` with `load_compile`, calls `validate_compile`, converts the error list into `DevinitError::ValidationErrors`, and, after successful validation, calls `generator::generate`.
- `print_validation_errors(errors)` privately prints all errors to stderr in the format “path + message”.
- `cli_logic() -> Result<(), DevinitError>` obtains arguments from `clap`, selects a subcommand, and executes it.

`add` uses these base URLs:

```text
http://localhost:5167/services/
http://localhost:5167/configs/
```

`add` calls `add_recipe`, writes the recipe to `compose.yaml.example` through `yaml_data`, and writes environment variables to `env.example` through `env_file_config`. `get` calls `get_config` and appends the returned recipes through `yaml_configs_data`. Errors from these operations are printed but are not returned as top-level errors from the command branch.

## 5. Errors

### `src/errors.rs`

- `ValidationError` is a small structure with `path` and `message` fields. It represents one validation error and allows the user to see the exact location of the problem.
- `GenerationError` represents a generation-stage error. It includes `Io` for file creation/writing/renaming and `Serialization` for YAML serialization errors.
- `DevinitError` is the application's general enum:
  - `HttpRequestError` — a `reqwest` error;
  - `JsonParseError` — a JSON deserialization error;
  - `FileIOError` — a filesystem error;
  - `YamlParseError` — a YAML error;
  - `ValidationErrors` — a list of structured validation errors;
  - `GenerationError` — a generator error;
  - `InvalidInput`, `ServiceNotFound`, `ConfigurationError` — input, service, and configuration errors;
  - `K8sValidationError`, `CiCdGenerationError`, `EnvConfigError`, `StorageError`, `ServiceGenerationError`, `Other` — extensions for future and separate subsystems. Not all variants are used in the current runtime path.

## 6. HTTP API

The API modules use a synchronous `reqwest::blocking::Client`. Each request checks the HTTP status with `error_for_status()` and then deserializes JSON.

### `src/api/add_recipe.rs`

- `add_recipe(recipe_name, base_url) -> Result<RecipeCompose, Box<dyn Error>>` joins `base_url` and the recipe name, performs a GET request with the `Accept: application/json` header, receives one `RecipeCompose`, and returns it to the caller.

### `src/api/get_config.rs`

- `get_config(config_url, config_name) -> Result<ConfigsList, Box<dyn Error>>` performs a GET request to the combined URL and receives a `ConfigsList` object containing a list of recipes.

### `src/api/login.rs`

- `login(company_url) -> Result<bool, Box<dyn Error>>` performs a POST request, checks the status, and expects a JSON boolean. The function is implemented, but the current `login` CLI branch does not call it.

## 7. Reading and Writing Files

### `src/file_config/yaml_config.rs`

- `load_compile(path) -> Result<CompileSpec, DevinitError>` checks that the input file exists, calls `ensure_file_not_empty`, reads it as a UTF-8 string, and deserializes it with `serde_yaml`.
- `yaml_data(config_struct, path) -> io::Result<()>` passes one `RecipeCompose` to the internal recipe-appending function.
- `yaml_configs_data(configs_list, path) -> io::Result<()>` passes all recipes from `ConfigsList` to it.
- `append_recipes(recipes, path) -> io::Result<()>` is the shared private implementation: it skips an empty list, rejects duplicate Docker images, opens the file in append mode, adds a separator, removes the `env` field from the Compose section, and serializes each recipe under its name.

These functions are used by the `add` and `get` commands, not by the main `compile` generator.

### `src/file_config/file_validator.rs`

- `ensure_file_not_empty(path)` reads the file and returns `InvalidData` if it is empty after whitespace is removed.
- `ensure_images_are_new(path, recipes)` collects images from existing YAML and checks both for duplicates in the file and duplicates within the new batch.
- `ensure_name_is_new(path, name)` checks comments in the form `# devinit config: <name>` in the env file and rejects a duplicate name.
- `read_existing_images(path)` privately returns the set of images from existing Compose recipes; for a missing or empty file it returns an empty set.
- `read_existing_names(path)` privately reads env-file lines and returns names from the special comments.

### `src/file_config/yaml_writer.rs`

- `write_yaml<T: Serialize>(value, path) -> Result<PathBuf, GenerationError>` creates parent directories, serializes the value to YAML, writes the result to a temporary file with a `.tmp` extension, and then renames it to the target path. It returns the result path. This order reduces the risk of leaving partially written YAML.

### `src/file_config/env_config.rs`

- `env_file_config(recipe, path) -> io::Result<()>` does nothing if `recipe.env` is absent or empty. Otherwise, it checks name uniqueness, opens the file in append mode, adds the `# devinit config: ...` comment, and writes each environment line.

## 8. Input Data Models

### `src/models/compile_struct.rs`

- `CompileSpec` is the root of `compile.yaml`: `services`, `kubernetes`, and `cicd`.
- `CompileService` is an untagged enum. A YAML string is parsed as `BuiltIn(Services)`, while an object is parsed as `Custom(Box<RecipeCompose>)`.
- `CompileService::to_recipe()` converts both variants into a single `RecipeCompose`: a built-in service calls `Services::to_recipe`, while a custom service is cloned.

### `src/models/docker_compose_struct.rs`

- `RecipeCompose` describes a service: `name`, `description`, `image`, `ports`, `environment`, `volumes`, `networks`, `depends_on`, `restart`, `command`, `files`, `env`, and `notes`.
- `ConfigsList` is a wrapper with a `configs: Vec<RecipeCompose>` field for the API.
- `File` is a nested file with `path` and `content`.

The `files`, `env`, and `notes` fields are stored by the model but are not emitted by the main Docker Compose generator. `env` is used separately by `env_file_config` when importing a recipe.

### `src/models/services_struct.rs`

`Services` contains the built-in values `postgresql`, `mysql`, `mongodb`, `mariadb`, `elasticsearch`, `prometheus`, `grafana`, `redis`, `rabbitmq`, `kafka`, `s3`, and `ec2`.

- `Services::to_recipe()` creates a standard recipe with a name, description, image, ports, and the `unless-stopped` policy. For S3 it adds the `server /data` command.
- `services_selector(services)` calls one of the specialized placeholder validators for some services and prints a message. It always returns `Ok(())`; the validator result is currently not analyzed.

The values in `to_recipe` are hard-coded. `resources/services.yaml` is a reference file and is not loaded by the current code.

### `src/models/env_struct.rs`

- `Env` is an empty placeholder structure. It has no fields or methods.

### `src/models/cicd_struct.rs`

- `Pipeline` is a CI/CD configuration with a `provider`, a list of `triggers`, and a list of `jobs`.
- `CiProvider` is either `Github` or `Gitlab`.
- `Trigger` is `Push { branches }`, `PullRequest { branches }`, or `Manual`.
- `Job` has a name, runner, `needs` dependencies, and steps.
- `Runner` is `UbuntuLatest`, `WindowsLatest`, or `MacosLatest`; the default is Ubuntu.
- `Step` has a step name, a `StepAction`, and an environment variable map.
- `StepAction` is `Checkout`, `Run { command }`, `DockerBuild { dockerfile, image }`, or `DockerPush { image }`.

Serde uses snake_case for input enum values, such as `provider: github` and `action: docker_build`.

### `src/models/k8s_struct.rs`

This file contains a typed Kubernetes Deployment model and no functions. The main structures are:

- `Deployment` — `api_version`, `kind`, `metadata`, `spec`;
- `ObjectMeta` — name, namespace, labels, annotations;
- `DeploymentSpec` — replicas, selector, template, and rollout parameters;
- `DeploymentStrategy`, `RollingUpdate` — strategy and update constraints;
- `LabelSelector`, `PodTemplateSpec`, `PodSpec` — selector, Pod template, and Pod settings;
- `Container`, `ContainerPort` — container, image, commands, ports, env, probes, resources, and mounts;
- `EnvVar`, `EnvVarSource`, `KeySelector`, `ObjectFieldSelector` — variables and value sources;
- `EnvFromSource`, `ConfigMapEnvSource`, `SecretEnvSource` — bulk environment imports;
- `ResourceRequirements` — requests and limits;
- `VolumeMount`, `Volume` — volume attachments;
- `EmptyDirVolumeSource`, `ConfigMapVolumeSource`, `SecretVolumeSource`, `PersistentVolumeClaimVolumeSource`, `HostPathVolumeSource` — five volume-source variants;
- `Probe`, `HttpGetAction`, `TcpSocketAction`, `ExecAction`, `GrpcAction` — health checks;
- `SecurityContext`, `PodSecurityContext` — security settings;
- `LocalObjectReference` — a reference to a Kubernetes object.

Most structures use `camelCase` during serialization (`apiVersion`, `imagePullPolicy`, `containerPort`, and so on). Unspecified optional fields are omitted through `skip_serializing_if`.

## 9. Validation

### `src/validator/compile_validator.rs`

- `validate_compile(spec) -> Result<(), Vec<ValidationError>>` is the general validator. It converts each service to a `RecipeCompose`, validates services, Kubernetes, and CI/CD, and then checks cross-service references and cycles. It returns all detected errors at once.
- `validate_cross_references(spec, errors)` privately checks duplicate names and the existence of every `depends_on` reference.
- `detect_cycles(spec, services, errors)` starts a dependency traversal for every service.
- `visit_service(...) -> bool` performs a DFS traversal. The `visiting` set detects a back edge, while `visited` prevents already processed nodes from being traversed again.

### `src/validator/services_validator.rs`

- `validate_service(name, service, errors)` checks the name, image, ports, environment, volumes, networks, `depends_on`, and `files.path`.
- `validate_port(value, path, errors)` privately takes the part after the last `:`, parses it as a `u16`, and requires a value greater than zero.

The following specialized functions are public but currently act as stubs and always return `true`:

- `validate_postgresql_service()`;
- `validate_mysql_service()`;
- `validate_mongodb_service()`;
- `validate_mariadb_service()`;
- `validate_elasticsearch_service()`;
- `validate_prometheus_service()`;
- `validate_grafana_service()`;
- `validate_redis_service()`;
- `validate_rabbitmq_service()`;
- `validate_kafka_service()`;
- `validate_s3_service()`;
- `validate_ec2_service()`.

### `src/validator/k8s_validator.rs`

- `validate_deployment(deployment, errors)` checks the Deployment name, `apiVersion`, `kind`, a positive replica count, the presence of containers, container names/images, non-zero container ports, valid environment variables, mutual exclusion of `value` and `valueFrom`, volume-mount names, mount paths, volume names, and exactly one volume source.
- `validate_name(name, path, errors)` privately rejects an empty name.

The validator does not check the complete Kubernetes schema contract. For example, selector/label consistency and the correctness of all numeric parameters remain outside the current implementation.

### `src/validator/cicd_validator.rs`

- `validate_pipeline(cicd, errors)` checks non-empty and unique job names, non-empty step names, non-empty `run` commands, and the existence of all names in `needs`.
- `cicd_validator(cicd) -> io::Result<()>` is an adapter for the old interface: it runs the main validation and returns one `std::io::Error` containing the number of problems if errors are found.

## 10. Generators

### `src/generator.rs`

- `generate(spec, output_dir) -> Result<Vec<PathBuf>, GenerationError>` sequentially generates Compose, Kubernetes, and CI/CD files only for non-empty or specified parts of `CompileSpec`. It returns the list of created paths.

### `src/generator/docker_compose_generator.rs`

- `generate_docker_compose(services, output_dir)` converts all `CompileService` values into recipes, stores them in a `BTreeMap` by name, builds the Compose model, and writes `docker-compose.yaml`.
- `ComposeService::from_recipe(recipe)` is a private constructor that copies supported Compose fields and converts `description` into the `com.devinit.description` label.
- `DockerComposeFile` is the internal root object with a `services` map.
- `ComposeService` is the internal representation. It serializes image, ports, environment, volumes, networks, depends_on, restart, command, and labels. The `files`, `env`, and `notes` fields are not included in Compose.

### `src/generator/k8s_generator.rs`

- `generate_k8s(deployment, output_dir)` creates `<output_dir>/k8s/<metadata.name>-deployment.yaml`, passes the Deployment to the shared `write_yaml`, and returns the path in a one-element `Vec`.

### `src/generator/cicd_generator.rs`

- `generate_cicd(pipeline, output_dir)` selects GitHub Actions or GitLab CI based on `CiProvider`.
- `generate_github_actions(pipeline, output_dir)` privately builds `GithubWorkflow` and writes `.github/workflows/devinit.yml`.
- `generate_gitlab_ci(pipeline, output_dir)` privately builds `GitlabPipeline` and writes `.gitlab-ci.yml`.
- `GithubWorkflow::from_pipeline(pipeline)` converts triggers and jobs into an internal GitHub model.
- `github_trigger(branches)` creates a trigger with branches, or an empty trigger if the branch list is empty.
- `runner_name(runner)` converts a runner to `ubuntu-latest`, `windows-latest`, or `macos-latest`.
- `github_step(step)` converts checkout to `actions/checkout@v4`, run to a shell command, Docker build/push to the corresponding commands, and carries over the environment.
- `GitlabPipeline::from_pipeline(pipeline)` builds stages from job names and a map of GitLab jobs.
- `gitlab_command(step)` converts run/build/push to a script command; checkout is ignored in GitLab and returns `None`.

The GitHub result contains `name: DevInit`, `on`, and `jobs`. In GitLab, each job receives a stage with the same name, and `needs` is copied directly.

## 11. `compile.yaml` Format

Minimal top-level form:

```yaml
services: []
kubernetes: null
cicd: null
```

### Built-in Services

```yaml
services:
  - postgresql
  - redis
```

Supported names: `postgresql`, `mysql`, `mongodb`, `mariadb`, `elasticsearch`, `prometheus`, `grafana`, `redis`, `rabbitmq`, `kafka`, `s3`, `ec2`.

### Custom Service

```yaml
services:
  - name: backend
    description: API service
    image: ghcr.io/example/backend:1.0.0
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgres://postgres:5432/app
    depends_on:
      - postgresql
    restart: unless-stopped
```

Non-empty `name` and `image` are required. Ports must end in a numeric value greater than zero, environment entries must contain `KEY=VALUE`, and dependencies must reference existing services without forming a cycle.

### Kubernetes

```yaml
kubernetes:
  apiVersion: apps/v1
  kind: Deployment
  metadata:
    name: backend
  spec:
    replicas: 1
    selector:
      matchLabels: {app: backend}
    template:
      metadata:
        name: backend
        labels: {app: backend}
      spec:
        containers:
          - name: backend
            image: ghcr.io/example/backend:1.0.0
```

The minimum checks cover non-empty `apiVersion`, `kind`, and `metadata.name`, positive `replicas`, one or more containers, and non-empty container name/image.

### CI/CD

```yaml
cicd:
  provider: github
  triggers:
    - type: push
      branches: [main]
  jobs:
    - name: build
      runner: ubuntu-latest
      steps:
        - name: Checkout
          action: checkout
        - name: Build
          action: run
          command: cargo build --locked
```

Allowed providers are `github` and `gitlab`; allowed runners are `ubuntu-latest`, `windows-latest`, and `macos-latest`; allowed actions are `checkout`, `run`, `docker_build`, and `docker_push`.

## 12. Generated Results

For this configuration:

```yaml
services:
  - postgresql
kubernetes: null
cicd: null
```

The following file is created:

```text
docker-compose.yaml
```

If a Kubernetes Deployment named `backend` is specified, `k8s/backend-deployment.yaml` is added. For GitHub, `.github/workflows/devinit.yml` is added; for GitLab, `.gitlab-ci.yml` is added.

Repeated generation replaces the file through a temporary `.tmp` file and does not create a second result with the same name.

## 13. Tests

### `tests/project_tests.rs`

Helper functions:

- `temp_paths(name)` creates unique temporary paths for `compile.yaml` and the output;
- `valid_yaml()` returns minimal valid YAML;
- `deployment()` creates a valid Kubernetes test model;
- `service(name, image, depends_on)` creates a basic Compose recipe.

The tests cover successful compilation, no generation after an error, malformed YAML, missing and empty files, repeated generation, simultaneous generation of all three targets, GitHub/GitLab structure, Compose labels, Kubernetes YAML, a missing CI dependency, an empty image, an invalid port, a missing/cyclic dependency, and an empty container image.

Run:

```bash
cargo test
```

## 14. Other Files and Dependencies

### `Cargo.toml`

The package uses edition 2024. Main dependencies:

- `clap` — CLI;
- `serde`, `serde_json` — serialization and JSON;
- `yaml_serde` under the `serde_yaml` name — YAML;
- `reqwest` — HTTP;
- `thiserror` — error enums;
- `ureq`, `inquire`, `dotenvy` — declared but not used by the current reviewed runtime code.

### `resources/services.yaml`

A reference YAML containing built-in services, images, descriptions, ports, and the S3 command. It is not currently loaded by the program: the actual source of values is the match expression in `Services::to_recipe()`.

### `README.md`

A brief conceptual description of the project's goals. For the exact schema and current behavior, use this document and `Instruction.md`.

### `Instruction.md`

User instructions for `compile.yaml`, the `compile` command, services, Kubernetes, and CI/CD. It contains format examples and limitations.

### `TODO.md`

Development plan: dry-run/check, force/no-clobber, JSON errors, Dockerfile and Compose checks, Kubernetes Service/probes/resources, environment profiles, registry login, cache, and an expanded test matrix.

### `compile.yaml`, `compile12.yaml`, `compile43.yaml`

Input examples:

- `compile.yaml` — PostgreSQL, Redis, a custom API, and GitHub Actions;
- `compile12.yaml` — an example with built-in services;
- `compile43.yaml` — Compose, a Kubernetes Deployment, and GitLab CI with `needs`.

## 15. Known Limitations

1. `list`, `license`, `test`, and actual `login` authorization are not fully implemented yet.
2. `add` and `get` use hard-coded `localhost:5167` URLs.
3. The `add` command's `--port` and `--version` parameters do not affect the request.
4. Specialized checks for built-in services return `true` without validation.
5. `resources/services.yaml` is not runtime configuration.
6. The `files` and `notes` fields are not generated in the Compose file.
7. The full Kubernetes schema, label/selector consistency, host-port collisions, and Dockerfiles are not checked.
8. The generator overwrites existing target files.
9. Environment values imported from recipes are written to `env.example`, but separate secret masking is not implemented.

These items are reflected in `TODO.md` and should be considered when extending the project.
