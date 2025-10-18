
use bpm_engine::{ProcessBuilder, ProcessEngine};

#[tokio::main]
async fn main() {
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

    let mut engine = ProcessEngine::new();
    if let Err(e) = engine.deploy(process5).await {
        println!("Error deploying process5: {}", e);
    }

    match engine.start_process("multi_var_process").await {
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