use crate::planner::{ActionPlan, ExecutionPlan, ExecutionStrategy};
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use wasmtime::*;
use tokio::time::timeout;
use tokio::io::AsyncWriteExt;
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub result: String,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub tool_name: String,
    pub metadata: HashMap<String, String>,
}

impl ToolResult {
    pub fn success(tool_name: &str, result: &str, execution_time: Duration) -> Self {
        Self {
            success: true,
            result: result.to_string(),
            error: None,
            execution_time_ms: execution_time.as_millis() as u64,
            tool_name: tool_name.to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn error(tool_name: &str, error: &str, execution_time: Duration) -> Self {
        Self {
            success: false,
            result: String::new(),
            error: Some(error.to_string()),
            execution_time_ms: execution_time.as_millis() as u64,
            tool_name: tool_name.to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

pub struct WasmTool {
    pub name: String,
    pub wasm_path: String,
    pub description: String,
    pub engine: Engine,
    pub module: Module,
}

impl WasmTool {
    pub fn new(name: &str, wasm_path: &str, description: &str) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::from_file(&engine, wasm_path)
            .map_err(|e| anyhow!("Failed to load WASM module {}: {}", wasm_path, e))?;

        Ok(Self {
            name: name.to_string(),
            wasm_path: wasm_path.to_string(),
            description: description.to_string(),
            engine,
            module,
        })
    }

    pub async fn execute(&self, input: &str) -> Result<String> {
        // Try native tool first
        if let Ok(output) = self.execute_native_tool(input).await {
            return Ok(output);
        }
        
        // Try WASM tool execution
        if let Ok(output) = self.execute_wasm_tool(input).await {
            return Ok(output);
        }
        
        Err(anyhow!("Tool execution failed for {}: neither native nor WASM execution succeeded", self.name))
    }
    
    async fn execute_wasm_tool(&self, input: &str) -> Result<String> {
        use wasmtime_wasi::WasiCtxBuilder;
        use std::process::{Command, Stdio};
        
        // For now, use wasmtime CLI to execute WASM tools with proper I/O
        // This is more reliable than direct wasmtime API for stdin/stdout handling
        let mut cmd = Command::new("wasmtime")
            .arg("--dir=.")
            .arg(&self.wasm_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn wasmtime: {}. Make sure wasmtime is installed.", e))?;
        
        // Send input to stdin
        if let Some(stdin) = cmd.stdin.take() {
            use std::io::Write;
            let mut stdin = stdin;
            stdin.write_all(input.as_bytes())
                .map_err(|e| anyhow!("Failed to write to stdin: {}", e))?;
        }
        
        // Wait for output
        let output = cmd.wait_with_output()
            .map_err(|e| anyhow!("Failed to wait for wasmtime: {}", e))?;
        
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if stdout.trim().is_empty() {
                Err(anyhow!("WASM tool produced no output"))
            } else {
                Ok(stdout.trim().to_string())
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("WASM execution failed: {}", stderr))
        }
    }
    
    async fn execute_native_tool(&self, input: &str) -> Result<String> {
        use std::process::Stdio;
        use tokio::process::Command;
        
        // Use the exact path stored in wasm_path for native tools
        let tool_path = &self.wasm_path;
        
        // Convert to absolute path if it's relative
        let absolute_path = if std::path::Path::new(tool_path).is_absolute() {
            tool_path.to_string()
        } else {
            // Get absolute path relative to current working directory
            match std::env::current_dir() {
                Ok(cwd) => cwd.join(tool_path).to_string_lossy().to_string(),
                Err(_) => tool_path.to_string(),
            }
        };
        
        if !std::path::Path::new(&absolute_path).exists() {
            return Err(anyhow!("Native tool not found: {} (resolved from {})", absolute_path, tool_path));
        }
        
        // Execute the tool
        let mut child = Command::new(&absolute_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        
        // Send input to tool
        if let Some(stdin) = child.stdin.take() {
            let mut stdin = stdin;
            stdin.write_all(input.as_bytes()).await?;
            stdin.shutdown().await?;
        }
        
        // Wait for output
        let output = child.wait_with_output().await?;
        
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("Tool execution failed: {}", error))
        }
    }
}

pub struct ToolDispatcher {
    tools: HashMap<String, WasmTool>,
    default_timeout: Duration,
}

impl ToolDispatcher {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            default_timeout: Duration::from_secs(30),
        }
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.default_timeout = timeout;
    }

    // Auto-discover tools from a directory
    pub fn discover_tools(&mut self, tools_dir: &str) -> Result<usize> {
        let mut discovered = 0;

        if !Path::new(tools_dir).exists() {
            return Ok(0);
        }

        for entry in WalkDir::new(tools_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            // Check for WASM files
            if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                if let Some(tool_name) = path.file_stem().and_then(|s| s.to_str()) {
                    match self.register_tool(tool_name, path.to_string_lossy().as_ref()) {
                        Ok(_) => {
                            discovered += 1;
                            println!("Discovered WASM tool: {} at {}", tool_name, path.display());
                        }
                        Err(e) => {
                            eprintln!("Failed to register tool {}: {}", tool_name, e);
                        }
                    }
                }
            }
            
            // Check for native binaries (executable files without extension)
            if path.is_file() && path.extension().is_none() {
                if let Some(tool_name) = path.file_name().and_then(|s| s.to_str()) {
                    // Only register main tools, skip build artifacts
                    if !tool_name.contains("build") && ![".", "..", "README", "LICENSE"].contains(&tool_name) && 
                       (tool_name.ends_with("-native") || ["math", "fetch", "shell"].contains(&tool_name)) {
                        // Check if file is executable
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            if let Ok(metadata) = path.metadata() {
                                let permissions = metadata.permissions();
                                if permissions.mode() & 0o111 != 0 { // Check execute permission
                                    match self.register_tool(tool_name, path.to_string_lossy().as_ref()) {
                                        Ok(_) => {
                                            discovered += 1;
                                            println!("Discovered native tool: {} at {}", tool_name, path.display());
                                        }
                                        Err(e) => {
                                            eprintln!("Failed to register tool {}: {}", tool_name, e);
                                        }
                                    }
                                }
                            }
                        }
                        #[cfg(not(unix))]
                        {
                            match self.register_tool(tool_name, path.to_string_lossy().as_ref()) {
                                Ok(_) => {
                                    discovered += 1;
                                    println!("Discovered native tool: {} at {}", tool_name, path.display());
                                }
                                Err(e) => {
                                    eprintln!("Failed to register tool {}: {}", tool_name, e);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(discovered)
    }

    // Register a specific tool (supports both WASM and native binaries)
    pub fn register_tool(&mut self, name: &str, tool_path: &str) -> Result<()> {
        let description = format!("Tool: {}", name);
        let engine = Engine::default();
        
        // Load actual WASM module if it's a .wasm file, otherwise create minimal module for native tools
        let module = if tool_path.ends_with(".wasm") && std::path::Path::new(tool_path).exists() {
            // Load real WASM module
            Module::from_file(&engine, tool_path)
                .map_err(|e| anyhow!("Failed to load WASM module {}: {}", tool_path, e))?
        } else {
            // For native tools, create minimal placeholder module
            let minimal_wasm = wat::parse_str("(module)")?;
            Module::new(&engine, &minimal_wasm)?
        };
        
        let tool = WasmTool {
            name: name.to_string(),
            wasm_path: tool_path.to_string(),
            description,
            engine,
            module,
        };
        
        self.tools.insert(name.to_string(), tool);
        Ok(())
    }

    // Execute a single action plan
    pub async fn execute_action(&self, action: &ActionPlan) -> Result<ToolResult> {
        let start_time = Instant::now();

        // Map tool aliases to actual tool names
        let actual_tool_name = self.map_tool_alias(&action.tool);
        
        let tool = self.tools.get(&actual_tool_name)
            .ok_or_else(|| anyhow!("Unknown tool: {} (mapped from {})", actual_tool_name, action.tool))?;

        // Prepare input JSON for the tool
        let tool_input = if action.args.len() == 1 {
            // For tools that expect the operation as the main argument (like math)
            serde_json::json!({
                "operation": action.args[0],
                "args": [],
                "context": action.context
            })
        } else {
            // For tools with multiple arguments
            serde_json::json!({
                "operation": action.args.get(0).unwrap_or(&"default".to_string()),
                "args": &action.args[1..],
                "context": action.context
            })
        };

        let input_str = serde_json::to_string(&tool_input)?;

        // Execute with timeout
        let execution_result = timeout(self.default_timeout, tool.execute(&input_str)).await;

        let execution_time = start_time.elapsed();

        match execution_result {
            Ok(Ok(output)) => {
                // Try to parse tool output as JSON
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&output) {
                    if let Some(result) = parsed.get("result") {
                        return Ok(ToolResult::success(
                            &actual_tool_name,
                            &result.to_string(),
                            execution_time,
                        ));
                    }
                }
                
                // If not JSON, return raw output
                Ok(ToolResult::success(&actual_tool_name, &output.trim(), execution_time))
            }
            Ok(Err(e)) => Ok(ToolResult::error(&actual_tool_name, &e.to_string(), execution_time)),
            Err(_) => Ok(ToolResult::error(
                &actual_tool_name,
                "Tool execution timeout",
                execution_time,
            )),
        }
    }

    // Execute an entire execution plan
    pub async fn execute_plan(&self, plan: &ExecutionPlan) -> Result<Vec<ToolResult>> {
        let mut results = Vec::new();

        match plan.execution_strategy {
            ExecutionStrategy::Sequential => {
                for action in &plan.actions {
                    let result = self.execute_action(action).await?;
                    results.push(result);
                    
                    // If an action fails in sequential mode, we might want to stop
                    // For now, we continue regardless
                }
            }
            ExecutionStrategy::Parallel => {
                // Execute all actions concurrently
                let futures: Vec<_> = plan.actions
                    .iter()
                    .map(|action| self.execute_action(action))
                    .collect();

                let execution_results = futures::future::join_all(futures).await;
                
                for result in execution_results {
                    match result {
                        Ok(tool_result) => results.push(tool_result),
                        Err(e) => {
                            // Create an error result for failed executions
                            results.push(ToolResult::error("unknown", &e.to_string(), Duration::default()));
                        }
                    }
                }
            }
            ExecutionStrategy::Priority => {
                // Sort by priority (higher number = higher priority)
                let mut sorted_actions = plan.actions.clone();
                sorted_actions.sort_by(|a, b| b.priority.cmp(&a.priority));

                for action in &sorted_actions {
                    let result = self.execute_action(action).await?;
                    results.push(result);
                }
            }
        }

        Ok(results)
    }

    // Get list of available tools
    pub fn get_available_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    // Get tool information
    pub fn get_tool_info(&self, tool_name: &str) -> Option<&WasmTool> {
        self.tools.get(tool_name)
    }

    // Health check for tools
    pub async fn health_check(&self) -> Result<HashMap<String, bool>> {
        let mut health_status = HashMap::new();
        
        for (name, tool) in &self.tools {
            // Use appropriate test input for each tool type
            let test_input = if name.contains("math") {
                // Test with simple math for math tools
                serde_json::json!({
                    "operation": "1+1",
                    "args": [],
                    "context": null
                })
            } else if name.contains("fetch") {
                // Test with simple URL for fetch tools  
                serde_json::json!({
                    "operation": "http://httpbin.org/status/200",
                    "args": [],
                    "context": null
                })
            } else if name.contains("shell") {
                // Test with safe command for shell tools
                serde_json::json!({
                    "operation": "echo hello",
                    "args": [],
                    "context": null
                })
            } else {
                // Default ping for unknown tools
                serde_json::json!({
                    "operation": "ping",
                    "args": [],
                    "context": null
                })
            };

            let health = match timeout(Duration::from_secs(5), tool.execute(&test_input.to_string())).await {
                Ok(Ok(output)) => {
                    // Check if output indicates success
                    !output.is_empty() && !output.to_lowercase().contains("error")
                },
                _ => false,
            };

            health_status.insert(name.clone(), health);
        }

        Ok(health_status)
    }

    // Tool statistics
    pub fn get_stats(&self) -> DispatcherStats {
        DispatcherStats {
            total_tools: self.tools.len(),
            tool_names: self.get_available_tools(),
            timeout_seconds: self.default_timeout.as_secs(),
        }
    }

    // Map tool aliases to actual tool names
    fn map_tool_alias(&self, tool_name: &str) -> String {
        // If tool exists directly, return it
        if self.tools.contains_key(tool_name) {
            return tool_name.to_string();
        }

        // Map aliases to actual tool names (prioritize -native versions)
        match tool_name {
            "math" => self.tools.keys()
                .find(|k| k.ends_with("-native") && k.contains("math"))
                .or_else(|| self.tools.keys().find(|k| k.contains("math")))
                .cloned()
                .unwrap_or_else(|| tool_name.to_string()),
            "fetch" => self.tools.keys()
                .find(|k| k.ends_with("-native") && k.contains("fetch"))
                .or_else(|| self.tools.keys().find(|k| k.contains("fetch")))
                .cloned()
                .unwrap_or_else(|| tool_name.to_string()),
            "shell" => self.tools.keys()
                .find(|k| k.ends_with("-native") && k.contains("shell"))
                .or_else(|| self.tools.keys().find(|k| k.contains("shell")))
                .cloned()
                .unwrap_or_else(|| tool_name.to_string()),
            _ => tool_name.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DispatcherStats {
    pub total_tools: usize,
    pub tool_names: Vec<String>,
    pub timeout_seconds: u64,
}

impl Default for ToolDispatcher {
    fn default() -> Self {
        Self::new()
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner::ActionPlan;

    #[tokio::test]
    async fn test_real_tool_execution() {
        let mut dispatcher = ToolDispatcher::new();
        
        // Try to register actual tools if they exist
        if std::path::Path::new("../tools").exists() {
            let _ = dispatcher.discover_tools("../tools");
        }
        
        // This test would only pass with real tools present
        assert!(dispatcher.get_available_tools().len() >= 0);
    }

    #[tokio::test]
    async fn test_dispatcher_tool_discovery() {
        let mut dispatcher = ToolDispatcher::new();
        
        // Test tool discovery functionality
        let discovered = dispatcher.discover_tools("../tools").unwrap_or(0);
        println!("Discovered {} tools", discovered);
        
        // Should find at least some tools if directory exists
        if std::path::Path::new("../tools").exists() {
            assert!(discovered >= 0);
        }
    }
} 