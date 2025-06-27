use tinyedgellmagents::{TinyEdgeAgent, TaskRequest};
use std::env;
use std::io::{self, Read, Write};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tinyedgellmagents")]
#[command(about = "TinyEdgeLLMAgents - Experimental Edge LLM Agent Runtime")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Model path (optional, defaults to core/model.gguf)
    #[arg(short, long)]
    model: Option<String>,
    
    /// Tools directory (optional, defaults to ../tools)
    #[arg(short, long)]
    tools: Option<String>,
    
    /// Enable interactive mode
    #[arg(short, long)]
    interactive: bool,
    
    /// Pretty print JSON output
    #[arg(short, long)]
    pretty: bool,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a single task via command line
    Task {
        /// The task to execute
        task: String,
        /// Maximum tokens for LLM response
        #[arg(long, default_value = "100")]
        max_tokens: u32,
        /// Temperature for LLM response
        #[arg(long, default_value = "0.7")]
        temperature: f32,
    },
    /// Show system status
    Status,
    /// List available tools
    Tools,
    /// Run health check
    Health,
    /// Enter interactive mode
    Interactive,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Initialize agent
    let model_path = cli.model.unwrap_or_else(|| 
        env::var("TINYEDGELLMAGENTS_MODEL").unwrap_or_else(|_| "core/model.gguf".to_string())
    );
    
    let tools_dir = cli.tools.unwrap_or_else(|| 
        env::var("TINYEDGELLMAGENTS_TOOLS_DIR").unwrap_or_else(|_| "../tools".to_string())
    );

    if !cli.verbose {
        // Suppress startup messages in non-verbose mode
        std::env::set_var("TINYEDGELLMAGENTS_QUIET", "1");
    }

    println!("TinyEdgeLLMAgents v0.1.0 - Experimental Edge LLM Agent Runtime");
    println!("============================================");
    println!("Model path: {}", model_path);
    println!("Tools directory: {}", tools_dir);
    
    if cli.verbose {
        println!("Initializing agent...");
    }
    
    let mut agent = TinyEdgeAgent::new(&model_path);
    
    if let Err(e) = agent.initialize().await {
        eprintln!("Failed to initialize agent: {}", e);
        std::process::exit(1);
    }
    
    if cli.verbose {
        println!("TinyEdgeAgent initialized successfully");
        println!("Loading tools...");
    }
    
    let tools_loaded = agent.load_tools(&tools_dir).await.unwrap_or(0);
    println!("Loaded {} tools", tools_loaded);
    
    // Handle commands
    match cli.command {
        Some(Commands::Task { task, max_tokens, temperature }) => {
            execute_single_task(&mut agent, &task, max_tokens, temperature, cli.pretty).await?;
        }
        Some(Commands::Status) => {
            show_status(&agent, cli.pretty).await?;
        }
        Some(Commands::Tools) => {
            show_tools(&agent, cli.pretty)?;
        }
        Some(Commands::Health) => {
            show_health(&agent, cli.pretty).await?;
        }
        Some(Commands::Interactive) => {
            run_interactive_mode(&mut agent, cli.pretty).await?;
        }
        None if cli.interactive => {
            run_interactive_mode(&mut agent, cli.pretty).await?;
        }
        None => {
            // Default: read from stdin (backwards compatible)
            run_stdin_mode(&mut agent, cli.pretty).await?;
        }
    }
    
    Ok(())
}

async fn execute_single_task(
    agent: &mut TinyEdgeAgent, 
    task: &str, 
    max_tokens: u32, 
    temperature: f32,
    pretty: bool
) -> Result<(), Box<dyn std::error::Error>> {
    let request = TaskRequest {
        task: task.to_string(),
        context: None,
        max_tokens: Some(max_tokens),
        temperature: Some(temperature),
    };
    
    let response = agent.execute_task(&request).await?;
    output_response(&response, pretty)?;
    
    if !response.success {
        std::process::exit(1);
    }
    
    Ok(())
}

async fn show_status(agent: &TinyEdgeAgent, pretty: bool) -> Result<(), Box<dyn std::error::Error>> {
    let health = agent.health_check().await?;
    let memory_stats = agent.get_memory_stats();
    let dispatcher_stats = agent.get_dispatcher_stats();
    
    let status = serde_json::json!({
        "version": "0.1.0",
        "status": "ready",
        "llm_loaded": health.llm_loaded,
        "total_tools": health.total_tools,
        "healthy_tools": health.tools_healthy.values().filter(|&&v| v).count(),
        "memory_usage": health.memory_usage,
        "memory_stats": memory_stats,
        "dispatcher_stats": dispatcher_stats
    });
    
    output_json(&status, pretty)?;
    Ok(())
}

