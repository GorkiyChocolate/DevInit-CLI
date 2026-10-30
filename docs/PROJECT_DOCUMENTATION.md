# DevInit CLI — Technical Project Documentation

**Version:** 0.0.2  
**Language:** Rust (Edition 2024)  
**Project Type:** Declarative Infrastructure Configuration Generator  
**Status:** Active Development  
**Repository:** https://github.com/GorkiyChocolate/DevInit-CLI

---

## 1. Project Overview

### 1.1 Introduction

DevInit is a Rust-based command-line application designed to simplify infrastructure configuration management through declarative generation, validation, and reusable infrastructure abstractions.

The primary objective is to reduce the repetitive work involved in preparing infrastructure configuration files for different development and deployment environments.

Instead of independently maintaining Docker Compose files, Kubernetes manifests, and CI/CD workflows, developers can describe supported infrastructure requirements in a single `compile.yaml` file.

DevInit processes this configuration, resolves dependencies and reusable service definitions, validates supported constraints, and produces the corresponding configuration artifacts.

### 1.2 Project Objectives

The main objectives of DevInit are:

1. Provide a unified declarative interface for infrastructure configuration generation.
2. Reduce repetitive configuration work through built-in service recipes and reusable presets.
3. Support multiple infrastructure targets without requiring developers to manually maintain every output format.
4. Detect invalid configurations before generating files.
5. Protect existing project files against unintended modifications.
6. Support local, offline-first configuration generation.
7. Provide an extensible architecture for additional infrastructure integrations.

### 1.3 Project Scope

DevInit v0.0.2 focuses on configuration generation rather than infrastructure execution.

Supported generation targets include:

- Docker Compose
- Kubernetes Deployments and Services
- Supported raw Kubernetes manifests
- GitHub Actions
- GitLab CI/CD

The CLI does not automatically deploy applications, create cloud resources, build Docker images, or apply Kubernetes manifests.

Its primary responsibility is to produce configuration files that can subsequently be reviewed and executed using the appropriate external tools.

### 1.4 Design Principles

DevInit follows several key design principles.

**Declarative configuration:** Users describe the desired configuration rather than a sequence of provisioning commands.

**Separation of responsibilities:** Parsing, resolution, validation, generation, and filesystem operations are handled by separate components.

**Offline-first operation:** Built-in recipes and local configuration generation should work without requiring the DevInit backend.

**Backward compatibility:** Existing configuration formats and public module interfaces should remain stable unless a deliberate breaking change is introduced.

**Filesystem safety:** Existing user-authored files should not be overwritten without explicit authorization.

**Deterministic generation:** Identical resolved input should produce predictable output structure and file paths.

---

## 2. System Architecture

### 2.1 Architecture Overview

DevInit uses a staged configuration-processing architecture.

```text
                    compile.yaml
                         |
                         v
                Configuration Loader
                         |
                         v
                 YAML Deserialization
                         |
                         v
              Preset / Registry Resolver
                         |
                         v
               Configuration Validation
                         |
                         v
              Dependency Graph Analysis
                         |
                         v
               ResolvedCompileSpec
              (Intermediate Representation)
                         |
                         v
                  GenerationPlan
                         |
                         v
                 Filesystem Preflight
                         |
                 +-------+-------+
                 |               |
                 v               v
             Dry Run         File Writer
                                 |
                                 v
                       Generated Artifacts
```

This architecture separates the interpretation of configuration data from the actual modification of the filesystem.

The system can therefore validate a configuration and construct its generation plan without writing files.

### 2.2 Main Processing Stages

**Stage 1 — Configuration Loading**

The CLI locates and reads `compile.yaml`, checks its basic structure, and deserializes supported YAML content.

**Stage 2 — Resolution**

Built-in service recipes, version selections, reusable presets, and supported overrides are resolved into concrete configuration data.

**Stage 3 — Validation**

DevInit validates service declarations, dependency relationships, supported infrastructure fields, and relevant structural constraints.

**Stage 4 — Intermediate Representation**

Validated configuration data is represented through `ResolvedCompileSpec`, providing a common input for the generation system.

**Stage 5 — Generation Planning**

Generators construct output artifacts in memory and register them with `GenerationPlan`.

