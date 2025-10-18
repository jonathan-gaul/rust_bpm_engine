
use bpm_engine::persistence;
use bpm_engine::{ProcessBuilder, ProcessEngine};
use std::env;

#[tokio::main]
async fn main() {
    // Example 0: Async start using a real Postgres store (assumes DATABASE_URL is set)
    if let Ok(database_url) = env::var("DATABASE_URL") {
        println!("--- Example 0: Async start with SQLx store ---");
        match persistence::sqlx::SqlxPostgresStore::new(&database_url).await {
            Ok(store) => {
                let store = std::sync::Arc::new(store);
                let mut engine = ProcessEngine::with_async_store(store);

                let process = ProcessBuilder::new("async_example")
                    .add_start("start")
                    .add_task("task", "Async Task")
                    .add_end("end")
                    .connect("start", "task")
                    .connect("task", "end")
                    .build()
                    .expect("Failed to build process");

                // Persist the definition via the async store before starting
                if let Err(e) = engine.deploy(process).await {
                    println!("Failed to persist definition: {}", e);
                }

                match engine.start_process("async_example").await {
                    Ok(id) => println!("✓ Async instance {} completed", id),
                    Err(e) => println!("Error starting async instance: {}", e),
                }
            }
            Err(e) => println!("Failed to create SQLx store: {}", e),
        }
    }
}