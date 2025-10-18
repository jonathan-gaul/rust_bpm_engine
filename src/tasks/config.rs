use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskConfig {
    // Command handler config
    pub command: Option<String>,
    pub args: Vec<String>,
    
    // HTTP handler config
    pub url: Option<String>,
    pub method: Option<String>,
    
    // Variable mapping
    pub input_mappings: HashMap<String, String>,  // process_var -> task_input
    pub output_mappings: HashMap<String, String>, // task_output -> process_var
}

impl TaskConfig {
    pub fn new() -> Self {
        TaskConfig {
            command: None,
            args: vec![],
            url: None,
            method: None,
            input_mappings: HashMap::new(),
            output_mappings: HashMap::new(),
        }
    }
}