
#!/usr/bin/env python3

from pathlib import Path
import sys

# Find the DevInit repository root
candidate = (
    Path(sys.argv[1]).resolve()
    if len(sys.argv) > 1
    else Path.cwd().resolve()
)

if (candidate / "cli/src/cli/commands.rs").is_file():
    repo = candidate
elif (candidate / "src/cli/commands.rs").is_file():
    repo = candidate.parent
else:
    raise SystemExit(
        "Cannot find CLI sources. "
        "Run from DevInit/ or DevInit/cli."
    )

base = repo / "cli/src"
updates = {}


def edit(name, replacements):
    path = base / name
    source = path.read_text(encoding="utf-8")

    for old, new in replacements:
        count = source.count(old)

        expected = (
            2
            if name == "generator/cicd_generator.rs"
            and old == "write_yaml(&workflow, &path)?"
            else 1
        )

        if count != expected:
            raise RuntimeError(
                f"{path}: expected {expected} occurrence(s), "
                f"found {count}: {old[:80]!r}"
            )

        source = source.replace(old, new, expected)

    updates[path] = source


# 1. CLI --force flag
edit("cli/commands.rs", [
    (
        "use clap::{Arg, Command};",
        "use clap::{Arg, ArgAction, Command};"
    ),
    (
        'Command::new("compile").about("validate and generate compile.yaml")',
        '''Command::new("compile")
        .about("validate and generate compile.yaml")
        .arg(Arg::new("force")
            .long("force")
            .help("Overwrite existing generated files")
            .action(ArgAction::SetTrue))'''
    ),
])


# 2. CLI execution logic
edit("cli/cli_logic.rs", [
    (
        "    let spec = load_compile(input_path)?;",
        '''    compile_at_with_options(input_path, output_dir, false)
}

pub fn compile_at_with_options(
    input_path: &Path,
    output_dir: &Path,
    force: bool,
) -> Result<Vec<PathBuf>, DevinitError> {
    let spec = load_compile(input_path)?;'''
    ),
    (
        "Ok(generator::generate(&spec, output_dir)?)",
        "Ok(generator::generate_with_options(&spec, output_dir, force)?)"
    ),
    (
        'Some(("compile", _)) => match compile() {',
        '''Some(("compile", sub_matches)) => match compile_at_with_options(
            Path::new("compile.yaml"),
            Path::new("."),
            sub_matches.get_flag("force"),
        ) {'''
    ),
])


# 3. Generator preflight and force propagation
edit("generator.rs", [
    (
        "    let mut generated = Vec::new();",
        '''    generate_with_options(spec, output_dir, false)
}

pub fn generate_with_options(
    spec: &CompileSpec,
    output_dir: &Path,
    force: bool,
) -> Result<Vec<PathBuf>, GenerationError> {
    if !force {
        let mut targets = Vec::new();

        if !spec.services.is_empty() {
            targets.push(output_dir.join("docker-compose.yaml"));
        }

        if let Some(kubernetes) = &spec.kubernetes {
            targets.push(output_dir.join("k8s").join(format!(
                "{}-deployment.yaml",
                kubernetes.metadata.name
            )));
        }

        if let Some(cicd) = &spec.cicd {
            let filename = match cicd.provider {
                crate::models::cicd_struct::CiProvider::Github => {
                    ".github/workflows/devinit.yml"
                }
                crate::models::cicd_struct::CiProvider::Gitlab => {
                    ".gitlab-ci.yml"
                }
            };

            targets.push(output_dir.join(filename));
        }

        for target in targets {
            match target.symlink_metadata() {
                Ok(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        format!(
                            "Refusing to overwrite {} (use --force)",
                            target.display()
                        ),
                    ).into());
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.into()),
            }
        }
    }

    let mut generated = Vec::new();'''
    ),
    (
        "docker_compose_generator::generate_docker_compose(",
        "docker_compose_generator::generate_docker_compose_with_options("
    ),
    (
        "            output_dir,\n        )?);",
        "            output_dir,\n            force,\n        )?);"
    ),
    (
        "k8s_generator::generate_k8s(kubernetes, output_dir)?",
        "k8s_generator::generate_k8s_with_options(kubernetes, output_dir, force)?"
    ),
    (
        "cicd_generator::generate_cicd(cicd, output_dir)?",
        "cicd_generator::generate_cicd_with_options(cicd, output_dir, force)?"
    ),
])


