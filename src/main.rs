use bpm_engine::{ProcessBuilder, ProcessEngine};

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
            if let Some(vars) = engine.get_instance_variables(&instance_id) {
                println!("Final variables:");
                for (key, value) in vars {
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
            if let Some(vars) = engine.get_instance_variables(&instance_id) {
                println!("User Information:");
                println!("  ID:    {}", vars.get("user_id").unwrap_or(&"N/A".to_string()));
                println!("  Name:  {}", vars.get("user_name").unwrap_or(&"N/A".to_string()));
                println!("  Email: {}", vars.get("user_email").unwrap_or(&"N/A".to_string()));
            }
        }
        Err(e) => println!("Error: {}", e),
    }
}