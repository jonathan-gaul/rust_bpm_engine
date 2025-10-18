use std::collections::HashMap;

mod tasks;
use tasks::node::Node;
use tasks::node::NodeType;
use tasks::context::TaskContext;
use tasks::config::TaskConfig;
use tasks::handlers::TaskHandler;
use tasks::handlers::TaskHandlerType;

use tasks::handlers::command::CommandTaskHandler;

// Process definition - the blueprint
#[derive(Debug)]
struct ProcessDefinition {
    id: String,
    nodes: HashMap<String, Node>,
    start_node_id: String,
}

// Process instance - a running execution
#[derive(Debug)]
struct ProcessInstance {
    id: String,
    process_def_id: String,
    current_node_ids: Vec<String>, // Multiple for parallel execution
    state: ProcessState,
    variables: HashMap<String, String>,
    active_tokens: usize, // For tracking parallel paths
    join_counters: HashMap<String, usize>, // Track how many tokens arrived at each join
}

#[derive(Debug, PartialEq)]
enum ProcessState {
    Running,
    Completed,
    Failed,
}

// The execution engine
struct ProcessEngine {
    definitions: HashMap<String, ProcessDefinition>,
    instances: HashMap<String, ProcessInstance>,
}

impl ProcessEngine {
    fn new() -> Self {
        ProcessEngine {
            definitions: HashMap::new(),
            instances: HashMap::new(),
        }
    }

    fn deploy(&mut self, definition: ProcessDefinition) {
        println!("Deploying process: {}", definition.id);
        self.definitions.insert(definition.id.clone(), definition);
    }

    fn start_process(&mut self, process_def_id: &str) -> Result<String, String> {
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
        
        // Execute from start
        self.execute_instance(&instance_id)?;
        
        Ok(instance_id)
    }