**Stage 6 — Filesystem Preflight**

Before writing, DevInit checks generated paths, existing files, conflicts, and supported filesystem safety conditions.

**Stage 7 — Output Generation**

When generation is authorized, the planned artifacts are written to their destination paths.

---

## 3. Repository Organization

### 3.1 Directory Structure

The Rust CLI uses the following source organization:

```text
DevInit-CLI/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── errors.rs
│   ├── generator.rs
│   │
│   ├── api/
│   │   ├── add_recipe.rs
│   │   ├── get_config.rs
│   │   └── login.rs
│   │
│   ├── cli/
│   │   ├── cli_logic.rs
│   │   └── commands.rs
│   │
│   ├── config/
│   │   └── presets.rs
│   │
│   ├── core/
│   │   ├── ir.rs
│   │   ├── resolver.rs
│   │   └── service_registry.rs
│   │
│   ├── docker/
│   │   └── docker_finder.rs
│   │
│   ├── file_config/
│   │   ├── env_config.rs
│   │   ├── file_validator.rs
│   │   ├── yaml_config.rs
│   │   └── yaml_writer.rs
│   │
│   ├── generation/
│   │   ├── generation_plan.rs
│   │   └── k8s_manifests.rs
│   │
│   ├── generator/
│   │   ├── cicd_generator.rs
│   │   ├── docker_compose_generator.rs
│   │   ├── k8s_generator.rs
│   │   └── k8s_service_generator.rs
│   │
│   ├── models/
│   ├── resolved_spec/
│   ├── validator/
│   └── tests/
│
├── tests/
│   └── project_tests.rs
│
└── docs/
    ├── PROJECT_DOCUMENTATION.md
    ├── USER_GUIDE.md
    ├── CONFIG_REFERENCE.md
    └── AI_INSTRUCTIONS.md
```

### 3.2 Module Compatibility

Several implementation files were reorganized into thematic directories for v0.0.2.

For example:

- `ir.rs` was moved into `core/`.
- `resolver.rs` was moved into `core/`.
- `service_registry.rs` was moved into `core/`.
- `presets.rs` was moved into `config/`.
- `generation_plan.rs` was moved into `generation/`.
- `k8s_manifests.rs` was moved into `generation/`.

The public Rust module paths remain compatible through declarations in `src/lib.rs`.

For example:

```rust
#[path = "core/ir.rs"]
pub mod ir;

#[path = "generation/generation_plan.rs"]
pub mod generation_plan;
```

This preserves existing references such as `crate::ir` and `crate::generation_plan`.

---

## 4. Application Entry Point and CLI

### 4.1 `src/main.rs`

The executable entry point initializes the CLI command handler and converts application errors into a nonzero process exit status.

The binary is named `devinit`, while the Cargo package retains the internal package name `devinit_cli_2`.

Separating these names avoids unnecessary changes to existing Rust imports.

### 4.2 `src/cli/commands.rs`

This module defines the command-line interface using the `clap` library.

It is responsible for:

- Registering available subcommands.
- Declaring supported command arguments and flags.
- Providing CLI help descriptions.
- Exposing version information.
- Passing parsed arguments to the command execution layer.

### 4.3 `src/cli/cli_logic.rs`

This module coordinates CLI execution.

Its primary responsibilities include:

- Dispatching commands.
- Loading configuration inputs.
- Constructing generation plans.
- Executing configuration checks.
- Performing dry-run previews.
- Running authorized file generation.
- Reporting validation and execution errors.

### 4.4 Core Commands

| Command | Purpose | File Modification |
|---|---|---|
| `devinit check` | Validate configuration and construct a generation plan | No |
| `devinit compile --dry-run` | Preview generation and report conflicts | No |
| `devinit compile` | Generate supported configuration files | Yes |
| `devinit compile --force` | Allow explicit replacement of existing output files | Yes |
| `devinit --help` | Display command help | No |
| `devinit --version` | Display version information | No |

Additional commands such as `add`, `get`, `list`, `login`, `license`, and `test` exist in the CLI interface.

These commands have different implementation maturity levels and should not all be treated as completed production features.

In particular, authentication and related remote functionality remain under development.

---

## 5. Configuration Processing

### 5.1 Configuration Input

