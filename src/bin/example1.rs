
use bpm_engine::{ProcessBuilder, ProcessEngine};

#[tokio::main]
async fn main() {
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
    if let Err(e) = engine.deploy(process1).await {
        println!("Error deploying process1: {}", e);
    }

    match engine.start_process("simple_process").await {
        Ok(instance_id) => {
            println!("✓ Instance {} completed\n", instance_id);
        }
        Err(e) => println!("Error: {}\n", e),
    }
}