    fn execute_instance(&mut self, instance_id: &str) -> Result<(), String> {
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

    fn advance_token(&mut self, instance_id: &str, next_nodes: &[String]) -> Result<(), String> {
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

    fn get_instance_state(&self, instance_id: &str) -> Option<&ProcessState> {
        self.instances.get(instance_id).map(|i| &i.state)
    }
}

// Helper to build process definitions
struct ProcessBuilder {
    id: String,
    nodes: HashMap<String, Node>,
    start_node_id: Option<String>,
}

impl ProcessBuilder {
    fn new(id: &str) -> Self {
        ProcessBuilder {
            id: id.to_string(),
            nodes: HashMap::new(),
            start_node_id: None,
        }
    }

    fn add_start(mut self, id: &str) -> Self {
        self.start_node_id = Some(id.to_string());
        self.nodes.insert(id.to_string(), Node {
            id: id.to_string(),
            node_type: NodeType::Start,
            outgoing: vec![],
            conditions: HashMap::new(),
        });
        self
    }

    fn add_task(mut self, id: &str, name: &str) -> Self {
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
    
    fn add_command_task(mut self, id: &str, name: &str, command: &str, args: Vec<&str>) -> Self {
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
    
    fn set_output_mapping(mut self, task_id: &str, task_var: &str, process_var: &str) -> Self {
        if let Some(node) = self.nodes.get_mut(task_id) {
            if let NodeType::Task { config, .. } = &mut node.node_type {
                config.output_mappings.insert(task_var.to_string(), process_var.to_string());
            }
        }
        self
    }

    fn add_exclusive_gateway(mut self, id: &str) -> Self {
        self.nodes.insert(id.to_string(), Node {
            id: id.to_string(),
            node_type: NodeType::ExclusiveGateway,
            outgoing: vec![],
            conditions: HashMap::new(),
        });
        self
    }

    fn add_parallel_gateway(mut self, id: &str) -> Self {
        self.nodes.insert(id.to_string(), Node {
            id: id.to_string(),
            node_type: NodeType::ParallelGateway,
            outgoing: vec![],
            conditions: HashMap::new(),
        });
        self
    }

    fn add_end(mut self, id: &str) -> Self {
        self.nodes.insert(id.to_string(), Node {
            id: id.to_string(),
            node_type: NodeType::End,
            outgoing: vec![],
            conditions: HashMap::new(),
        });
        self
    }

    fn connect(mut self, from: &str, to: &str) -> Self {
        if let Some(node) = self.nodes.get_mut(from) {
            node.outgoing.push(to.to_string());
        }
        self
    }

    fn build(self) -> Result<ProcessDefinition, String> {
        let start_node_id = self.start_node_id
            .ok_or_else(|| "Process must have a start node".to_string())?;

        Ok(ProcessDefinition {
            id: self.id,
            nodes: self.nodes,
            start_node_id,
        })
    }
}

fn main() {
    println!("=== Simple BPM Engine PoC ===\n");

    // Example 1: Simple sequential process
    println!("--- Example 1: Sequential Process ---");
    let process1 = ProcessBuilder::new("simple_process")
        .add_start("start")
        .add_task("task1", "Prepare Data")
        .add_task("task2", "Process Data")
        .add_end("end")
        .connect("start", "task1")
        .connect("task1", "task2")
        .connect("task2", "end")
        .build()
        .expect("Failed to build process");

    let mut engine = ProcessEngine::new();
    engine.deploy(process1);

    match engine.start_process("simple_process") {
        Ok(instance_id) => {
            println!("✓ Instance {} completed\n", instance_id);
        }
        Err(e) => println!("Error: {}\n", e),
    }

    // Example 2: Parallel execution (fork-join)
    println!("--- Example 2: Parallel Execution ---");
    let process2 = ProcessBuilder::new("parallel_process")
        .add_start("start")
        .add_parallel_gateway("fork")
        .add_task("task1", "Send Email")
        .add_task("task2", "Update Database")
        .add_task("task3", "Log Activity")
        .add_parallel_gateway("join")
        .add_end("end")
        .connect("start", "fork")
        .connect("fork", "task1")
        .connect("fork", "task2")
        .connect("fork", "task3")
        .connect("task1", "join")
        .connect("task2", "join")
        .connect("task3", "join")
        .connect("join", "end")
        .build()
        .expect("Failed to build parallel process");

    engine.deploy(process2);

    match engine.start_process("parallel_process") {
        Ok(instance_id) => {
            println!("✓ Instance {} completed\n", instance_id);
        }
        Err(e) => println!("Error: {}\n", e),
    }

    // Example 3: Exclusive gateway (conditional branch)
    println!("--- Example 3: Conditional Branch ---");
    let process3 = ProcessBuilder::new("conditional_process")
        .add_start("start")
        .add_task("check_amount", "Check Amount")
        .add_exclusive_gateway("gateway")
        .add_task("approve", "Auto Approve")
        .add_task("review", "Manual Review")
        .add_end("end")
        .connect("start", "check_amount")
        .connect("check_amount", "gateway")
        .connect("gateway", "approve")
        .connect("gateway", "review")
        .connect("approve", "end")
        .connect("review", "end")
        .build()
        .expect("Failed to build conditional process");

    engine.deploy(process3);

    match engine.start_process("conditional_process") {
        Ok(instance_id) => {
            println!("✓ Instance {} completed\n", instance_id);
        }
        Err(e) => println!("Error: {}\n", e),
    }
    
    // Example 4: Command task with output variables
    println!("--- Example 4: Command Task with Variables ---");
    
    // Use cross-platform commands
    #[cfg(target_os = "windows")]
    let (list_cmd, list_args) = ("cmd", vec!["/C", "dir"]);
    #[cfg(not(target_os = "windows"))]
    let (list_cmd, list_args) = ("ls", vec!["-la"]);
    
    #[cfg(target_os = "windows")]
    let (echo_cmd, echo_args) = ("cmd", vec!["/C", "echo", "Task completed successfully"]);
    #[cfg(not(target_os = "windows"))]
    let (echo_cmd, echo_args) = ("echo", vec!["Task completed successfully"]);
    
    let process4 = ProcessBuilder::new("command_process")
        .add_start("start")
        .add_command_task("list_files", "List Files", list_cmd, list_args)
        .set_output_mapping("list_files", "stdout", "file_list")
        .set_output_mapping("list_files", "exit_code", "list_exit_code")
        .add_command_task("echo_test", "Echo Test", echo_cmd, echo_args)
        .set_output_mapping("echo_test", "stdout", "echo_output")
        .add_end("end")
        .connect("start", "list_files")
        .connect("list_files", "echo_test")
        .connect("echo_test", "end")
        .build()
        .expect("Failed to build command process");

    engine.deploy(process4);

    match engine.start_process("command_process") {
        Ok(instance_id) => {
            println!("✓ Instance {} completed", instance_id);
            if let Some(instance) = engine.instances.get(&instance_id) {
                println!("Final variables:");
                for (key, value) in &instance.variables {
                    println!("  {} = {}", key, value.lines().next().unwrap_or(value));
                }
                println!();
            }
        }
        Err(e) => println!("Error: {}\n", e),
    }
    
    // Example 5: Multiple variables from one task
    println!("--- Example 5: Setting Multiple Variables ---");
    
    #[cfg(target_os = "windows")]
    let (multi_cmd, multi_args) = ("cmd", vec!["/C", "echo Starting user fetch... & echo VAR:user_id=12345 & echo Fetching details... & echo VAR:user_name=John Doe & echo VAR:user_email=john@example.com & echo Done!"]);
    #[cfg(not(target_os = "windows"))]
    let (multi_cmd, multi_args) = ("sh", vec!["-c", "echo 'Starting user fetch...'; echo 'VAR:user_id=12345'; echo 'Fetching details...'; echo 'VAR:user_name=John Doe'; echo 'VAR:user_email=john@example.com'; echo 'Done!'"]);
    
    let process5 = ProcessBuilder::new("multi_var_process")
        .add_start("start")
        .add_command_task("fetch_user", "Fetch User Info", multi_cmd, multi_args)
        .set_output_mapping("fetch_user", "user_id", "user_id")
        .set_output_mapping("fetch_user", "user_name", "user_name")
        .set_output_mapping("fetch_user", "user_email", "user_email")
        .add_end("end")
        .connect("start", "fetch_user")
        .connect("fetch_user", "end")
        .build()
        .expect("Failed to build multi-var process");

    engine.deploy(process5);

    match engine.start_process("multi_var_process") {
        Ok(instance_id) => {
            println!("✓ Instance {} completed", instance_id);
            if let Some(instance) = engine.instances.get(&instance_id) {
                println!("User Information:");
                println!("  ID:    {}", instance.variables.get("user_id").unwrap_or(&"N/A".to_string()));
                println!("  Name:  {}", instance.variables.get("user_name").unwrap_or(&"N/A".to_string()));
                println!("  Email: {}", instance.variables.get("user_email").unwrap_or(&"N/A".to_string()));
            }
        }
        Err(e) => println!("Error: {}", e),
    }
}

// Note: This requires uuid crate. Add to Cargo.toml:
// [dependencies]
// uuid = { version = "1.0", features = ["v4"] }


// Process execution unit tests were moved to integration tests under `tests/`.