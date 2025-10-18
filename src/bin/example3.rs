
use bpm_engine::{ProcessBuilder, ProcessEngine};

#[tokio::main]
async fn main() {
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

    let mut engine = ProcessEngine::new();
    if let Err(e) = engine.deploy(process3).await {
        println!("Error deploying process3: {}", e);
    }

    match engine.start_process("conditional_process").await {
        Ok(instance_id) => {
            println!("✓ Instance {} completed\n", instance_id);
        }
        Err(e) => println!("Error: {}\n", e),
    }
}