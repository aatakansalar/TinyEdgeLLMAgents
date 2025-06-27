use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    pub tool: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: u8, // 1-10, higher = more urgent
}

fn default_priority() -> u8 {
    5
}

impl ActionPlan {
    pub fn new(tool: &str, args: Vec<String>) -> Self {
        Self {
            tool: tool.to_string(),
            args,
            context: None,
            reasoning: None,
            priority: 5, // Default priority
        }
    }

    pub fn with_context(mut self, context: &str) -> Self {
        self.context = Some(context.to_string());
        self
    }

    pub fn with_reasoning(mut self, reasoning: &str) -> Self {
        self.reasoning = Some(reasoning.to_string());
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.clamp(1, 10);
        self
    }

    // Create hash for caching tool results
    pub fn cache_key(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.tool.hash(&mut hasher);
        for arg in &self.args {
            arg.hash(&mut hasher);
        }
        format!("{}_{:x}", self.tool, hasher.finish())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub actions: Vec<ActionPlan>,
    pub execution_strategy: ExecutionStrategy,
    pub timeout_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ExecutionStrategy {
    Sequential,   // Execute one by one
    Parallel,     // Execute all at once  
    Priority,     // Execute by priority order
}

pub struct Planner {
    available_tools: HashMap<String, ToolDefinition>,
    default_timeout: u64,
}

#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<String>,
    pub examples: Vec<String>,
}

impl Planner {
    pub fn new() -> Self {
        Self {
            available_tools: HashMap::new(),
            default_timeout: 30, // 30 seconds default
        }
    }

    pub fn register_tool(&mut self, tool: ToolDefinition) {
        self.available_tools.insert(tool.name.clone(), tool);
    }

    pub fn get_available_tools(&self) -> &HashMap<String, ToolDefinition> {
        &self.available_tools
    }

    // Main function: Parse LLM response into execution plan
    pub fn parse_llm_response(&self, response: &str) -> Result<ExecutionPlan> {
        // Try different parsing strategies
        if let Ok(plan) = self.parse_json_response(response) {
            return Ok(plan);
        }

        if let Ok(plan) = self.parse_structured_text(response) {
            return Ok(plan);
        }

        if let Ok(plan) = self.parse_natural_language(response) {
            return Ok(plan);
        }

        Err(anyhow!("Could not parse LLM response into action plan: {}", response))
    }

    // Parse direct JSON format like {"tool": "math", "args": ["2+2"]}
    fn parse_json_response(&self, response: &str) -> Result<ExecutionPlan> {
        let response = response.trim();
        
        // Handle single action
        if let Ok(single_action) = serde_json::from_str::<ActionPlan>(response) {
            if self.validate_action(&single_action)? {
                return Ok(ExecutionPlan {
                    actions: vec![single_action],
                    execution_strategy: ExecutionStrategy::Sequential,
                    timeout_seconds: self.default_timeout,
                });
            }
        }

        // Handle multiple actions array
        if let Ok(actions) = serde_json::from_str::<Vec<ActionPlan>>(response) {
            let validated_actions = self.validate_actions(actions)?;
            return Ok(ExecutionPlan {
                actions: validated_actions,
                execution_strategy: ExecutionStrategy::Sequential,
                timeout_seconds: self.default_timeout,
            });
        }

        // Handle full execution plan
        if let Ok(plan) = serde_json::from_str::<ExecutionPlan>(response) {
            let validated_actions = self.validate_actions(plan.actions)?;
            return Ok(ExecutionPlan {
                actions: validated_actions,
                execution_strategy: plan.execution_strategy,
                timeout_seconds: plan.timeout_seconds,
            });
        }

        Err(anyhow!("Invalid JSON format"))
    }

    // Parse structured text format like "Use tool: math with args: 2+2"
    fn parse_structured_text(&self, response: &str) -> Result<ExecutionPlan> {
        let mut actions = Vec::new();
        
        for line in response.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(action) = self.parse_tool_line(line)? {
                actions.push(action);
            }
        }

        if actions.is_empty() {
            return Err(anyhow!("No valid tool commands found"));
        }

        Ok(ExecutionPlan {
            actions,
            execution_strategy: ExecutionStrategy::Sequential,
            timeout_seconds: self.default_timeout,
        })
    }

