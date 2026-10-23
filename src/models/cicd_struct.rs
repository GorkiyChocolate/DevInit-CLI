use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Defines a CI/CD pipeline configuration.
pub struct Pipeline {
    /// Selects the target CI provider.
    pub provider: CiProvider,

    #[serde(default)]
    /// Lists events that start the pipeline.
    pub triggers: Vec<Trigger>,

    #[serde(default)]
    /// Contains jobs executed by the pipeline.
    pub jobs: Vec<Job>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Lists supported CI/CD providers.
pub enum CiProvider {
    /// Uses GitHub Actions.
    Github,
    /// Uses GitLab CI.
    Gitlab,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Describes an event that triggers a pipeline.
pub enum Trigger {
    Push {
        #[serde(default)]
        /// Restricts push events to these branches.
        branches: Vec<String>,
    },

    PullRequest {
        #[serde(default)]
        /// Restricts pull request events to these branches.
        branches: Vec<String>,
    },

    /// Allows a manually started pipeline.
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Defines one CI/CD job.
pub struct Job {
    /// Names the job uniquely within the pipeline.
    pub name: String,

    #[serde(default)]
    /// Selects the runner used by the job.
    pub runner: Runner,

    #[serde(default)]
    /// Lists jobs that must finish first.
    pub needs: Vec<String>,

    #[serde(default)]
    /// Contains the ordered job steps.
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
/// Lists supported CI runner images.
pub enum Runner {
    #[default]
    /// Uses the latest Ubuntu runner.
    UbuntuLatest,

    /// Uses the latest Windows runner.
    WindowsLatest,

    /// Uses the latest macOS runner.
    MacosLatest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Defines one executable pipeline step.
pub struct Step {
    /// Names the step for display and diagnostics.
    pub name: String,

    #[serde(flatten)]
    /// Specifies the operation performed by the step.
    pub action: StepAction,

    #[serde(default)]
    /// Provides environment variables for the step.
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
/// Lists operations supported by a pipeline step.
pub enum StepAction {
    /// Checks out the source repository.
    Checkout,

    /// Runs a shell command.
    Run {
        /// Contains the command to execute.
        command: String,
    },

    /// Builds a Docker image.
    DockerBuild {
        /// Identifies the Dockerfile used for the build.
        dockerfile: String,
        /// Names the image produced by the build.
        image: String,
    },

    /// Pushes a Docker image to a registry.
    DockerPush {
        /// Identifies the image to push.
        image: String,
    },
}
