
use bpm_engine::{ProcessBuilder, ProcessEngine};

#[tokio::main]
async fn main() {    
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

    let mut engine = ProcessEngine::new();
    if let Err(e) = engine.deploy(process2).await {
        println!("Error deploying process2: {}", e);
    }

    match engine.start_process("parallel_process").await {
        Ok(instance_id) => {
            println!("✓ Instance {} completed\n", instance_id);
        }
        Err(e) => println!("Error: {}\n", e),
    }
}