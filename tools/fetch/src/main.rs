use serde::{Deserialize, Serialize};
use std::io::{self, Read};

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

#[cfg(not(target_arch = "wasm32"))]
async fn perform_http_request(url: &str, method: &str) -> anyhow::Result<String> {
    // Native implementation - gerçek HTTP istekleri
    use reqwest;
    
    let client = reqwest::Client::new();
    let response = match method.to_uppercase().as_str() {
        "GET" => client.get(url).send().await?,
        "POST" => client.post(url).send().await?,
        _ => return Err(anyhow::anyhow!("Unsupported HTTP method: {}", method)),
    };
    
    let text = response.text().await?;
    Ok(text)
}

#[cfg(target_arch = "wasm32")]
fn perform_http_request_wasm(url: &str, method: &str) -> anyhow::Result<String> {
    // WASM implementation - simulated responses
    match method.to_uppercase().as_str() {
        "GET" => {
            if url.contains("httpbin.org/json") {
                Ok(r#"{"slideshow": {"title": "Sample Slide Show"}}"#.to_string())
            } else if url.contains("api.github.com") {
                Ok(r#"{"message": "API rate limit exceeded"}"#.to_string())
            } else {
                Ok(format!(r#"{{"url": "{}", "method": "GET", "simulated": true}}"#, url))
            }
        },
        "POST" => {
            Ok(format!(r#"{{"url": "{}", "method": "POST", "simulated": true, "status": "created"}}"#, url))
        },
        _ => Err(anyhow::anyhow!("Unsupported HTTP method: {}", method)),
    }
}

fn parse_url_and_method(operation: &str) -> (String, String) {
    // "GET https://example.com" -> ("https://example.com", "GET")
    // "POST https://api.example.com" -> ("https://api.example.com", "POST")
    // "https://example.com" -> ("https://example.com", "GET")
    
    let parts: Vec<&str> = operation.split_whitespace().collect();
    match parts.len() {
        1 => (parts[0].to_string(), "GET".to_string()),
        2 => (parts[1].to_string(), parts[0].to_uppercase()),
        _ => (operation.to_string(), "GET".to_string()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    
    let tool_input: ToolInput = serde_json::from_str(&input)?;
    
    let result = match tool_input.operation.as_str() {
        op if op.starts_with("http://") || op.starts_with("https://") || op.contains(" http") => {
            let (url, method) = parse_url_and_method(&tool_input.operation);
            match perform_http_request(&url, &method).await {
                Ok(response) => ToolOutput {
                    result: response,
                    status: "success".to_string(),
                    error: None,
                    metadata: Some(serde_json::json!({
                        "url": url,
                        "method": method,
                        "tool": "fetch",
                        "runtime": "native"
                    })),
                },
                Err(e) => ToolOutput {
                    result: "".to_string(),
                    status: "error".to_string(),
                    error: Some(e.to_string()),
                    metadata: None,
                },
            }
        },
        _ => ToolOutput {
            result: "".to_string(),
            status: "error".to_string(),
            error: Some("Invalid operation. Use: GET/POST <URL> or just <URL> for GET".to_string()),
            metadata: None,
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
    
    let result = match tool_input.operation.as_str() {
        op if op.starts_with("http://") || op.starts_with("https://") || op.contains(" http") => {
            let (url, method) = parse_url_and_method(&tool_input.operation);
            match perform_http_request_wasm(&url, &method) {
                Ok(response) => ToolOutput {
                    result: response,
                    status: "success".to_string(),
                    error: None,
                    metadata: Some(serde_json::json!({
                        "url": url,
                        "method": method,
                        "tool": "fetch",
                        "runtime": "wasm",
                        "simulated": true
                    })),
                },
                Err(e) => ToolOutput {
                    result: "".to_string(),
                    status: "error".to_string(),
                    error: Some(e.to_string()),
                    metadata: None,
                },
            }
        },
        _ => ToolOutput {
            result: "".to_string(),
            status: "error".to_string(),
            error: Some("Invalid operation. Use: GET/POST <URL> or just <URL> for GET".to_string()),
            metadata: None,
        },
    };
    
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
} 