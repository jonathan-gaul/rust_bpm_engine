use bpm_engine::{ProcessBuilder, ProcessEngine};

#[tokio::main]
async fn main() {
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

    let mut engine = ProcessEngine::new();
    if let Err(e) = engine.deploy(process4).await {
        println!("Error deploying process4: {}", e);
    }

    match engine.start_process("command_process").await {
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
}