The primary configuration file is named `compile.yaml`.

It contains declarations for supported infrastructure targets.

A basic example:

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

DevInit resolves the relevant service declarations and generates a Docker Compose configuration.

The detailed configuration contract is maintained separately in `docs/CONFIG_REFERENCE.md`.

### 5.2 YAML Loading

The `file_config` subsystem handles configuration reading and related filesystem operations.

Important modules include:

**`yaml_config.rs`**

Handles YAML configuration loading and deserialization.

**`file_validator.rs`**

Provides auxiliary checks for configuration files and imported recipes.

**`yaml_writer.rs`**

Contains reusable YAML serialization and writing functionality.

**`env_config.rs`**

Handles environment-related output associated with supported recipe-import operations.

These modules should remain separate from infrastructure-specific generation logic.

### 5.3 Preset Resolution

The `config/presets.rs` module provides support for reusable infrastructure configuration presets.

Presets allow developers to define common infrastructure configurations once and reference them across multiple projects.

Supported preset categories include:

- Docker Compose service groups.
- Kubernetes configurations.
- CI/CD pipelines.

Project-level overrides can customize supported preset properties without duplicating entire definitions.

### 5.4 Global Configuration

On Linux, DevInit supports global configuration using:

```text
~/.config/devinit/devinit.yaml
```

The `XDG_CONFIG_HOME` environment variable can affect the configuration location.

Windows-specific global configuration behavior must be verified against the target Windows build before documenting a final stable path contract.

### 5.5 Preset Design Constraints

Preset resolution should maintain the following properties:

1. Unknown preset names must produce understandable errors.
2. Invalid overrides must be rejected.
3. Supported overrides must preserve unrelated configuration fields.
4. Nested or recursive preset references must not introduce uncontrolled expansion.
5. Preset resolution must not silently alter unrelated services.

---

## 6. Built-in Service Registry

### 6.1 Purpose

The Service Registry reduces the need to manually describe commonly used infrastructure components.

Developers can select supported built-in services, and DevInit resolves them into concrete service configurations.

### 6.2 Registry Components

The registry functionality is primarily implemented through:

- `core/service_registry.rs`
- `core/resolver.rs`
- Supporting configuration models

The registry provides service definitions and supported version handling.

The resolver integrates those definitions into project configurations.

### 6.3 Supported Service Families

The current project includes built-in definitions for several infrastructure categories.

| Category | Services |
|---|---|
| Relational Databases | PostgreSQL, MySQL, MariaDB |
| NoSQL Databases | MongoDB |
| Caching | Redis |
| Messaging | RabbitMQ, Kafka |
| Monitoring | Prometheus, Grafana |
| Search | Elasticsearch |
| Additional Definitions | S3, EC2 |

The availability of specific versions and the behavior of individual definitions depend on the installed registry implementation.

Not every built-in identifier necessarily corresponds to a complete cloud provisioning implementation.

### 6.4 Version Resolution

Version resolution is intended to:

- Identify supported service versions.
- Select appropriate image definitions.
- Reject unsupported versions.
- Preserve explicitly supported configuration overrides.

### 6.5 Offline-First Behavior

Built-in service definitions and locally available cached recipes do not require access to the backend.

Remote recipes may require network connectivity and an available DevInit service endpoint.

---

## 7. Intermediate Representation

### 7.1 Purpose

DevInit uses an intermediate representation to separate configuration processing from output generation.

The main abstraction is `ResolvedCompileSpec`.

It represents a validated, resolved configuration that can be consumed by generators.

### 7.2 Responsibilities

The IR layer provides:

- Access to resolved configuration data.
- Dependency-aware service ordering.
- A common representation for generation.
- Separation between configuration source syntax and generated artifacts.

### 7.3 Dependency Graph

Services can declare dependencies using `depends_on`.

For example:

```yaml
services:
  - name: backend
    image: backend:latest
    depends_on:
      - postgresql

  - name: postgresql
    version: "17"
```

The dependency graph validator verifies that references are valid and detects circular relationships.

An invalid dependency graph must be rejected before generation.

Conceptually:

```text
backend
   |
   v
postgresql
```

A cyclic dependency, such as `backend` depending on `worker` while `worker` depends on `backend`, is invalid.

