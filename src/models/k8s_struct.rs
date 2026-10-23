use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines a Kubernetes Deployment resource.
pub struct Deployment {
    /// Stores the Kubernetes API version.
    pub api_version: String,
    /// Stores the resource kind.
    pub kind: String,
    /// Identifies the deployment metadata.
    pub metadata: ObjectMeta,
    /// Defines the desired deployment state.
    pub spec: DeploymentSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Identifies a Kubernetes object.
pub struct ObjectMeta {
    /// Stores the object name.
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally stores the namespace.
    pub namespace: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally stores object labels.
    pub labels: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally stores object annotations.
    pub annotations: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines the desired state of a Deployment.
pub struct DeploymentSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sets the desired replica count.
    pub replicas: Option<u32>,

    /// Selects pods managed by the deployment.
    pub selector: LabelSelector,

    /// Defines the pod template to create.
    pub template: PodTemplateSpec,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally defines the rollout strategy.
    pub strategy: Option<DeploymentStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sets the minimum ready duration in seconds.
    pub min_ready_seconds: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Limits retained rollout revisions.
    pub revision_history_limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sets the rollout progress deadline in seconds.
    pub progress_deadline_seconds: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Pauses rollout processing when enabled.
    pub paused: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines how a deployment rollout is performed.
pub struct DeploymentStrategy {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    /// Stores the strategy name, such as RollingUpdate.
    pub strategy_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Defines rolling update limits.
    pub rolling_update: Option<RollingUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines limits for a rolling update.
pub struct RollingUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Limits unavailable replicas during an update.
    pub max_unavailable: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Limits extra replicas during an update.
    pub max_surge: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Selects objects by matching labels.
pub struct LabelSelector {
    /// Stores required label pairs.
    pub match_labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Defines pod metadata and specification.
pub struct PodTemplateSpec {
    /// Stores pod template metadata.
    pub metadata: ObjectMeta,
    /// Stores the pod template specification.
    pub spec: PodSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines how a pod is scheduled and run.
pub struct PodSpec {
    /// Lists containers required by the pod.
    pub containers: Vec<Container>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally lists initialization containers.
    pub init_containers: Option<Vec<Container>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally lists volumes available to containers.
    pub volumes: Option<Vec<Volume>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally defines the pod restart policy.
    pub restart_policy: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sets the pod termination grace period.
    pub termination_grace_period_seconds: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Selects the pod service account.
    pub service_account_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Lists image pull secret references.
    pub image_pull_secrets: Option<Vec<LocalObjectReference>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Restricts scheduling to matching node labels.
    pub node_selector: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Defines pod-level security settings.
    pub security_context: Option<PodSecurityContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines one container in a pod.
pub struct Container {
    /// Stores the container name.
    pub name: String,
    /// Stores the container image.
    pub image: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Controls when the image is pulled.
    pub image_pull_policy: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Defines the container entrypoint.
    pub command: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Defines arguments passed to the entrypoint.
    pub args: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sets the container working directory.
    pub working_dir: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Lists ports exposed by the container.
    pub ports: Option<Vec<ContainerPort>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Defines container environment variables.
    pub env: Option<Vec<EnvVar>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Imports environment variables from resources.
    pub env_from: Option<Vec<EnvFromSource>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Defines resource requests and limits.
    pub resources: Option<ResourceRequirements>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Lists mounted volumes.
    pub volume_mounts: Option<Vec<VolumeMount>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Defines the liveness health check.
    pub liveness_probe: Option<Probe>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Defines the readiness health check.
    pub readiness_probe: Option<Probe>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Defines the startup health check.
    pub startup_probe: Option<Probe>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Defines container-level security settings.
    pub security_context: Option<SecurityContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines a container network port.
pub struct ContainerPort {
    /// Stores the container port number.
    pub container_port: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally names the port.
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally selects the network protocol.
    pub protocol: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally publishes a host port.
    pub host_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines one container environment variable.
pub struct EnvVar {
    /// Stores the variable name.
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stores a literal variable value.
    pub value: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// References a value supplied by Kubernetes.
    pub value_from: Option<EnvVarSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines a source for an environment variable.
pub struct EnvVarSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// References a ConfigMap key.
    pub config_map_key_ref: Option<KeySelector>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// References a Secret key.
    pub secret_key_ref: Option<KeySelector>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// References a field in the pod metadata.
    pub field_ref: Option<ObjectFieldSelector>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Selects a named key from a resource.
pub struct KeySelector {
    /// Stores the referenced resource name.
    pub name: String,
    /// Stores the referenced key.
    pub key: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Allows the reference to be absent when enabled.
    pub optional: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Selects a field from an object.
pub struct ObjectFieldSelector {
    /// Stores the selected field path.
    pub field_path: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally specifies the API version.
    pub api_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Defines a source for importing environment variables.
pub struct EnvFromSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Prefixes imported variable names.
    pub prefix: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// References a ConfigMap source.
    pub config_map_ref: Option<ConfigMapEnvSource>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// References a Secret source.
    pub secret_ref: Option<SecretEnvSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Defines a ConfigMap environment source.
pub struct ConfigMapEnvSource {
    /// Stores the ConfigMap name.
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Allows the ConfigMap to be absent when enabled.
    pub optional: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Defines a Secret environment source.
pub struct SecretEnvSource {
    /// Stores the Secret name.
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Allows the Secret to be absent when enabled.
    pub optional: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Defines container resource requests and limits.
pub struct ResourceRequirements {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stores requested resource amounts.
    pub requests: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stores maximum resource amounts.
    pub limits: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines a volume mount inside a container.
pub struct VolumeMount {
    /// Identifies the mounted volume.
    pub name: String,
    /// Stores the mount path inside the container.
    pub mount_path: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally selects a subdirectory of the volume.
    pub sub_path: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Makes the mount read-only when enabled.
    pub read_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines a volume available to a pod.
pub struct Volume {
    /// Stores the volume name.
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally uses an empty directory volume.
    pub empty_dir: Option<EmptyDirVolumeSource>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally uses a ConfigMap volume.
    pub config_map: Option<ConfigMapVolumeSource>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally uses a Secret volume.
    pub secret: Option<SecretVolumeSource>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally uses a persistent volume claim.
    pub persistent_volume_claim: Option<PersistentVolumeClaimVolumeSource>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally mounts a host path.
    pub host_path: Option<HostPathVolumeSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines an empty directory volume.
pub struct EmptyDirVolumeSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Selects the storage medium.
    pub medium: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Limits the volume size.
    pub size_limit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines a ConfigMap-backed volume.
pub struct ConfigMapVolumeSource {
    /// Stores the ConfigMap name.
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sets the default file mode.
    pub default_mode: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Allows the ConfigMap to be absent when enabled.
    pub optional: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines a Secret-backed volume.
pub struct SecretVolumeSource {
    /// Stores the Secret name.
    pub secret_name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sets the default file mode.
    pub default_mode: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Allows the Secret to be absent when enabled.
    pub optional: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines a persistent volume claim mount.
pub struct PersistentVolumeClaimVolumeSource {
    /// Stores the claim name.
    pub claim_name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Mounts the claim as read-only when enabled.
    pub read_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Defines a host path volume.
pub struct HostPathVolumeSource {
    /// Stores the host path.
    pub path: String,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    /// Describes the host path type.
    pub path_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines a container health probe.
pub struct Probe {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Uses an HTTP health check.
    pub http_get: Option<HttpGetAction>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Uses a TCP socket health check.
    pub tcp_socket: Option<TcpSocketAction>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Uses a command health check.
    pub exec: Option<ExecAction>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Uses a gRPC health check.
    pub grpc: Option<GrpcAction>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Delays the first probe attempt.
    pub initial_delay_seconds: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sets the probe interval.
    pub period_seconds: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sets the probe timeout.
    pub timeout_seconds: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sets the number of successes required.
    pub success_threshold: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sets the number of failures allowed.
    pub failure_threshold: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Defines an HTTP health check action.
pub struct HttpGetAction {
    /// Stores the HTTP request path.
    pub path: String,
    /// Stores the target port.
    pub port: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally sets the request host.
    pub host: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally selects HTTP or HTTPS.
    pub scheme: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Defines a TCP health check action.
pub struct TcpSocketAction {
    /// Stores the target port.
    pub port: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally sets the target host.
    pub host: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Defines a command health check action.
pub struct ExecAction {
    /// Stores the command and its arguments.
    pub command: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Defines a gRPC health check action.
pub struct GrpcAction {
    /// Stores the gRPC port.
    pub port: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optionally selects a gRPC service.
    pub service: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines security settings for a container.
pub struct SecurityContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sets the user ID for the container process.
    pub run_as_user: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sets the group ID for the container process.
    pub run_as_group: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Requires the process to run as non-root.
    pub run_as_non_root: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Grants privileged container access when enabled.
    pub privileged: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Makes the root filesystem read-only when enabled.
    pub read_only_root_filesystem: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Controls privilege escalation for the process.
    pub allow_privilege_escalation: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Defines security settings inherited by pod containers.
pub struct PodSecurityContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sets the default user ID.
    pub run_as_user: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sets the default group ID.
    pub run_as_group: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Requires pod processes to run as non-root.
    pub run_as_non_root: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Sets the filesystem group ID.
    pub fs_group: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// References another Kubernetes object by name.
pub struct LocalObjectReference {
    /// Stores the referenced object name.
    pub name: String,
}
