use bpm_engine::{ProcessBuilder, ProcessEngine, ProcessState};

// Test that a simple process with a placeholder task completes
#[test]
fn placeholder_task_completes() {
    let process = ProcessBuilder::new("test_simple")
        .add_start("start")
        .add_task("noop", "Noop Task")
        .add_end("end")
        .connect("start", "noop")
        .connect("noop", "end")
        .build()
        .expect("failed to build process");

    let mut engine = ProcessEngine::new();
    engine.deploy(process);

    let instance_id = engine.start_process("test_simple").expect("start failed");

    // After running, instance should be completed
    let state = engine.get_instance_state(&instance_id).expect("no state");
    assert_eq!(state, &ProcessState::Completed);
}

// Test that a command task can emit VAR:key=value which is mapped into process variables
#[test]
fn command_task_emits_vars_and_mapped() {
    // Use /bin/sh -c on unix-like systems
    #[cfg(not(target_os = "windows"))]
    let (cmd, args) = ("sh", vec!["-c", "echo VAR:mykey=hello_world; echo done"]);

    #[cfg(target_os = "windows")]
    let (cmd, args) = ("cmd", vec!["/C", "echo VAR:mykey=hello_world & echo done"]);

    let builder = ProcessBuilder::new("test_cmd")
        .add_start("start")
        .add_command_task("emit", "Emit Vars", cmd, args)
        .set_output_mapping("emit", "mykey", "mapped_key")
        .add_end("end")
        .connect("start", "emit")
        .connect("emit", "end");

    let process = builder.build().expect("build failed");

    let mut engine = ProcessEngine::new();
    engine.deploy(process);

    let instance_id = engine.start_process("test_cmd").expect("start failed");

    // Verify variable mapping applied
    let vars = engine.get_instance_variables(&instance_id).expect("no vars");
    let val = vars.get("mapped_key").expect("mapped_key not found");
    assert!(val.contains("hello_world"), "unexpected value: {}", val);
}

// Test parallel fork/join semantics: fork into two placeholder tasks and join back
#[test]
fn parallel_fork_join_completes() {
    let process = ProcessBuilder::new("test_parallel")
        .add_start("start")
        .add_parallel_gateway("fork")
        .add_task("t1", "Task 1")
        .add_task("t2", "Task 2")
        .add_parallel_gateway("join")
        .add_end("end")
        .connect("start", "fork")
        .connect("fork", "t1")
        .connect("fork", "t2")
        .connect("t1", "join")
        .connect("t2", "join")
        .connect("join", "end")
        .build()
        .expect("failed to build process");

    let mut engine = ProcessEngine::new();
    engine.deploy(process);

    let instance_id = engine.start_process("test_parallel").expect("start failed");

    let state = engine.get_instance_state(&instance_id).expect("no state");
    assert_eq!(state, &ProcessState::Completed);
}

#[test]
fn parallel_tokens_and_join_counters() {
    // Build a process with fork and join, same as previous test
    let process = ProcessBuilder::new("test_parallel2")
        .add_start("start")
        .add_parallel_gateway("fork")
        .add_task("t1", "Task 1")
        .add_task("t2", "Task 2")
        .add_parallel_gateway("join")
        .add_end("end")
        .connect("start", "fork")
        .connect("fork", "t1")
        .connect("fork", "t2")
        .connect("t1", "join")
        .connect("t2", "join")
        .connect("join", "end")
        .build()
        .expect("failed to build process");

    let mut engine = ProcessEngine::new();
    engine.deploy(process);

    let instance_id = engine.start_process("test_parallel2").expect("start failed");

    // After starting the instance the engine processes synchronously; expect completion
    assert_eq!(engine.get_instance_state(&instance_id).unwrap(), &ProcessState::Completed);

    // Active tokens should be 0 (process completed)
    assert_eq!(engine.get_active_tokens(&instance_id).unwrap_or(0), 0);

    // Join counter for the join node should not exist (cleared after join)
    assert!(engine.get_join_counter(&instance_id, "join").is_none());
}