fn show_tools(agent: &TinyEdgeAgent, pretty: bool) -> Result<(), Box<dyn std::error::Error>> {
    let tools = agent.get_available_tools();
    let tools_info = serde_json::json!({
        "available_tools": tools,
        "total_count": tools.len()
    });
    
    output_json(&tools_info, pretty)?;
    Ok(())
}

async fn show_health(agent: &TinyEdgeAgent, pretty: bool) -> Result<(), Box<dyn std::error::Error>> {
    let health = agent.health_check().await?;
    output_json(&health, pretty)?;
    Ok(())
}

async fn run_interactive_mode(agent: &mut TinyEdgeAgent, pretty: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nTinyEdgeLLMAgents Interactive Mode");
    println!("Type your tasks naturally, or use commands:");
    println!("  /help    - Show this help");
    println!("  /status  - Show system status");
    println!("  /tools   - List available tools");
    println!("  /health  - Run health check");
    println!("  /quit    - Exit interactive mode");
    println!();
    
    loop {
        print!("💬 > ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        match input {
            "/quit" | "/exit" | "/q" => {
                println!("👋 Goodbye!");
                break;
            }
            "/help" | "/h" => {
                println!("📖 Available commands:");
                println!("  /help, /h     - Show this help");
                println!("  /status, /s   - Show system status");
                println!("  /tools, /t    - List available tools");
                println!("  /health       - Run health check");
                println!("  /quit, /q     - Exit");
                println!("  Or just type any task naturally!");
                continue;
            }
            "/status" | "/s" => {
                show_status(agent, pretty).await?;
                continue;
            }
            "/tools" | "/t" => {
                show_tools(agent, pretty)?;
                continue;
            }
            "/health" => {
                show_health(agent, pretty).await?;
                continue;
            }
            _ => {
                // Regular task
                let request = TaskRequest {
                    task: input.to_string(),
                    context: None,
                    max_tokens: Some(100),
                    temperature: Some(0.7),
                };
                
                println!("🔄 Processing...");
                match agent.execute_task(&request).await {
                    Ok(response) => {
                        println!("✅ Result:");
                        output_response(&response, pretty)?;
                    }
                    Err(e) => {
                        println!("❌ Error: {}", e);
                    }
                }
            }
        }
        println!();
    }
    
    Ok(())
}

async fn run_stdin_mode(agent: &mut TinyEdgeAgent, pretty: bool) -> Result<(), Box<dyn std::error::Error>> {
    if atty::is(atty::Stream::Stdin) {
        println!("\n📥 Reading from stdin...");
        println!("💡 Tip: Use --interactive for interactive mode");
        println!("📖 Example: echo '{{\"task\": \"What is 2+2?\"}}' | tinyedgeagents");
        println!();
    }
    
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    
    if input.trim().is_empty() {
        eprintln!("❌ No input provided");
        std::process::exit(1);
    }
    
    // Try to parse as JSON first
    match serde_json::from_str::<TaskRequest>(&input) {
        Ok(request) => {
            let response = agent.execute_task(&request).await?;
            output_response(&response, pretty)?;
            
            if !response.success {
                std::process::exit(1);
            }
        }
        Err(_) => {
            // Treat as plain text task
            let request = TaskRequest {
                task: input.trim().to_string(),
                context: None,
                max_tokens: Some(100),
                temperature: Some(0.7),
            };
            
            let response = agent.execute_task(&request).await?;
            output_response(&response, pretty)?;
            
            if !response.success {
                std::process::exit(1);
            }
        }
    }
    
    Ok(())
}

fn output_response(response: &tinyedgellmagents::TaskResponse, pretty: bool) -> Result<(), Box<dyn std::error::Error>> {
    output_json(response, pretty)
}

fn output_json(value: &impl serde::Serialize, pretty: bool) -> Result<(), Box<dyn std::error::Error>> {
    let output = if pretty {
        serde_json::to_string_pretty(value)?
    } else {
        serde_json::to_string(value)?
    };
    
    println!("{}", output);
    Ok(())
} 