pub mod memory;
pub mod planner;
pub mod dispatcher;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tinyedgellmagents_core::{SuperTinyWasmLLM, InferenceRequest, InferenceResponse};

pub use memory::{AgentMemory, Message, MemoryStats};
pub use planner::{ActionPlan, ExecutionPlan, ExecutionStrategy, Planner, ToolDefinition};
pub use dispatcher::{ToolDispatcher, ToolResult, DispatcherStats};

#[derive(Debug, Deserialize)]
pub struct TaskRequest {
    pub task: String,
    pub context: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub success: bool,
    pub result: String,
    pub reasoning: Option<String>,
    pub tools_used: Vec<String>,
    pub execution_time_ms: u64,
    pub memory_stats: MemoryStats,
}

pub struct TinyEdgeAgent {
    llm: SuperTinyWasmLLM,
    memory: AgentMemory,
    planner: Planner,
    dispatcher: ToolDispatcher,
    model_loaded: bool,
}

impl TinyEdgeAgent {
    pub fn new(model_path: &str) -> Self {
        Self {
            llm: SuperTinyWasmLLM::new(model_path.to_string()),
            memory: AgentMemory::new(),
            planner: Planner::default(), // Includes default tools
            dispatcher: ToolDispatcher::new(),
            model_loaded: false,
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        // Load the LLM model
        self.llm.load_model()
            .map_err(|e| anyhow!("Failed to load LLM model: {}", e))?;
        self.model_loaded = true;

        println!("TinyEdgeAgent initialized successfully");
        Ok(())
    }

    pub async fn load_tools(&mut self, tools_dir: &str) -> Result<usize> {
        if !Path::new(tools_dir).exists() {
            println!("Tools directory '{}' does not exist, skipping tool loading", tools_dir);
            return Ok(0);
        }

        let discovered = self.dispatcher.discover_tools(tools_dir)?;
        println!("Loaded {} tools from {}", discovered, tools_dir);
        
        // Health check tools before registering with planner
        let tool_health = self.dispatcher.health_check().await.unwrap_or_default();
        
        // Clear planner and register only healthy discovered tools
        self.planner = Planner::new(); // Reset planner to remove default tools
        
        // Update planner with only healthy tools
        for tool_name in self.dispatcher.get_available_tools() {
            let is_healthy = tool_health.get(&tool_name).copied().unwrap_or(false);
            
            if is_healthy {
                if let Some(tool_info) = self.dispatcher.get_tool_info(&tool_name) {
                    let tool_def = ToolDefinition {
                        name: tool_name.clone(),
                        description: tool_info.description.clone(),
                        parameters: vec!["operation".to_string(), "args...".to_string()],
                        examples: vec![format!("{{\"tool\": \"{}\", \"args\": [\"operation\", \"arg1\"]}}", tool_name)],
                    };
                    self.planner.register_tool(tool_def);
                    println!("Registered healthy tool: {}", tool_name);
                } else {
                    println!("Skipped unhealthy tool: {}", tool_name);
                }
            }
        }

        Ok(discovered)
    }

    pub async fn execute_task(&mut self, request: &TaskRequest) -> Result<TaskResponse> {
        let start_time = std::time::Instant::now();

        if !self.model_loaded {
            return Err(anyhow!("Agent not initialized. Call initialize() first."));
        }

        // Store task in memory
        self.memory.store("current_task", &request.task);
        self.memory.add_to_history(Message::new("user", &request.task));

        // Build context for LLM
        let context = self.memory.build_context_prompt(3); // Include last 3 messages
        let system_prompt = self.planner.generate_system_prompt();
        
        let enhanced_prompt = format!(
            "{}\n\n{}\n\nUser task: {}",
            system_prompt,
            context,
            request.task
        );

        // Generate plan via LLM
        let llm_request = InferenceRequest {
            prompt: enhanced_prompt,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };

        let llm_response = self.llm.generate_response(&llm_request)
            .map_err(|e| anyhow!("LLM inference failed: {}", e))?;

        // Store LLM response in memory
        self.memory.add_to_history(Message::new("assistant", &llm_response.response));

        // Parse LLM response into execution plan
        let execution_plan = match self.planner.parse_llm_response(&llm_response.response) {
            Ok(plan) => plan,
            Err(e) => {
                // Fallback: try to extract simple text response
                println!("Warning: Failed to parse LLM response as action plan: {}", e);
                return Ok(TaskResponse {
                    success: true,
                    result: llm_response.response,
                    reasoning: Some("Direct LLM response (no tools executed)".to_string()),
                    tools_used: vec![],
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    memory_stats: self.memory.get_stats(),
                });
            }
        };

        // Execute the plan
        let tool_results = self.dispatcher.execute_plan(&execution_plan).await
            .map_err(|e| anyhow!("Tool execution failed: {}", e))?;

        // Process results
        let mut final_result = String::new();
        let mut tools_used = Vec::new();
        let mut all_successful = true;

        for result in &tool_results {
            tools_used.push(result.tool_name.clone());
            
            if result.success {
                if !final_result.is_empty() {
                    final_result.push_str("; ");
                }
                final_result.push_str(&result.result);
                
                // Cache successful results
                if let Some(action) = execution_plan.actions.iter().find(|a| a.tool == result.tool_name) {
                    self.memory.cache_tool_result(&action.cache_key(), &result.result);
                }
            } else {
                all_successful = false;
                if let Some(error) = &result.error {
                    final_result.push_str(&format!("Error in {}: {}", result.tool_name, error));
                }
            }
        }

        // Store results in memory
        self.memory.store("last_result", &final_result);
        self.memory.add_to_history(Message::new("system", &format!("Task completed. Result: {}", final_result)));

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(TaskResponse {
            success: all_successful,
            result: if final_result.is_empty() { "No results generated".to_string() } else { final_result },
            reasoning: execution_plan.actions.first().and_then(|a| a.reasoning.clone()),
            tools_used,
            execution_time_ms: execution_time,
            memory_stats: self.memory.get_stats(),
        })
    }

    // Agent introspection
    pub fn get_available_tools(&self) -> Vec<String> {
        self.dispatcher.get_available_tools()
    }

    pub fn get_memory_stats(&self) -> MemoryStats {
        self.memory.get_stats()
    }

    pub fn get_dispatcher_stats(&self) -> DispatcherStats {
        self.dispatcher.get_stats()
    }

    pub async fn health_check(&self) -> Result<AgentHealthStatus> {
        let tool_health = self.dispatcher.health_check().await?;
        
        Ok(AgentHealthStatus {
            llm_loaded: self.model_loaded,
            tools_healthy: tool_health,
            memory_usage: self.memory.get_stats().memory_usage_estimate,
            total_tools: self.dispatcher.get_stats().total_tools,
        })
    }

    // Memory management
    pub fn clear_memory(&mut self) {
        self.memory.clear_session();
        self.memory.clear_history();
        self.memory.clear_tool_cache();
    }

    pub fn export_memory(&self) -> Result<String> {
        self.memory.export_to_json()
    }

    pub fn import_memory(&mut self, json_data: &str) -> Result<()> {
        self.memory.import_from_json(json_data)
    }
}

#[derive(Debug, Serialize)]
pub struct AgentHealthStatus {
    pub llm_loaded: bool,
    pub tools_healthy: std::collections::HashMap<String, bool>,
    pub memory_usage: usize,
    pub total_tools: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_creation() {
        let agent = TinyEdgeAgent::new("test_model.gguf");
        assert!(!agent.model_loaded);
        assert_eq!(agent.get_available_tools().len(), 0);
    }

    #[tokio::test]
    async fn test_memory_operations() {
        let mut agent = TinyEdgeAgent::new("test_model.gguf");
        
        agent.memory.store("test_key", "test_value");
        assert_eq!(agent.memory.retrieve("test_key"), Some("test_value"));
        
        let stats = agent.get_memory_stats();
        assert_eq!(stats.session_entries, 1);
    }

    #[test]
    fn test_task_request_parsing() {
        let json = r#"{"task": "What is 2+2?", "max_tokens": 50}"#;
        let request: TaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.task, "What is 2+2?");
        assert_eq!(request.max_tokens, Some(50));
    }
} 