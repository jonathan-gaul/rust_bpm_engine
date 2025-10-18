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

    let mut builder = ProcessBuilder::new("test_cmd")
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