### 7.4 Deterministic Ordering

Dependency-aware ordering is useful for maintaining predictable processing behavior.

The graph implementation includes deterministic ordering tests to reduce unnecessary differences between repeated generations.

---

## 8. Validation System

### 8.1 Overview

The validation subsystem is located in `src/validator/`.

Its primary responsibility is to identify unsupported or inconsistent configuration data before files are generated.

Validation is divided into several components.

### 8.2 Compile Validation

**Module:** `validator/compile_validator.rs`

Coordinates validation of the resolved configuration.

It invokes relevant validators and collects structured errors.

### 8.3 Service Validation

**Modules:**

- `validator/services_validator.rs`
- `validator/project_validator.rs`

Supported checks include:

- Required service properties.
- Invalid service names.
- Duplicate service declarations.
- Invalid image declarations.
- Selected Docker Compose port mapping errors.
- Conflicting published ports.
- Invalid dependency references.
- Unsupported combinations of selected service properties.

Port validation includes checks for invalid numeric ranges and supported protocol declarations.

More complex Docker Compose port syntax is not fully supported by the custom validator.

### 8.4 Dependency Validation

**Module:** `validator/dependency_graph.rs`

The graph validator checks:

- Missing dependencies.
- Self-dependencies.
- Duplicate dependency declarations.
- Circular dependency chains.
- Dependency ordering.

### 8.5 Kubernetes Validation

**Modules:**

- `validator/k8s_validator.rs`
- `generation/k8s_manifests.rs`

DevInit supports validation of typed Kubernetes configurations and selected raw manifest structures.

For supported raw Deployment manifests, structural checks include:

- Required Deployment specification.
- Selector and Pod-label consistency.
- Presence of containers.
- Required container names and images.
- Selected container port constraints.

For supported Service manifests, validation includes selected service types and port constraints.

These checks detect common configuration mistakes but are not a replacement for complete Kubernetes API schema validation.

### 8.6 CI/CD Validation

**Module:** `validator/cicd_validator.rs`

CI/CD validation includes:

- Job name checks.
- Duplicate job detection.
- Required step properties.
- Missing job dependencies.
- Self-referencing dependencies.
- Circular job dependency detection.

Dependency cycles are detected through graph traversal.

### 8.7 Structured Errors

The `errors.rs` module defines the application's error representations.

DevInit distinguishes errors originating from:

- Configuration input.
- YAML parsing.
- Validation.
- Filesystem operations.
- Infrastructure generation.
- External service communication.

Where supported, validation errors identify the relevant configuration section or field.

Example:

```text
Configuration error:
k8s.backend: Deployment selector does not match Pod labels
```

Clear error messages are important both for human debugging and for correcting AI-generated configurations.

---

## 9. Generation Engine

### 9.1 Overview

The generation engine translates validated configuration data into infrastructure-specific output artifacts.

The architecture separates output rendering from file writing.

### 9.2 Docker Compose Generator

**Module:** `generator/docker_compose_generator.rs`

The Docker Compose generator transforms supported service declarations into a Compose-compatible YAML representation.

It handles supported properties such as:

- Container images.
- Port mappings.
- Environment variables.
- Volumes.
- Networks.
- Dependencies.
- Restart policies.
- Commands.
- Selected metadata labels.

The primary output is:

```text
docker-compose.yaml
```

### 9.3 Kubernetes Generators

**Modules:**

- `generator/k8s_generator.rs`
- `generator/k8s_service_generator.rs`
- `generation/k8s_manifests.rs`

DevInit supports Kubernetes generation through typed models, simplified templates, and selected raw manifests.

Depending on the configuration, generated resources are written under:

```text
k8s/
```

For example:

```text
k8s/backend.yaml
k8s/backend-service.yaml
```

Typed Kubernetes generation may use different output naming conventions, such as deployment-specific filenames.

Raw manifest processing also preserves supported configuration structures during serialization.

### 9.4 CI/CD Generator

**Module:** `generator/cicd_generator.rs`

The CI/CD generator converts supported pipeline declarations into target-specific workflow formats.

Supported providers include GitHub Actions and GitLab CI/CD.

Typical output paths:

```text
.github/workflows/devinit.yml
.gitlab-ci.yml
```

