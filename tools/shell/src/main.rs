use serde::{Deserialize, Serialize};
use std::io::{self, Read};

#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;

#[derive(Deserialize, Debug)]
struct ToolInput {
    operation: String,
    args: Vec<String>,
    #[allow(dead_code)]
    context: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ToolOutput {
    result: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
}

// Güvenli komutlar listesi - sadece bunlar çalıştırılabilir
const ALLOWED_COMMANDS: &[&str] = &[
    "ls", "pwd", "echo", "cat", "head", "tail", "wc", "grep", "find", "whoami", "date", "uname"
];

#[cfg(not(target_arch = "wasm32"))]
fn execute_shell_command(cmd: &str, args: &[String]) -> anyhow::Result<String> {
    // Native implementation - gerçek shell komutları
    
    // Güvenlik kontrolü
    if !ALLOWED_COMMANDS.contains(&cmd) {
        return Err(anyhow::anyhow!("Command '{}' is not allowed for security reasons", cmd));
    }
    
    let output = Command::new(cmd)
        .args(args)
        .output()?;
    
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Command failed: {}", stderr))
    }
}

#[cfg(target_arch = "wasm32")]
fn execute_shell_command_wasm(cmd: &str, args: &[String]) -> anyhow::Result<String> {
    // WASM implementation - simulated shell commands
    
    if !ALLOWED_COMMANDS.contains(&cmd) {
        return Err(anyhow::anyhow!("Command '{}' is not allowed", cmd));
    }
    
    match cmd {
        "ls" => {
            let default_path = ".".to_string();
            let path = args.get(0).unwrap_or(&default_path);
            Ok(format!("file1.txt\nfile2.txt\ndirectory/\n(simulated ls for {})", path))
        },
        "pwd" => Ok("/home/user/workspace".to_string()),
        "echo" => Ok(args.join(" ")),
        "whoami" => Ok("wasmuser".to_string()),
        "date" => Ok("Wed Jun 27 07:35:00 UTC 2024 (simulated)".to_string()),
        "uname" => {
            let default_flag = "-s".to_string();
            let flag = args.get(0).unwrap_or(&default_flag);
            match flag.as_str() {
                "-a" => Ok("WASM wasm-runtime 1.0.0 wasm32 wasm32-wasip1".to_string()),
                "-s" => Ok("WASM".to_string()),
                _ => Ok("WASM".to_string()),
            }
        },
        "cat" => {
            let default_file = "file.txt".to_string();
            let filename = args.get(0).unwrap_or(&default_file);
            Ok(format!("This is simulated content of {}", filename))
        },
        "wc" => {
            let default_file = "file.txt".to_string();
            let filename = args.get(0).unwrap_or(&default_file);
            Ok(format!("      10      42     256 {}", filename))
        },
        _ => Ok(format!("Simulated output for: {} {}", cmd, args.join(" "))),
    }
}

fn parse_command(operation: &str) -> (String, Vec<String>) {
    let parts: Vec<&str> = operation.split_whitespace().collect();
    if parts.is_empty() {
        return ("echo".to_string(), vec!["Invalid command".to_string()]);
    }
    
    let cmd = parts[0].to_string();
    let args = parts[1..].iter().map(|s| s.to_string()).collect();
    (cmd, args)
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    
    let tool_input: ToolInput = serde_json::from_str(&input)?;
    
    let (cmd, mut args) = parse_command(&tool_input.operation);
    
    // tool_input.args'ı da ekle
    args.extend(tool_input.args);
    
    let result = match execute_shell_command(&cmd, &args) {
        Ok(output) => ToolOutput {
            result: output.trim().to_string(),
            status: "success".to_string(),
            error: None,
            metadata: Some(serde_json::json!({
                "command": cmd,
                "args": args,
                "tool": "shell",
                "runtime": "native"
            })),
        },
        Err(e) => ToolOutput {
            result: "".to_string(),
            status: "error".to_string(),
            error: Some(e.to_string()),
            metadata: Some(serde_json::json!({
                "command": cmd,
                "args": args,
                "tool": "shell",
                "runtime": "native"
            })),
        },
    };
    
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() -> anyhow::Result<()> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    
    let tool_input: ToolInput = serde_json::from_str(&input)?;
    
    let (cmd, mut args) = parse_command(&tool_input.operation);
    
    // tool_input.args'ı da ekle
    args.extend(tool_input.args);
    
    let result = match execute_shell_command_wasm(&cmd, &args) {
        Ok(output) => ToolOutput {
            result: output,
            status: "success".to_string(),
            error: None,
            metadata: Some(serde_json::json!({
                "command": cmd,
                "args": args,
                "tool": "shell",
                "runtime": "wasm",
                "simulated": true
            })),
        },
        Err(e) => ToolOutput {
            result: "".to_string(),
            status: "error".to_string(),
            error: Some(e.to_string()),
            metadata: Some(serde_json::json!({
                "command": cmd,
                "args": args,
                "tool": "shell",
                "runtime": "wasm",
                "simulated": true
            })),
        },
    };
    
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
} 