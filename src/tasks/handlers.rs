
use crate::tasks::context::TaskContext;
use crate::tasks::config::TaskConfig;
use crate::tasks::output::TaskOutput;

// Task handler trait
pub trait TaskHandler {
    fn execute(&self, context: &TaskContext, config: &TaskConfig) -> Result<TaskOutput, String>;
}

#[derive(Debug, Clone)]
pub enum TaskHandlerType {
    Command,
    Http,
    Docker,
    Function, // For testing/simple cases
}

pub mod command;
pub mod function;