# 4. Docker Compose generator
edit("generator/docker_compose_generator.rs", [
    (
        "use crate::file_config::yaml_writer::write_yaml;",
        "use crate::file_config::yaml_writer::write_yaml_with_options;"
    ),
    (
        "    let mut compose_services = BTreeMap::new();",
        '''    generate_docker_compose_with_options(
        services,
        output_dir,
        false,
    )
}

pub fn generate_docker_compose_with_options(
    services: &[CompileService],
    output_dir: &Path,
    force: bool,
) -> Result<Vec<PathBuf>, GenerationError> {
    let mut compose_services = BTreeMap::new();'''
    ),
    (
        'write_yaml(&compose, &output_dir.join("docker-compose.yaml"))?',
        'write_yaml_with_options(&compose, &output_dir.join("docker-compose.yaml"), force)?'
    ),
])


# 5. Kubernetes generator
edit("generator/k8s_generator.rs", [
    (
        "use crate::file_config::yaml_writer::write_yaml;",
        "use crate::file_config::yaml_writer::write_yaml_with_options;"
    ),
    (
        "    let path = output_dir\n",
        '''    generate_k8s_with_options(deployment, output_dir, false)
}

pub fn generate_k8s_with_options(
    deployment: &Deployment,
    output_dir: &Path,
    force: bool,
) -> Result<Vec<PathBuf>, GenerationError> {
    let path = output_dir
'''
    ),
    (
        "write_yaml(deployment, &path)?",
        "write_yaml_with_options(deployment, &path, force)?"
    ),
])


# 6. CI/CD generator
edit("generator/cicd_generator.rs", [
    (
        "use crate::file_config::yaml_writer::write_yaml;",
        "use crate::file_config::yaml_writer::write_yaml_with_options;"
    ),
    (
        "    // Select the provider-specific output format.",
        '''    generate_cicd_with_options(
        pipeline,
        output_dir,
        false,
    )
}

pub fn generate_cicd_with_options(
    pipeline: &Pipeline,
    output_dir: &Path,
    force: bool,
) -> Result<Vec<PathBuf>, GenerationError> {
    // Select the provider-specific output format.'''
    ),
    (
        "generate_github_actions(pipeline, output_dir)",
        "generate_github_actions(pipeline, output_dir, force)"
    ),
    (
        "generate_gitlab_ci(pipeline, output_dir)",
        "generate_gitlab_ci(pipeline, output_dir, force)"
    ),
    (
        "fn generate_github_actions(\n    pipeline: &Pipeline,\n    output_dir: &Path,",
        "fn generate_github_actions(\n    pipeline: &Pipeline,\n    output_dir: &Path,\n    force: bool,"
    ),
    (
        "fn generate_gitlab_ci(\n    pipeline: &Pipeline,\n    output_dir: &Path,",
        "fn generate_gitlab_ci(\n    pipeline: &Pipeline,\n    output_dir: &Path,\n    force: bool,"
    ),
    (
        "write_yaml(&workflow, &path)?",
        "write_yaml_with_options(&workflow, &path, force)?"
    ),
])


# 7. Apply all prepared modifications
# No files are written until every replacement is verified.
for path, contents in updates.items():
    path.write_text(contents, encoding="utf-8")
    print("Updated:", path.relative_to(repo))

print("\nPatch applied successfully.")
print("Next: cargo fmt && cargo check && cargo test")