The generator handles supported job, runner, trigger, step, and dependency structures.

The resulting workflows must still be reviewed and validated against the target CI/CD platform.

---

## 10. GenerationPlan

### 10.1 Purpose

`GenerationPlan` is a central component of DevInit's file-generation architecture.

Instead of writing each generated artifact immediately, generators contribute planned files to an intermediate collection.

The plan can then be inspected and checked before any filesystem modification.

### 10.2 Planned Artifacts

A planned artifact contains its destination path and rendered output content.

The generation plan manages:

- Output collection.
- Duplicate destination detection.
- Deterministic ordering.
- Filesystem preflight checks.
- Authorized file writing.

### 10.3 Preview Mode

Running:

```bash
devinit compile --dry-run
```

allows users to inspect proposed file operations.

The preview distinguishes operations such as:

- `CREATE`
- `CONFLICT`
- `OVERWRITE`

For example:

```text
DevInit Dry Run

Planned files:
  CREATE     k8s/backend.yaml
  CREATE     docker-compose.yaml

Summary:
  2 files to create
  0 existing files to overwrite
  0 conflicts
  0 files written
```

This is an illustrative output showing the operation categories.

### 10.4 Check Mode

Running:

```bash
devinit check
```

validates the configuration and constructs the generation plan without writing files.

The check operation is intended for developers, automation pipelines, and pre-generation verification.

### 10.5 Force Mode

By default, existing output files are protected against overwriting.

Explicit replacement can be enabled using:

```bash
devinit compile --force
```

The force option does not disable all path-safety checks.

Users should review generated changes before using this option, especially when the destination contains manually maintained infrastructure configurations.

---

## 11. Filesystem Safety

### 11.1 Safety Objectives

DevInit is designed to avoid accidental modification or destruction of project files.

Relevant protections include:

- Existing-file conflict checks.
- Prevention of unintended overwrites.
- Validation of output paths.
- Rejection of path traversal.
- Symlink-related safety checks.
- Staged file writing.

### 11.2 Existing File Protection

When a generated output already exists, DevInit normally reports a conflict rather than replacing the file.

This behavior is intended to preserve manually edited configurations.

### 11.3 Path Validation

Generated output destinations are checked to prevent unsafe paths and supported forms of directory traversal.

Symlink checks help prevent writing to unexpected destinations.

### 11.4 Writing Strategy

The generation system uses temporary or staged file-writing operations to reduce the risk of incomplete output.

However, **multi-file generation is not guaranteed to be fully atomic**.

A filesystem or process failure can still interrupt a generation operation after some artifacts have been written.

Full transaction-like rollback and recovery remain areas for future improvement.

### 11.5 Security Boundaries

DevInit does not provide a complete sandbox for untrusted infrastructure configuration.

Generated CI/CD commands, container images, and raw Kubernetes manifests can contain user-supplied content.

Users remain responsible for reviewing generated files before executing them with Docker, Kubernetes, or CI/CD platforms.

---

## 12. Dockerfile Discovery

### 12.1 Current Implementation

**Module:** `docker/docker_finder.rs`

DevInit includes a Dockerfile discovery utility intended to locate Dockerfile-like files in project directories.

The utility can recognize common Dockerfile naming conventions and perform recursive directory traversal.

### 12.2 Current Limitation

Dockerfile discovery is not integrated into the primary Docker Compose generation pipeline in the v0.0.2 implementation reviewed for this documentation.

As a result, finding a Dockerfile does not automatically create the corresponding Compose `build` definition.

### 12.3 Planned Integration

Future work includes:

- Mapping Dockerfiles to services.
- Resolving build contexts.
- Handling multiple Dockerfiles.
- Supporting explicit selection when discovery is ambiguous.
- Generating `build.context` and `build.dockerfile`.
- Preserving existing image-based configurations.

This functionality is tracked as a future enhancement.

---

## 13. API Integration and Authentication

### 13.1 API Modules

The `src/api/` directory contains components for external service interaction.

**`add_recipe.rs`**

Supports retrieving individual service recipes from an API.

**`get_config.rs`**

Supports retrieving configuration collections.

**`login.rs`**

Contains authentication-related API functionality.

### 13.2 Implementation Status

