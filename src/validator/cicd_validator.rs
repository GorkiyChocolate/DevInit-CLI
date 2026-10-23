use crate::errors::ValidationError;
use crate::models::cicd_struct::Pipeline;
use std::collections::HashSet;

/// Validates job names, steps, commands, and job dependencies.
pub fn validate_pipeline(cicd: &Pipeline, errors: &mut Vec<ValidationError>) {
    let mut jobs = HashSet::new();
    // Check every job and its steps.
    for (job_index, job) in cicd.jobs.iter().enumerate() {
        let path = format!("cicd.jobs[{job_index}]");
        // Require a visible job name.
        if job.name.trim().is_empty() {
            errors.push(ValidationError {
                path: format!("{path}.name"),
                message: "cannot be empty".to_string(),
            });
        }
        // Require unique job names for dependency lookup.
        if !jobs.insert(job.name.as_str()) {
            errors.push(ValidationError {
                path: format!("{path}.name"),
                message: "job name must be unique".to_string(),
            });
        }
        // Validate every step belonging to the job.
        for (step_index, step) in job.steps.iter().enumerate() {
            let step_path = format!("{path}.steps[{step_index}]");
            // Require a visible step name.
            if step.name.trim().is_empty() {
                errors.push(ValidationError {
                    path: format!("{step_path}.name"),
                    message: "cannot be empty".to_string(),
                });
            }
            // Require commands for run actions.
            if let crate::models::cicd_struct::StepAction::Run { command } = &step.action
                && command.trim().is_empty()
            {
                errors.push(ValidationError {
                    path: format!("{step_path}.command"),
                    message: "cannot be empty".to_string(),
                });
            }
        }
    }
    // Validate every declared job dependency.
    for (job_index, job) in cicd.jobs.iter().enumerate() {
        // Check each referenced prerequisite job.
        for (need_index, need) in job.needs.iter().enumerate() {
            // Report references to unknown jobs.
            if !jobs.contains(need.as_str()) {
                errors.push(ValidationError {
                    path: format!("cicd.jobs[{job_index}].needs[{need_index}]"),
                    message: format!("job '{need}' does not exist"),
                });
            }
        }
    }
}

/// Returns an I/O error when the pipeline contains validation failures.
pub fn cicd_validator(cicd: &Pipeline) -> Result<(), std::io::Error> {
    let mut errors = Vec::new();
    validate_pipeline(cicd, &mut errors);
    // Accept the pipeline only when no rules failed.
    if errors.is_empty() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{} validation error(s)", errors.len()),
        ))
    }
}
