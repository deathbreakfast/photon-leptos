//! Experiment registry and runners.

mod knee;
mod registry;
mod runner;

pub use knee::{evaluate_step, DegradationThresholds, StepVerdict};
pub use registry::{find, status_label, ExperimentMeta, ExperimentStatus, REGISTRY};
pub use runner::{run_experiment, RunContext, RunOutcome};
