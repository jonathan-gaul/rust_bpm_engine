use crate::tasks::context::TaskContext;
use crate::tasks::output::TaskOutput;
use crate::tasks::config::TaskConfig;
use crate::tasks::handlers::TaskHandler;

use std::collections::HashMap;

// Function task handler - executes closures (for testing)
pub struct FunctionTaskHandler {
    func: fn(&TaskContext) -> Result<HashMap<String, String>, String>,
}

impl TaskHandler for FunctionTaskHandler {
    fn execute(&self, context: &TaskContext, _config: &TaskConfig) -> Result<TaskOutput, String> {
        match (self.func)(context) {
            Ok(vars) => Ok(TaskOutput {
                variables: vars,
                success: true,
                message: None,
            }),
            Err(e) => Ok(TaskOutput {
                variables: HashMap::new(),
                success: false,
                message: Some(e),
            }),
        }
    }
}
