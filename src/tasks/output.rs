
use std::collections::HashMap;

// Result from task execution
pub struct TaskOutput {
    pub variables: HashMap<String, String>,
    pub success: bool,
    pub message: Option<String>,
}