The command interfaces for remote recipe retrieval and authentication are not equivalent to a completed account-management system.

In particular, the `devinit login` workflow requires further development before it can be treated as a secure, production-ready authentication mechanism.

### 13.3 Planned Authentication Capabilities

Future work includes:

- Interactive login.
- Backend authentication integration.
- Secure local credential storage.
- Session validation.
- Token expiration handling.
- Logout functionality.
- Authentication status reporting.

Local configuration generation should remain available without authentication.

---

## 14. Testing Strategy

### 14.1 Overview

DevInit uses automated tests to verify configuration handling, validation, generation, and filesystem behavior.

The v0.0.2 development baseline contains:

| Test Category | Count |
|---|---:|
| Unit Tests | 121 |
| Integration Tests | 16 |
| Total | **137** |

These counts describe the verified development baseline and may increase in later revisions.

### 14.2 Unit Tests

Unit tests are organized into separate files under:

```text
src/tests/
```

They are connected to their production modules using Rust conditional compilation:

```rust
#[cfg(test)]
#[path = "../tests/project_validator_tests.rs"]
mod tests;
```

This preserves access to private module functions without embedding large test implementations directly in production source files.

### 14.3 Integration Tests

Integration tests are located under:

```text
tests/
```

The main integration test file is:

```text
tests/project_tests.rs
```

These tests verify application behavior across multiple components.

### 14.4 Tested Areas

Automated coverage includes:

- Service parsing and validation.
- Built-in service resolution.
- Supported version handling.
- Dependency graph validation.
- Docker Compose generation.
- Kubernetes generation.
- CI/CD workflow generation.
- Reusable preset expansion.
- Invalid configuration rejection.
- Existing-file protection.
- GenerationPlan behavior.
- Path traversal and symlink protection.
- CLI check and dry-run behavior.
- CI/CD circular dependency detection.

### 14.5 End-to-End Validation

Manual end-to-end testing has also been used to verify the actual CLI behavior.

Verified scenarios include:

- Rejecting invalid Docker Compose port mappings.
- Rejecting Kubernetes Deployment selector mismatches.
- Preventing file creation after configuration validation failure.
- Successfully validating valid Kubernetes manifests.
- Generating multiple Kubernetes output files.
- Confirming that `check` and `--dry-run` do not write files.

These checks complement automated tests but do not constitute exhaustive cross-platform validation.

### 14.6 Development Verification

Before merging changes, run:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

All checks should pass before publishing a release.

---

## 15. Configuration Documentation

Configuration syntax is documented separately to avoid duplicating the complete schema in this technical reference.

### 15.1 User Guide

**File:** `docs/USER_GUIDE.md`

Contains installation instructions, basic commands, examples, and common troubleshooting scenarios.

### 15.2 Configuration Reference

**File:** `docs/CONFIG_REFERENCE.md`

Documents supported `compile.yaml` structures, input properties, configuration values, and relevant limitations.

### 15.3 AI Instructions

**File:** `docs/AI_INSTRUCTIONS.md`

Provides guidelines for AI assistants generating DevInit-compatible configurations.

It is intended to reduce unsupported configuration fields, invented service identifiers, and invalid YAML structures.

AI-generated configurations should always be validated using `devinit check`.

### 15.4 Technical Documentation

**File:** `docs/PROJECT_DOCUMENTATION.md`

This document describes the architecture, responsibilities, implementation structure, and extension points of the CLI.

---

## 16. Extending DevInit

### 16.1 Adding a Built-in Service

A new built-in service typically requires:

1. Defining the service identifier and its supported versions.
2. Adding the relevant recipe definition.
3. Updating the registry and resolution behavior.
4. Validating supported configuration properties.
5. Adding unit tests.
6. Updating the configuration reference.

Service definitions should follow the existing offline-first registry design.

### 16.2 Adding a New Generator

A new infrastructure generator should:

1. Accept supported resolved configuration data.
2. Validate any target-specific requirements.
3. Render configuration output without immediately modifying files.
4. Register its artifacts with `GenerationPlan`.
5. Reuse centralized filesystem safety checks.
6. Include unit and integration tests.
7. Document output paths and supported configuration fields.

A new generator should not bypass the existing generation plan unless an architectural change has been explicitly designed and reviewed.

