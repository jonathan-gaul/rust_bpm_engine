use std::collections::HashMap;

// Context passed to task handlers
pub struct TaskContext {
    pub variables: HashMap<String, String>,
}

impl TaskContext {
    pub fn get(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }
    
    pub fn set(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }
}