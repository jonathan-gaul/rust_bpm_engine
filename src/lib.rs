use std::collections::HashMap;
use std::sync::Arc;
use crate::persistence::InstanceStore;

mod tasks;
pub mod persistence;
use tasks::node::Node;
use tasks::node::NodeType;
use tasks::context::TaskContext;
use tasks::config::TaskConfig;
use tasks::handlers::TaskHandler;
use tasks::handlers::TaskHandlerType;

use tasks::handlers::command::CommandTaskHandler;

// Process definition - the blueprint
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessDefinition {
    id: String,
    nodes: HashMap<String, Node>,
    start_node_id: String,
}

// Process instance - a running execution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessInstance {
    id: String,
    process_def_id: String,
    current_node_ids: Vec<String>, // Multiple for parallel execution
    state: ProcessState,
    variables: HashMap<String, String>,
    active_tokens: usize, // For tracking parallel paths
    join_counters: HashMap<String, usize>, // Track how many tokens arrived at each join
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub enum ProcessState {
    Running,
    Completed,
    Failed,
}

// The execution engine
pub struct ProcessEngine {
    definitions: HashMap<String, ProcessDefinition>,
    instances: HashMap<String, ProcessInstance>,
    async_store: Option<Arc<dyn InstanceStore>>,
}

impl ProcessEngine {
    pub fn new() -> Self {
        ProcessEngine {
            definitions: HashMap::new(),
            instances: HashMap::new(),
            async_store: None,
        }
    }

    pub fn with_async_store(store: Arc<dyn InstanceStore>) -> Self {
        ProcessEngine {
            definitions: HashMap::new(),
            instances: HashMap::new(),
            async_store: Some(store),
        }
    }

    pub fn deploy(&mut self, definition: ProcessDefinition) {
        println!("Deploying process: {}", definition.id);
        // Insert into in-memory definitions
        self.definitions.insert(definition.id.clone(), definition.clone());

        // If an async store is configured, persist the definition in the background
        if let Some(store) = &self.async_store {
            let store = store.clone();
            // Clone the definition for the background task
            let def = definition.clone();
            let _ = tokio::spawn(async move {
                let _ = store.save_definition(&def).await;
            });
        }
    }

    /// Async deploy: persist the definition via the async store and insert into memory.
    pub async fn deploy_async(&mut self, definition: ProcessDefinition) -> Result<(), String> {
        println!("Deploying process (async): {}", definition.id);
        // Insert into memory
        self.definitions.insert(definition.id.clone(), definition.clone());

        if let Some(store) = &self.async_store {
            store.save_definition(&definition).await?;
        }

        Ok(())
    }

    pub fn start_process(&mut self, process_def_id: &str) -> Result<String, String> {
        let definition = self.definitions.get(process_def_id)
            .ok_or_else(|| format!("Process definition '{}' not found", process_def_id))?;

        let instance_id = format!("instance_{}", uuid::Uuid::new_v4());
        let instance = ProcessInstance {
            id: instance_id.clone(),
            process_def_id: process_def_id.to_string(),
            current_node_ids: vec![definition.start_node_id.clone()],
            state: ProcessState::Running,
            variables: HashMap::new(),
            active_tokens: 1,
            join_counters: HashMap::new(),
        };

        println!("Starting process instance: {}", instance_id);
        self.instances.insert(instance_id.clone(), instance);
        // If async store is present, persist instance (fire-and-forget is OK here but we await)
        if let Some(store) = &self.async_store {
            let inst = self.instances.get(&instance_id).unwrap().clone();
            // spawn a background task to save (best-effort)
            let store = store.clone();
            let id = instance_id.clone();
            // We'll block here by using tokio::spawn; user must call start_process_async for full async behavior
            let _ = tokio::spawn(async move {
                let _ = store.save_instance(&inst).await;
                let _ = id;
            });
        }

        // Execute from start (sync)
        self.execute_instance(&instance_id)?;
        
        Ok(instance_id)
    }

