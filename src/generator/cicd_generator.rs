use crate::errors::GenerationError;
use crate::file_config::yaml_writer::write_yaml;
use crate::models::cicd_struct::{CiProvider, Pipeline, Runner, StepAction, Trigger};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Generates a CI/CD file for the selected provider.
pub fn generate_cicd(
    pipeline: &Pipeline,
    output_dir: &Path,
) -> Result<Vec<PathBuf>, GenerationError> {
    // Select the provider-specific output format.
    match pipeline.provider {
        CiProvider::Github => generate_github_actions(pipeline, output_dir),
        CiProvider::Gitlab => generate_gitlab_ci(pipeline, output_dir),
    }
}

/// Generates a GitHub Actions workflow file.
fn generate_github_actions(
    pipeline: &Pipeline,
    output_dir: &Path,
) -> Result<Vec<PathBuf>, GenerationError> {
    let workflow = GithubWorkflow::from_pipeline(pipeline);
    let path = output_dir.join(".github/workflows/devinit.yml");
    Ok(vec![write_yaml(&workflow, &path)?])
}

/// Generates a GitLab CI configuration file.
fn generate_gitlab_ci(
    pipeline: &Pipeline,
    output_dir: &Path,
) -> Result<Vec<PathBuf>, GenerationError> {
    let workflow = GitlabPipeline::from_pipeline(pipeline);
    let path = output_dir.join(".gitlab-ci.yml");
    Ok(vec![write_yaml(&workflow, &path)?])
}

#[derive(Serialize)]
/// Represents a GitHub Actions workflow.
struct GithubWorkflow {
    /// Stores the workflow display name.
    name: &'static str,
    #[serde(rename = "on")]
    /// Stores event trigger definitions.
    triggers: BTreeMap<String, GithubTrigger>,
    /// Stores workflow jobs.
    jobs: BTreeMap<String, GithubJob>,
}

#[derive(Serialize)]
#[serde(untagged)]
/// Represents a GitHub trigger with or without branch filters.
enum GithubTrigger {
    /// Restricts a trigger to selected branches.
    Branches {
        /// Stores the branch filters.
        branches: Vec<String>,
    },
    /// Represents a trigger without additional settings.
    Empty(BTreeMap<String, String>),
}

#[derive(Serialize)]
/// Represents a GitHub Actions job.
struct GithubJob {
    /// Selects the runner image.
    #[serde(rename = "runs-on")]
    runs_on: String,
    /// Lists prerequisite jobs.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    needs: Vec<String>,
    /// Stores ordered job steps.
    steps: Vec<GithubStep>,
}

#[derive(Serialize)]
/// Represents one GitHub Actions step.
struct GithubStep {
    /// Stores the display name.
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally identifies an action to use.
    uses: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally stores a shell command.
    run: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    /// Stores step environment variables.
    env: BTreeMap<String, String>,
}

impl GithubWorkflow {
    /// Converts a generic pipeline into GitHub Actions data.
    fn from_pipeline(pipeline: &Pipeline) -> Self {
        let mut triggers = BTreeMap::new();
        // Convert each trigger into GitHub syntax.
        for trigger in &pipeline.triggers {
            match trigger {
                Trigger::Push { branches } => {
                    triggers.insert("push".to_string(), github_trigger(branches));
                }
                Trigger::PullRequest { branches } => {
                    triggers.insert("pull_request".to_string(), github_trigger(branches));
                }
                Trigger::Manual => {
                    triggers.insert(
                        "workflow_dispatch".to_string(),
                        GithubTrigger::Empty(BTreeMap::new()),
                    );
                }
            }
        }

        let jobs = pipeline
            .jobs
            .iter()
            .map(|job| {
                (
                    job.name.clone(),
                    GithubJob {
                        runs_on: runner_name(&job.runner).to_string(),
                        needs: job.needs.clone(),
                        steps: job.steps.iter().map(github_step).collect(),
                    },
                )
            })
            .collect();

        Self {
            name: "DevInit",
            triggers,
            jobs,
        }
    }
}

/// Converts branch filters into a GitHub trigger value.
fn github_trigger(branches: &[String]) -> GithubTrigger {
    // Use an empty trigger when no branch filters are supplied.
    if branches.is_empty() {
        GithubTrigger::Empty(BTreeMap::new())
    } else {
        GithubTrigger::Branches {
            branches: branches.to_vec(),
        }
    }
}

/// Maps an internal runner to its GitHub Actions name.
fn runner_name(runner: &Runner) -> &'static str {
    match runner {
        Runner::UbuntuLatest => "ubuntu-latest",
        Runner::WindowsLatest => "windows-latest",
        Runner::MacosLatest => "macos-latest",
    }
}

/// Converts an internal step into a GitHub Actions step.
fn github_step(step: &crate::models::cicd_struct::Step) -> GithubStep {
    let (uses, run) = match &step.action {
        StepAction::Checkout => (Some("actions/checkout@v4".to_string()), None),
        StepAction::Run { command } => (None, Some(command.clone())),
        StepAction::DockerBuild { dockerfile, image } => (
            None,
            Some(format!("docker build -f {dockerfile} -t {image} .")),
        ),
        StepAction::DockerPush { image } => (None, Some(format!("docker push {image}"))),
    };
    GithubStep {
        name: step.name.clone(),
        uses,
        run,
        env: step.env.clone(),
    }
}

#[derive(Serialize)]
/// Represents a GitLab pipeline document.
struct GitlabPipeline {
    /// Lists pipeline stages in execution order.
    stages: Vec<String>,
    #[serde(flatten)]
    /// Stores GitLab jobs by name.
    jobs: BTreeMap<String, GitlabJob>,
}

#[derive(Serialize)]
/// Represents one GitLab CI job.
struct GitlabJob {
    /// Stores the job stage.
    stage: String,
    /// Stores shell commands for the job.
    script: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    /// Lists prerequisite jobs.
    needs: Vec<String>,
}

impl GitlabPipeline {
    /// Converts a generic pipeline into GitLab CI data.
    fn from_pipeline(pipeline: &Pipeline) -> Self {
        let stages = pipeline.jobs.iter().map(|job| job.name.clone()).collect();
        let jobs = pipeline
            .jobs
            .iter()
            .map(|job| {
                (
                    job.name.clone(),
                    GitlabJob {
                        stage: job.name.clone(),
                        script: job.steps.iter().filter_map(gitlab_command).collect(),
                        needs: job.needs.clone(),
                    },
                )
            })
            .collect();
        Self { stages, jobs }
    }
}

/// Converts a pipeline step into a GitLab shell command.
fn gitlab_command(step: &crate::models::cicd_struct::Step) -> Option<String> {
    match &step.action {
        StepAction::Checkout => None,
        StepAction::Run { command } => Some(command.clone()),
        StepAction::DockerBuild { dockerfile, image } => {
            Some(format!("docker build -f {dockerfile} -t {image} ."))
        }
        StepAction::DockerPush { image } => Some(format!("docker push {image}")),
    }
}
