use std::io::{self, Read};
use supertinywasmllm::{SuperTinyWasmLLM, InferenceRequest, send_error_response, Result};

fn read_stdin() -> Result<String> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer.trim().to_string())
}

fn main() -> Result<()> {
    println!("SuperTinyWasmLLM starting up...");
    
    // Get model path from environment or use default
    let model_path = std::env::var("SUPERTINYWASMLLM_MODEL_PATH")
        .unwrap_or_else(|_| "model.gguf".to_string());
    
    println!("Model path: {}", model_path);
    
    // Initialize SuperTinyWasmLLM
    let mut llm = SuperTinyWasmLLM::new(model_path);
    
    // Load model
    if let Err(e) = llm.load_model() {
        eprintln!("Failed to load model: {}", e);
        send_error_response(&format!("Model loading failed: {}", e), 1)?;
        return Err(e);
    }
    
    println!("Ready for inference! Send JSON to stdin...");
    
    // Read JSON input from stdin
    let input = match read_stdin() {
        Ok(input) => input,
        Err(e) => {
            eprintln!("Failed to read stdin: {}", e);
            send_error_response(&format!("Stdin read failed: {}", e), 2)?;
            return Err(e);
        }
    };
    
    // Parse JSON request
    let request: InferenceRequest = match serde_json::from_str(&input) {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Error: {}", e);
            send_error_response(&format!("Failed to parse JSON: {}", e), 3)?;
            return Err(e.into());
        }
    };
    
    // Generate response
            match llm.generate_response(&request) {
        Ok(response) => {
            let json_response = serde_json::to_string(&response)?;
            println!("{}", json_response);
        }
        Err(e) => {
            eprintln!("Inference failed: {}", e);
            send_error_response(&format!("Inference failed: {}", e), 4)?;
            return Err(e);
        }
    }
    
    Ok(())
}