    fn parse_tool_line(&self, line: &str) -> Result<Option<ActionPlan>> {
        // Pattern: "Use tool: math with args: 2+2"
        if let Some(caps) = regex::Regex::new(r"(?i)use\s+tool:\s*(\w+)\s+with\s+args?:\s*(.+)")
            .unwrap()
            .captures(line) {
            let tool = caps.get(1).unwrap().as_str();
            let args_str = caps.get(2).unwrap().as_str();
            let args = self.parse_args(args_str);
            
            let action = ActionPlan::new(tool, args);
            if self.validate_action(&action)? {
                return Ok(Some(action));
            }
        }

        // Pattern: "math(2+2)"
        if let Some(caps) = regex::Regex::new(r"(\w+)\(([^)]+)\)")
            .unwrap()
            .captures(line) {
            let tool = caps.get(1).unwrap().as_str();
            let args_str = caps.get(2).unwrap().as_str();
            let args = self.parse_args(args_str);
            
            let action = ActionPlan::new(tool, args);
            if self.validate_action(&action)? {
                return Ok(Some(action));
            }
        }

        Ok(None)
    }

    // Parse natural language and try to extract tool usage
    fn parse_natural_language(&self, response: &str) -> Result<ExecutionPlan> {
        let mut actions = Vec::new();
        
        // Look for math expressions
        if let Some(action) = self.extract_math_action(response)? {
            actions.push(action);
        }

        // Look for web requests
        if let Some(action) = self.extract_fetch_action(response)? {
            actions.push(action);
        }

        // Look for shell commands
        if let Some(action) = self.extract_shell_action(response)? {
            actions.push(action);
        }

        if actions.is_empty() {
            return Err(anyhow!("Could not extract actions from natural language"));
        }

        Ok(ExecutionPlan {
            actions,
            execution_strategy: ExecutionStrategy::Sequential,
            timeout_seconds: self.default_timeout,
        })
    }

    fn extract_math_action(&self, text: &str) -> Result<Option<ActionPlan>> {
        // Simple math expression detection
        let math_patterns = [
            r"\b(\d+\s*[+\-*/]\s*\d+(?:\s*[+\-*/]\s*\d+)*)\b",
            r"\bsqrt\(\d+\)",
            r"\b\d+\s*\^\s*\d+\b",
        ];

        for pattern in &math_patterns {
            if let Some(caps) = regex::Regex::new(pattern).unwrap().captures(text) {
                let expression = caps.get(1).unwrap_or(caps.get(0).unwrap()).as_str();
                let action = ActionPlan::new("math", vec![expression.to_string()])
                    .with_reasoning("Detected math expression in natural language");
                
                if self.validate_action(&action)? {
                    return Ok(Some(action));
                }
            }
        }

        Ok(None)
    }

    fn extract_fetch_action(&self, text: &str) -> Result<Option<ActionPlan>> {
        // URL detection
        let url_pattern = r"https?://[^\s]+";
        if let Some(caps) = regex::Regex::new(url_pattern).unwrap().captures(text) {
            let url = caps.get(0).unwrap().as_str();
            let action = ActionPlan::new("fetch", vec!["get".to_string(), url.to_string()])
                .with_reasoning("Detected URL in text");
            
            if self.validate_action(&action)? {
                return Ok(Some(action));
            }
        }

        Ok(None)
    }

    fn extract_shell_action(&self, text: &str) -> Result<Option<ActionPlan>> {
        // Look for shell command indicators
        let shell_indicators = ["run command", "execute", "shell", "command"];
        
        for indicator in &shell_indicators {
            if text.to_lowercase().contains(indicator) {
                // This is a very basic implementation
                // In practice, you'd want more sophisticated NLP
                let action = ActionPlan::new("shell", vec!["echo".to_string(), "Hello from shell".to_string()])
                    .with_reasoning("Detected shell command request");
                
                if self.validate_action(&action)? {
                    return Ok(Some(action));
                }
            }
        }

        Ok(None)
    }