    /// Async start: fully async and persists via configured async store if present
    pub async fn start_process_async(&mut self, process_def_id: &str) -> Result<String, String> {
        let definition = self.definitions.get(process_def_id)
            .ok_or_else(|| format!("Process definition '{}' not found", process_def_id))?;

        let instance_id = format!("instance_{}", uuid::Uuid::new_v4());
        let instance = ProcessInstance {
            id: instance_id.clone(),
            process_def_id: process_def_id.to_string(),
            current_node_ids: vec![definition.start_node_id.clone()],
            state: ProcessState::Running,
            variables: HashMap::new(),
            active_tokens: 1,
            join_counters: HashMap::new(),
        };

        println!("Starting process instance: {}", instance_id);
        self.instances.insert(instance_id.clone(), instance);

        // If an async store is configured, persist the instance. Process definitions
        // are persisted during `deploy` (so they exist before instances are created).
        if let Some(store) = &self.async_store {
            let inst = self.instances.get(&instance_id).unwrap().clone();
            store.save_instance(&inst).await?;
        }

        // Execute the instance asynchronously
        self.execute_instance_async(&instance_id).await?;

        Ok(instance_id)
    }

    pub fn execute_instance(&mut self, instance_id: &str) -> Result<(), String> {
        loop {
            let instance = self.instances.get(instance_id)
                .ok_or_else(|| format!("Instance '{}' not found", instance_id))?;

            if instance.state != ProcessState::Running {
                break;
            }

            if instance.current_node_ids.is_empty() {
                break;
            }

            // Process one token at a time
            let current_node_id = instance.current_node_ids[0].clone();
            let process_def_id = instance.process_def_id.clone();

            // Clone the node data we need before any mutable operations
            let (node_id, node_type, outgoing, incoming_count) = {
                let definition = self.definitions.get(&process_def_id)
                    .ok_or_else(|| "Process definition not found".to_string())?;

                let node = definition.nodes.get(&current_node_id)
                    .ok_or_else(|| format!("Node '{}' not found", current_node_id))?;

                // Count incoming edges for join detection
                let incoming = definition.nodes.values()
                    .filter(|n| n.outgoing.contains(&current_node_id))
                    .count();

                (node.id.clone(), node.node_type.clone(), node.outgoing.clone(), incoming)
            };

            println!("Executing node: {} ({:?})", node_id, node_type);

            // Remove current token
            let instance = self.instances.get_mut(instance_id).unwrap();
            instance.current_node_ids.remove(0);

            // Execute the node based on type
            match &node_type {
                NodeType::Start => {
                    println!("  -> Process started");
                    self.advance_token(instance_id, &outgoing)?;
                }
                NodeType::Task { name, handler_type, config } => {
                    println!("  -> Executing task: {}", name);
                    
                    // Check if this is a placeholder task (no handler configured)
                    let is_placeholder = match handler_type {
                        TaskHandlerType::Command => config.command.is_none(),
                        _ => true,
                    };
                    
                    if is_placeholder {
                        println!("    [PLACEHOLDER] No handler configured, skipping execution");
                        self.advance_token(instance_id, &outgoing)?;
                        continue;
                    }
                    
                    // Create task context with current variables
                    let task_context = TaskContext {
                        variables: self.instances.get(instance_id).unwrap().variables.clone(),
                    };
                    
                    // Execute based on handler type
                    let task_output = match handler_type {
                        TaskHandlerType::Command => {
                            CommandTaskHandler.execute(&task_context, config)?
                        }
                        TaskHandlerType::Function => {
                            return Err("Function handler not yet wired up".to_string());
                        }
                        _ => {
                            return Err(format!("Handler type {:?} not implemented", handler_type));
                        }
                    };
                    
                    // Apply output mappings to process variables
                    let instance = self.instances.get_mut(instance_id).unwrap();
                    for (task_var, process_var) in &config.output_mappings {
                        if let Some(value) = task_output.variables.get(task_var) {
                            let preview = if value.len() > 50 {
                                format!("{}... ({} bytes)", &value[..50], value.len())
                            } else {
                                value.clone()
                            };
                            println!("    [VAR] Setting {} = {}", process_var, preview.lines().next().unwrap_or(&preview));
                            instance.variables.insert(process_var.clone(), value.clone());
                        }
                    }
                    
                    if !task_output.success {
                        println!("    [ERROR] Task failed: {:?}", task_output.message);
                    }
                    
                    self.advance_token(instance_id, &outgoing)?;
                }
                NodeType::ExclusiveGateway => {
                    println!("  -> Exclusive gateway (taking first path)");
                    // Simple: take first outgoing. Real impl would evaluate conditions
                    if !outgoing.is_empty() {
                        let next = vec![outgoing[0].clone()];
                        self.advance_token(instance_id, &next)?;
                    }
                }
                NodeType::ParallelGateway => {
                    if outgoing.len() > 1 {
                        // Fork: create multiple tokens
                        println!("  -> Parallel gateway: FORK ({} paths)", outgoing.len());
                        self.advance_token(instance_id, &outgoing)?;
                    } else if incoming_count > 1 {
                        // Join: wait for all incoming tokens
                        let instance = self.instances.get_mut(instance_id).unwrap();
                        let counter = instance.join_counters.entry(node_id.clone()).or_insert(0);
                        *counter += 1;
                        
                        println!("  -> Parallel gateway: JOIN (token {}/{})", counter, incoming_count);
                        
                        if *counter == incoming_count {
                            // All tokens arrived, proceed
                            println!("  -> All paths converged, continuing");
                            instance.join_counters.remove(&node_id);
                            instance.active_tokens -= incoming_count - 1; // Merge tokens
                            self.advance_token(instance_id, &outgoing)?;
                        }
                        // Otherwise, just consume this token and wait
                    } else {
                        // Simple pass-through
                        println!("  -> Parallel gateway: PASS-THROUGH");
                        self.advance_token(instance_id, &outgoing)?;
                    }
                }
                NodeType::End => {
                    println!("  -> Process completed");
                    let instance = self.instances.get_mut(instance_id).unwrap();
                    instance.active_tokens -= 1;
                    if instance.active_tokens == 0 {
                        instance.state = ProcessState::Completed;
                    }
                }
            }
        }

        Ok(())
    }

