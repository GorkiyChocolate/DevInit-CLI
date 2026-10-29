use crate::errors::ValidationError;
use crate::models::cicd_struct::Pipeline;
use std::collections::{HashMap, HashSet};

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
    validate_job_cycles(cicd, errors);
}

/// Detects circular dependencies between CI/CD jobs.
fn validate_job_cycles(cicd: &Pipeline, errors: &mut Vec<ValidationError>) {
    let indexes: HashMap<&str, usize> = cicd
        .jobs
        .iter()
        .enumerate()
        .map(|(index, job)| (job.name.as_str(), index))
        .collect();

    let mut states = vec![0u8; cicd.jobs.len()];
    let mut stack = Vec::new();

    for index in 0..cicd.jobs.len() {
        if states[index] == 0 {
            visit_job(index, cicd, &indexes, &mut states, &mut stack, errors);
        }
    }
}

fn visit_job(
    index: usize,
    cicd: &Pipeline,
    indexes: &HashMap<&str, usize>,
    states: &mut [u8],
    stack: &mut Vec<usize>,
    errors: &mut Vec<ValidationError>,
) {
    states[index] = 1;
    stack.push(index);

    for dependency in &cicd.jobs[index].needs {
        let Some(&next) = indexes.get(dependency.as_str()) else {
            // Missing dependencies are reported by the existing validator.
            continue;
        };

        match states[next] {
            0 => {
                visit_job(next, cicd, indexes, states, stack, errors);
            }

            1 => {
                if let Some(position) = stack.iter().position(|&node| node == next) {
                    let mut cycle: Vec<&str> = stack[position..]
                        .iter()
                        .map(|&node| cicd.jobs[node].name.as_str())
                        .collect();

                    cycle.push(cicd.jobs[next].name.as_str());

                    errors.push(ValidationError {
                        path: format!("cicd.jobs[{index}].needs"),
                        message: format!(
                            "circular job dependency detected: {}",
                            cycle.join(" -> ")
                        ),
                    });
                }
            }

            _ => {}
        }
    }

    stack.pop();
    states[index] = 2;
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

#[cfg(test)]
#[path = "../tests/cicd_validator_tests.rs"]
mod tests;
