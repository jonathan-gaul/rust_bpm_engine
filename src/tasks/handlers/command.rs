use crate::tasks::context::TaskContext;
use crate::tasks::config::TaskConfig;
use crate::tasks::output::TaskOutput;
use crate::tasks::handlers::TaskHandler;

use std::process::Command;
use std::collections::HashMap;

// Command task handler - executes shell commands
pub struct CommandTaskHandler;

impl TaskHandler for CommandTaskHandler {
    fn execute(&self, context: &TaskContext, config: &TaskConfig) -> Result<TaskOutput, String> {
        let command = config.command.as_ref()
            .ok_or_else(|| "No command specified".to_string())?;
        
        println!("    [CMD] Executing: {} {:?}", command, config.args);
        
        // Build command with arguments that may reference process variables
        let mut cmd = Command::new(command);
        for arg in &config.args {
            // Simple variable substitution: ${varname}
            let processed_arg = if arg.starts_with("${") && arg.ends_with("}") {
                let var_name = &arg[2..arg.len()-1];
                context.get(var_name)
                    .map(|v| v.clone())
                    .unwrap_or_else(|| arg.clone())
            } else {
                arg.clone()
            };
            cmd.arg(processed_arg);
        }
        
        let output = cmd.output()
            .map_err(|e| format!("Failed to execute command: {}", e))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        
        println!("    [CMD] Exit code: {}", output.status.code().unwrap_or(-1));
        if !stdout.is_empty() {
            println!("    [CMD] Output: {}", stdout.trim());
        }
        
        let success = output.status.success();
        let error_msg = if !success { Some(stderr.clone()) } else { None };
        
        let mut result_vars = HashMap::new();
        result_vars.insert("stdout".to_string(), stdout.clone());
        result_vars.insert("stderr".to_string(), stderr);
        result_vars.insert("exit_code".to_string(), output.status.code().unwrap_or(-1).to_string());
        
        // Parse key=value pairs from stdout (one per line)
        // Format: KEY=VALUE or VAR:KEY=VALUE (prefixed)
        // Lines without '=' or not starting with VAR: are treated as logs and ignored for variable extraction
        for line in stdout.lines() {
            let line = line.trim();
            
            // Check if this is a prefixed variable line
            let var_line = if line.starts_with("VAR:") {
                &line[4..] // Strip VAR: prefix
            } else if line.contains('=') && !line.contains(' ') {
                // Simple heuristic: if it has = and no spaces before =, treat as variable
                // This allows: key=value but ignores: "This is log output with = sign"
                let eq_pos = line.find('=').unwrap();
                if line[..eq_pos].contains(' ') {
                    continue; // Skip if there's a space before =, likely a log line
                }
                line
            } else {
                continue; // Not a variable line
            };
            
            if let Some(eq_pos) = var_line.find('=') {
                let key = var_line[..eq_pos].trim().to_string();
                let value = var_line[eq_pos + 1..].trim().to_string();
                if !key.is_empty() {
                    println!("    [VAR-PARSED] {} = {}", key, value);
                    result_vars.insert(key, value);
                }
            }
        }
        
        Ok(TaskOutput {
            variables: result_vars,
            success,
            message: error_msg,
        })
    }
}

// CommandTaskHandler unit tests were moved to integration tests under `tests/`.