### 16.3 Adding Validation Rules

New validation rules should:

- Identify unsupported or invalid configurations clearly.
- Produce actionable error messages.
- Preserve previously valid input where appropriate.
- Include both positive and negative test cases.
- Avoid silently modifying input configuration.

### 16.4 Modifying the CLI Interface

When adding a new command:

1. Register it in `cli/commands.rs`.
2. Implement command execution in the appropriate CLI logic module.
3. Reuse existing error types where practical.
4. Add relevant tests.
5. Update command documentation.
6. Ensure the new command does not unintentionally affect offline generation.

---

## 17. Known Limitations

DevInit v0.0.2 has several documented limitations.

### Configuration Validation

The custom validators do not implement the complete Docker Compose, Kubernetes, GitHub Actions, or GitLab CI/CD specifications.

### Dockerfile Integration

The Dockerfile discovery utility is not yet connected to automatic Compose build generation.

### Filesystem Transactions

Multi-file generation does not guarantee complete atomicity or automatic rollback.

### Authentication

Secure interactive CLI authentication is not fully implemented.

### Remote Infrastructure

Backend-dependent registry and configuration distribution capabilities remain under development.

### Infrastructure Execution

DevInit generates configuration files but does not automatically execute deployments, build containers, or provision cloud resources.

### Cross-Platform Behavior

Linux development and automated testing are established. Windows packaging and platform-specific runtime behavior require continued validation as part of release engineering.

---

## 18. Development Roadmap

### 18.1 Configuration Safety

Strengthen file protection and improve recovery behavior during partial generation failures.

### 18.2 Dockerfile Integration

Connect Dockerfile discovery with Docker Compose generation and support explicit build configuration.

### 18.3 Expanded Service Registry

Add more built-in service recipes, supported versions, and reusable infrastructure abstractions.

### 18.4 CLI Authentication

Complete secure account authentication and backend session integration.

### 18.5 Machine-Readable Configuration Schema

Introduce a JSON Schema for `compile.yaml` to improve editor validation, automated verification, and AI-assisted configuration generation.

### 18.6 Configuration Versioning and Distribution

Develop additional mechanisms for distributing and managing reusable infrastructure configurations through the wider DevInit platform.

These roadmap items are planned capabilities rather than completed v0.0.2 features.

---

## 19. Development and Contribution Guidelines

DevInit follows a test-driven verification approach for changes affecting core configuration behavior.

Contributors should:

- Preserve existing CLI commands unless a breaking change is intentional.
- Avoid unnecessary architectural changes.
- Keep configuration parsing, validation, generation, and writing responsibilities separate.
- Add automated tests for new functionality.
- Preserve existing user files by default.
- Keep public documentation synchronized with implementation.
- Avoid introducing mandatory backend dependencies into offline generation.
- Document known limitations and unsupported configuration patterns.

Feature requests and bug reports can be submitted through the project's GitHub Issues:

https://github.com/GorkiyChocolate/DevInit-CLI/issues

---

## 20. Project Background

DevInit CLI is part of the DevInit diploma project developed at Astana IT University.

The academic project is titled:

**Design and Development of a CLI-Oriented Platform for Infrastructure Configuration Management with Declarative Generation, Versioning, and Distribution Support.**

The broader DevInit system consists of:

- A Rust-based CLI for configuration processing and generation.
- A backend service for configuration management and distribution.
- A frontend interface for interacting with the platform.

The CLI is designed to function independently for supported local configuration workflows.

---

## 21. Conclusion

DevInit provides a modular foundation for declarative infrastructure configuration management.

Its central architecture separates configuration loading, service resolution, validation, intermediate representation, generation planning, and filesystem writing.

Version 0.0.2 establishes several important capabilities:

- Declarative multi-target configuration generation.
- Built-in infrastructure recipes and reusable presets.
- Dependency-aware validation.
- Kubernetes and CI/CD configuration support.
- Centralized generation planning.
- Safe-by-default file handling.
- Read-only validation and preview commands.
- Automated regression testing.

Future development will focus on improving configuration safety, expanding supported infrastructure abstractions, integrating Dockerfile discovery, and providing secure remote configuration management.

**DevInit CLI — Define once. Validate safely. Generate with confidence.**