    /// Async execute instance: mirrors execute_instance but persists via async_store after task output application
    pub async fn execute_instance_async(&mut self, instance_id: &str) -> Result<(), String> {
        loop {
            let instance = self.instances.get(instance_id)
                .ok_or_else(|| format!("Instance '{}' not found", instance_id))?;

            if instance.state != ProcessState::Running {
                break;
            }

            if instance.current_node_ids.is_empty() {
                break;
            }

            // Process one token at a time
            let current_node_id = instance.current_node_ids[0].clone();
            let process_def_id = instance.process_def_id.clone();

            // Clone the node data we need before any mutable operations
            let (node_id, node_type, outgoing, incoming_count) = {
                let definition = self.definitions.get(&process_def_id)
                    .ok_or_else(|| "Process definition not found".to_string())?;

                let node = definition.nodes.get(&current_node_id)
                    .ok_or_else(|| format!("Node '{}' not found", current_node_id))?;

                // Count incoming edges for join detection
                let incoming = definition.nodes.values()
                    .filter(|n| n.outgoing.contains(&current_node_id))
                    .count();

                (node.id.clone(), node.node_type.clone(), node.outgoing.clone(), incoming)
            };

            println!("Executing node: {} ({:?})", node_id, node_type);

            // Remove current token
            let instance = self.instances.get_mut(instance_id).unwrap();
            instance.current_node_ids.remove(0);

            // Execute the node based on type
            match &node_type {
                NodeType::Start => {
                    println!("  -> Process started");
                    self.advance_token(instance_id, &outgoing)?;
                }
                NodeType::Task { name, handler_type, config } => {
                    println!("  -> Executing task: {}", name);
                    
                    // Check if this is a placeholder task (no handler configured)
                    let is_placeholder = match handler_type {
                        TaskHandlerType::Command => config.command.is_none(),
                        _ => true,
                    };
                    
                    if is_placeholder {
                        println!("    [PLACEHOLDER] No handler configured, skipping execution");
                        self.advance_token(instance_id, &outgoing)?;
                        continue;
                    }
                    
                    // Create task context with current variables
                    let task_context = TaskContext {
                        variables: self.instances.get(instance_id).unwrap().variables.clone(),
                    };
                    
                    // Execute based on handler type
                    let task_output = match handler_type {
                        TaskHandlerType::Command => {
                            CommandTaskHandler.execute(&task_context, config)?
                        }
                        TaskHandlerType::Function => {
                            return Err("Function handler not yet wired up".to_string());
                        }
                        _ => {
                            return Err(format!("Handler type {:?} not implemented", handler_type));
                        }
                    };
                    
                    // Apply output mappings to process variables
                    let instance = self.instances.get_mut(instance_id).unwrap();
                    for (task_var, process_var) in &config.output_mappings {
                        if let Some(value) = task_output.variables.get(task_var) {
                            let preview = if value.len() > 50 {
                                format!("{}... ({} bytes)", &value[..50], value.len())
                            } else {
                                value.clone()
                            };
                            println!("    [VAR] Setting {} = {}", process_var, preview.lines().next().unwrap_or(&preview));
                            instance.variables.insert(process_var.clone(), value.clone());
                        }
                    }
                    
                    if !task_output.success {
                        println!("    [ERROR] Task failed: {:?}", task_output.message);
                    }

                    // Persist instance after task if store present
                    if let Some(store) = &self.async_store {
                        let inst = self.instances.get(instance_id).unwrap().clone();
                        store.update_instance(&inst).await?;
                    }
                    
                    self.advance_token(instance_id, &outgoing)?;
                }
                NodeType::ExclusiveGateway => {
                    println!("  -> Exclusive gateway (taking first path)");
                    // Simple: take first outgoing. Real impl would evaluate conditions
                    if !outgoing.is_empty() {
                        let next = vec![outgoing[0].clone()];
                        self.advance_token(instance_id, &next)?;
                    }
                }
                NodeType::ParallelGateway => {
                    if outgoing.len() > 1 {
                        // Fork: create multiple tokens
                        println!("  -> Parallel gateway: FORK ({} paths)", outgoing.len());
                        self.advance_token(instance_id, &outgoing)?;
                    } else if incoming_count > 1 {
                        // Join: wait for all incoming tokens
                        let instance = self.instances.get_mut(instance_id).unwrap();
                        let counter = instance.join_counters.entry(node_id.clone()).or_insert(0);
                        *counter += 1;
                        
                        println!("  -> Parallel gateway: JOIN (token {}/{})", counter, incoming_count);
                        
                        if *counter == incoming_count {
                            // All tokens arrived, proceed
                            println!("  -> All paths converged, continuing");
                            instance.join_counters.remove(&node_id);
                            instance.active_tokens -= incoming_count - 1; // Merge tokens
                            self.advance_token(instance_id, &outgoing)?;
                        }
                        // Otherwise, just consume this token and wait
                    } else {
                        // Simple pass-through
                        println!("  -> Parallel gateway: PASS-THROUGH");
                        self.advance_token(instance_id, &outgoing)?;
                    }
                }
                NodeType::End => {
                    println!("  -> Process completed");
                    let instance = self.instances.get_mut(instance_id).unwrap();
                    instance.active_tokens -= 1;
                    if instance.active_tokens == 0 {
                        instance.state = ProcessState::Completed;
                        if let Some(store) = &self.async_store {
                            let inst = instance.clone();
                            store.update_instance(&inst).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn advance_token(&mut self, instance_id: &str, next_nodes: &[String]) -> Result<(), String> {
        let instance = self.instances.get_mut(instance_id)
            .ok_or_else(|| "Instance not found".to_string())?;
        
        for next_id in next_nodes {
            instance.current_node_ids.push(next_id.clone());
        }
        
        // Adjust token count for parallel splits
        if next_nodes.len() > 1 {
            instance.active_tokens += next_nodes.len() - 1;
        }
        
        Ok(())
    }

    pub fn get_instance(&self, instance_id: &str) -> Option<&ProcessInstance> {
        self.instances.get(instance_id)
    }

    /// Return a reference to the variables map for an instance (read-only)
    pub fn get_instance_variables(&self, instance_id: &str) -> Option<&HashMap<String, String>> {
        self.instances.get(instance_id).map(|i| &i.variables)
    }

    /// Return the state of an instance
    pub fn get_instance_state(&self, instance_id: &str) -> Option<&ProcessState> {
        self.instances.get(instance_id).map(|i| &i.state)
    }

    /// Return the number of active tokens for an instance (for testing/inspection)
    pub fn get_active_tokens(&self, instance_id: &str) -> Option<usize> {
        self.instances.get(instance_id).map(|i| i.active_tokens)
    }

    /// Return the join counter for a specific node of an instance (if any)
    pub fn get_join_counter(&self, instance_id: &str, node_id: &str) -> Option<usize> {
        self.instances.get(instance_id).and_then(|i| i.join_counters.get(node_id).cloned())
    }
}

// Helper to build process definitions
#[derive(Debug, Clone)]
pub struct ProcessBuilder {
    pub id: String,
    pub nodes: HashMap<String, Node>,
    pub start_node_id: Option<String>,
}

impl ProcessBuilder {
    pub fn new(id: &str) -> Self {
        ProcessBuilder {
            id: id.to_string(),
            nodes: HashMap::new(),
            start_node_id: None,
        }
    }

    pub fn add_start(mut self, id: &str) -> Self {
        self.start_node_id = Some(id.to_string());
        self.nodes.insert(id.to_string(), Node {
            id: id.to_string(),
            node_type: NodeType::Start,
            outgoing: vec![],
            conditions: HashMap::new(),
        });
        self
    }

    pub fn add_task(mut self, id: &str, name: &str) -> Self {
        self.nodes.insert(id.to_string(), Node {
            id: id.to_string(),
            node_type: NodeType::Task { 
                name: name.to_string(),
                handler_type: TaskHandlerType::Command,
                config: TaskConfig::new(),
            },
            outgoing: vec![],
            conditions: HashMap::new(),
        });
        self
    }
    
    pub fn add_command_task(mut self, id: &str, name: &str, command: &str, args: Vec<&str>) -> Self {
        let mut config = TaskConfig::new();
        config.command = Some(command.to_string());
        config.args = args.iter().map(|s| s.to_string()).collect();
        
        self.nodes.insert(id.to_string(), Node {
            id: id.to_string(),
            node_type: NodeType::Task { 
                name: name.to_string(),
                handler_type: TaskHandlerType::Command,
                config,
            },
            outgoing: vec![],
            conditions: HashMap::new(),
        });
        self
    }
    
    pub fn set_output_mapping(mut self, task_id: &str, task_var: &str, process_var: &str) -> Self {
        if let Some(node) = self.nodes.get_mut(task_id) {
            if let NodeType::Task { config, .. } = &mut node.node_type {
                config.output_mappings.insert(task_var.to_string(), process_var.to_string());
            }
        }
        self
    }

    pub fn add_exclusive_gateway(mut self, id: &str) -> Self {
        self.nodes.insert(id.to_string(), Node {
            id: id.to_string(),
            node_type: NodeType::ExclusiveGateway,
            outgoing: vec![],
            conditions: HashMap::new(),
        });
        self
    }

    pub fn add_parallel_gateway(mut self, id: &str) -> Self {
        self.nodes.insert(id.to_string(), Node {
            id: id.to_string(),
            node_type: NodeType::ParallelGateway,
            outgoing: vec![],
            conditions: HashMap::new(),
        });
        self
    }

    pub fn add_end(mut self, id: &str) -> Self {
        self.nodes.insert(id.to_string(), Node {
            id: id.to_string(),
            node_type: NodeType::End,
            outgoing: vec![],
            conditions: HashMap::new(),
        });
        self
    }

    pub fn connect(mut self, from: &str, to: &str) -> Self {
        if let Some(node) = self.nodes.get_mut(from) {
            node.outgoing.push(to.to_string());
        }
        self
    }

    pub fn build(self) -> Result<ProcessDefinition, String> {
        let start_node_id = self.start_node_id
            .ok_or_else(|| "Process must have a start node".to_string())?;

        Ok(ProcessDefinition {
            id: self.id,
            nodes: self.nodes,
            start_node_id,
        })
    }
}

// Examples were moved back to `src/main.rs`. Library contains core engine and builders only.
