use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,      // "user", "assistant", "system"
    pub content: String,   // Message content
    pub timestamp: u64,    // Unix timestamp
    pub metadata: HashMap<String, String>, // Extra context
}

impl Message {
    pub fn new(role: &str, content: &str) -> Self {
        Self {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

#[derive(Debug)]
pub struct AgentMemory {
    session_data: HashMap<String, String>,
    conversation_history: Vec<Message>,
    tool_results_cache: HashMap<String, String>, // tool_call_hash -> result
    max_history_size: usize,
}

impl AgentMemory {
    pub fn new() -> Self {
        Self {
            session_data: HashMap::new(),
            conversation_history: Vec::new(),
            tool_results_cache: HashMap::new(),
            max_history_size: 50, // Keep last 50 messages
        }
    }

    // Session data management
    pub fn store(&mut self, key: &str, value: &str) {
        self.session_data.insert(key.to_string(), value.to_string());
    }

    pub fn retrieve(&self, key: &str) -> Option<&str> {
        self.session_data.get(key).map(|s| s.as_str())
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.session_data.remove(key)
    }

    pub fn clear_session(&mut self) {
        self.session_data.clear();
    }

    // Conversation history management
    pub fn add_to_history(&mut self, message: Message) {
        self.conversation_history.push(message);
        
        // Trim history if it exceeds max size
        if self.conversation_history.len() > self.max_history_size {
            self.conversation_history.remove(0);
        }
    }

    pub fn get_history(&self) -> &Vec<Message> {
        &self.conversation_history
    }

    pub fn get_recent_history(&self, count: usize) -> &[Message] {
        let start_idx = if self.conversation_history.len() > count {
            self.conversation_history.len() - count
        } else {
            0
        };
        &self.conversation_history[start_idx..]
    }

    pub fn clear_history(&mut self) {
        self.conversation_history.clear();
    }

    // Tool results caching
    pub fn cache_tool_result(&mut self, tool_call_hash: &str, result: &str) {
        self.tool_results_cache.insert(tool_call_hash.to_string(), result.to_string());
    }

    pub fn get_cached_tool_result(&self, tool_call_hash: &str) -> Option<&str> {
        self.tool_results_cache.get(tool_call_hash).map(|s| s.as_str())
    }

    pub fn clear_tool_cache(&mut self) {
        self.tool_results_cache.clear();
    }

    // Context building for LLM
    pub fn build_context_prompt(&self, include_history_count: usize) -> String {
        let mut context = String::new();
        
        // Add session data as context
        if !self.session_data.is_empty() {
            context.push_str("Session Context:\n");
            for (key, value) in &self.session_data {
                context.push_str(&format!("- {}: {}\n", key, value));
            }
            context.push('\n');
        }

        // Add recent conversation history
        if !self.conversation_history.is_empty() && include_history_count > 0 {
            context.push_str("Recent Conversation:\n");
            let recent_messages = self.get_recent_history(include_history_count);
            for message in recent_messages {
                context.push_str(&format!("{}: {}\n", message.role, message.content));
            }
            context.push('\n');
        }

        context
    }

    // Memory statistics
    pub fn get_stats(&self) -> MemoryStats {
        MemoryStats {
            session_entries: self.session_data.len(),
            history_messages: self.conversation_history.len(),
            cached_tool_results: self.tool_results_cache.len(),
            memory_usage_estimate: self.estimate_memory_usage(),
        }
    }

    fn estimate_memory_usage(&self) -> usize {
        let session_size: usize = self.session_data.iter()
            .map(|(k, v)| k.len() + v.len())
            .sum();
        
        let history_size: usize = self.conversation_history.iter()
            .map(|msg| msg.content.len() + msg.role.len() + 
                msg.metadata.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>())
            .sum();
        
        let cache_size: usize = self.tool_results_cache.iter()
            .map(|(k, v)| k.len() + v.len())
            .sum();

        session_size + history_size + cache_size
    }

    // Persistence (basic implementation)
    pub fn export_to_json(&self) -> Result<String> {
        #[derive(Serialize)]
        struct MemoryExport {
            session_data: HashMap<String, String>,
            conversation_history: Vec<Message>,
            tool_results_cache: HashMap<String, String>,
        }

        let export = MemoryExport {
            session_data: self.session_data.clone(),
            conversation_history: self.conversation_history.clone(),
            tool_results_cache: self.tool_results_cache.clone(),
        };

        Ok(serde_json::to_string_pretty(&export)?)
    }

    pub fn import_from_json(&mut self, json_data: &str) -> Result<()> {
        #[derive(Deserialize)]
        struct MemoryImport {
            session_data: HashMap<String, String>,
            conversation_history: Vec<Message>,
            tool_results_cache: HashMap<String, String>,
        }

        let import: MemoryImport = serde_json::from_str(json_data)?;
        
        self.session_data = import.session_data;
        self.conversation_history = import.conversation_history;
        self.tool_results_cache = import.tool_results_cache;

        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct MemoryStats {
    pub session_entries: usize,
    pub history_messages: usize,
    pub cached_tool_results: usize,
    pub memory_usage_estimate: usize,
}

impl Default for AgentMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_basic_operations() {
        let mut memory = AgentMemory::new();
        
        // Test session storage
        memory.store("user_name", "Alice");
        assert_eq!(memory.retrieve("user_name"), Some("Alice"));
        
        // Test history
        let msg = Message::new("user", "Hello");
        memory.add_to_history(msg);
        assert_eq!(memory.get_history().len(), 1);
        
        // Test context building
        let context = memory.build_context_prompt(10);
        assert!(context.contains("user_name: Alice"));
        assert!(context.contains("user: Hello"));
    }

    #[test]
    fn test_memory_size_limits() {
        let mut memory = AgentMemory::new();
        memory.max_history_size = 3;
        
        // Add more messages than limit
        for i in 0..5 {
            memory.add_to_history(Message::new("user", &format!("Message {}", i)));
        }
        
        // Should only keep last 3
        assert_eq!(memory.get_history().len(), 3);
        assert_eq!(memory.get_history()[0].content, "Message 2");
    }

    #[test]
    fn test_tool_caching() {
        let mut memory = AgentMemory::new();
        
        memory.cache_tool_result("math_2+2", "4");
        assert_eq!(memory.get_cached_tool_result("math_2+2"), Some("4"));
        assert_eq!(memory.get_cached_tool_result("nonexistent"), None);
    }
} 