    fn parse_args(&self, args_str: &str) -> Vec<String> {
        // Simple comma-separated parsing
        args_str.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    fn validate_action(&self, action: &ActionPlan) -> Result<bool> {
        // Check direct tool name first
        if self.available_tools.contains_key(&action.tool) {
            if action.args.is_empty() {
                return Err(anyhow!("Tool {} requires arguments", action.tool));
            }
            return Ok(true);
        }

        // Check for tool aliases/mappings
        let mapped_tool = match action.tool.as_str() {
            "math" => self.available_tools.keys().find(|k| k.contains("math")),
            "fetch" => self.available_tools.keys().find(|k| k.contains("fetch")),
            "shell" => self.available_tools.keys().find(|k| k.contains("shell")),
            _ => None,
        };

        if mapped_tool.is_some() {
            if action.args.is_empty() {
                return Err(anyhow!("Tool {} requires arguments", action.tool));
            }
            return Ok(true);
        }

        Err(anyhow!("Unknown tool: {}", action.tool))
    }

    fn validate_actions(&self, actions: Vec<ActionPlan>) -> Result<Vec<ActionPlan>> {
        let mut validated = Vec::new();
        
        for action in actions {
            if self.validate_action(&action)? {
                // Don't modify tool names - LLM provides exact names
                validated.push(action);
            }
        }

        if validated.is_empty() {
            return Err(anyhow!("No valid actions found"));
        }

        Ok(validated)
    }

    fn map_tool_alias(&self, tool_name: &str) -> String {
        // If tool exists directly, return it
        if self.available_tools.contains_key(tool_name) {
            return tool_name.to_string();
        }

        // Map aliases to actual tool names
        match tool_name {
            "math" => self.available_tools.keys()
                .find(|k| k.contains("math"))
                .cloned()
                .unwrap_or_else(|| tool_name.to_string()),
            "fetch" => self.available_tools.keys()
                .find(|k| k.contains("fetch"))
                .cloned()
                .unwrap_or_else(|| tool_name.to_string()),
            "shell" => self.available_tools.keys()
                .find(|k| k.contains("shell"))
                .cloned()
                .unwrap_or_else(|| tool_name.to_string()),
            _ => tool_name.to_string(),
        }
    }

    // Generate system prompt for LLM with available tools
    pub fn generate_system_prompt(&self) -> String {
        let mut prompt = String::from(
            "You are an autonomous agent. Parse user requests and output JSON action plans.\n\n"
        );

        prompt.push_str("Available tools:\n");
        for (name, tool) in &self.available_tools {
            prompt.push_str(&format!(
                "- {}: {} (parameters: {})\n",
                name,
                tool.description,
                tool.parameters.join(", ")
            ));
        }

        prompt.push_str("\nOutput format: {\"tool\": \"tool_name\", \"args\": [\"arg1\", \"arg2\"], \"reasoning\": \"explanation\"}\n");
        prompt.push_str("For multiple actions: [{\"tool\": \"tool1\", \"args\": [...]}, {\"tool\": \"tool2\", \"args\": [...]}]\n\n");

        prompt.push_str("Examples:\n");
        for tool in self.available_tools.values() {
            for example in &tool.examples {
                prompt.push_str(&format!("- {}\n", example));
            }
        }

        prompt
    }
}

impl Default for Planner {
    fn default() -> Self {
        let mut planner = Self::new();
        
        // Register default tools (prioritize native versions)
        planner.register_tool(ToolDefinition {
            name: "math-native".to_string(),
            description: "Perform mathematical calculations".to_string(),
            parameters: vec!["expression".to_string()],
            examples: vec![
                "User: What is 5*7? → {\"tool\": \"math-native\", \"args\": [\"5*7\"]}".to_string(),
                "User: Calculate sqrt(16) → {\"tool\": \"math-native\", \"args\": [\"sqrt(16)\"]}".to_string(),
            ],
        });
        
        planner.register_tool(ToolDefinition {
            name: "math".to_string(),
            description: "Perform mathematical calculations".to_string(),
            parameters: vec!["expression".to_string()],
            examples: vec![
                "User: What is 5*7? → {\"tool\": \"math\", \"args\": [\"5*7\"]}".to_string(),
                "User: Calculate sqrt(16) → {\"tool\": \"math\", \"args\": [\"sqrt(16)\"]}".to_string(),
            ],
        });

        planner.register_tool(ToolDefinition {
            name: "fetch".to_string(),
            description: "Make HTTP requests to fetch data".to_string(),
            parameters: vec!["method".to_string(), "url".to_string()],
            examples: vec![
                "User: Get data from example.com → {\"tool\": \"fetch\", \"args\": [\"get\", \"http://example.com\"]}".to_string(),
            ],
        });

        planner.register_tool(ToolDefinition {
            name: "shell".to_string(),
            description: "Execute shell commands safely".to_string(),
            parameters: vec!["command".to_string(), "args...".to_string()],
            examples: vec![
                "User: List files → {\"tool\": \"shell\", \"args\": [\"ls\", \"-la\"]}".to_string(),
            ],
        });

        planner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_parsing() {
        let planner = Planner::default();
        
        let json_input = r#"{"tool": "math", "args": ["2+2"], "reasoning": "Simple addition"}"#;
        let plan = planner.parse_llm_response(json_input).unwrap();
        
        assert_eq!(plan.actions.len(), 1);
        assert_eq!(plan.actions[0].tool, "math");
        assert_eq!(plan.actions[0].args[0], "2+2");
    }

    #[test]
    fn test_natural_language_math() {
        let planner = Planner::default();
        
        let text = "I need to calculate 5 * 7 for my homework";
        let plan = planner.parse_llm_response(text).unwrap();
        
        assert_eq!(plan.actions.len(), 1);
        assert_eq!(plan.actions[0].tool, "math");
    }

    #[test]
    fn test_system_prompt_generation() {
        let planner = Planner::default();
        let prompt = planner.generate_system_prompt();
        
        assert!(prompt.contains("math"));
        assert!(prompt.contains("fetch"));
        assert!(prompt.contains("shell"));
        assert!(prompt.contains("JSON"));
    }
} 