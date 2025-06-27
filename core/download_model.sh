#!/bin/bash

set -euo pipefail

echo "SuperTinyWasmLLM Model Downloader"
echo "================================="

# Configuration
DEFAULT_MODEL_URL="https://huggingface.co/mradermacher/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/TinyLlama-1.1B-Chat-v1.0.Q4_K_M.gguf"
MODEL_FILE="model.gguf"

# Alternative smaller models for testing
TINY_MODELS=(
    "https://huggingface.co/afrideva/Tinystories-gpt-0.1-3m-GGUF/resolve/main/ggml-model-Q4_K_M.gguf"
    "https://huggingface.co/keenanpepper/TinyStories-3M-fork/resolve/main/ggml-model-f16.gguf"
)

print_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -h, --help     Show this help message"
    echo "  -s, --small    Download smaller test model (~10MB)"
    echo "  -f, --force    Force re-download even if model exists"
    echo ""
    echo "Default: Downloads TinyLlama-1.1B-Chat (~637MB)"
}

download_model() {
    local url=$1
    local filename=$2
    local description=$3
    
    echo "Downloading $description..."
    echo "URL: $url"
    
    if command -v curl >/dev/null 2>&1; then
        curl -L --progress-bar --output "$filename" "$url"
    elif command -v wget >/dev/null 2>&1; then
        wget --progress=bar:force -O "$filename" "$url"
    else
        echo "Error: Neither curl nor wget found. Please install one of them."
        exit 1
    fi
}

validate_model() {
    local filename=$1
    
    if [ ! -f "$filename" ]; then
        echo "Error: Model file not found after download"
        return 1
    fi
    
    local size=$(ls -lh "$filename" | awk '{print $5}')
    echo "Downloaded model: $filename ($size)"
    
    # Basic validation - check if file is not empty and has reasonable size
    local size_bytes=$(stat -c%s "$filename" 2>/dev/null || stat -f%z "$filename" 2>/dev/null || echo "0")
    if [ "$size_bytes" -lt 1000000 ]; then  # Less than 1MB is suspicious
        echo "Warning: Downloaded file seems too small. Please verify."
        return 1
    fi
    
    echo "Model validation passed!"
    return 0
}

# Parse command line arguments
SMALL_MODEL=false
FORCE_DOWNLOAD=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            print_usage
            exit 0
            ;;
        -s|--small)
            SMALL_MODEL=true
            shift
            ;;
        -f|--force)
            FORCE_DOWNLOAD=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            print_usage
            exit 1
            ;;
    esac
done

# Check if model already exists
if [ -f "$MODEL_FILE" ] && [ "$FORCE_DOWNLOAD" = false ]; then
    echo "Model already exists: $MODEL_FILE"
    size=$(ls -lh "$MODEL_FILE" | awk '{print $5}')
    echo "Size: $size"
    echo "Use --force to re-download"
    exit 0
fi

# Remove existing model if force download
if [ "$FORCE_DOWNLOAD" = true ] && [ -f "$MODEL_FILE" ]; then
    echo "Removing existing model..."
    rm -f "$MODEL_FILE"
fi

# Select model to download
if [ "$SMALL_MODEL" = true ]; then
    echo "Downloading small test model for development..."
    
    # Try each small model URL
    for url in "${TINY_MODELS[@]}"; do
        echo "Trying: $url"
        if download_model "$url" "$MODEL_FILE" "Small test model (~10MB)"; then
            if validate_model "$MODEL_FILE"; then
                echo "Successfully downloaded small model!"
                exit 0
            else
                echo "Validation failed, trying next URL..."
                rm -f "$MODEL_FILE"
            fi
        else
            echo "Download failed, trying next URL..."
            rm -f "$MODEL_FILE"
        fi
    done
    
    echo "Error: Failed to download any small model"
    exit 1
else
    echo "Downloading default TinyLlama model..."
    if download_model "$DEFAULT_MODEL_URL" "$MODEL_FILE" "TinyLlama-1.1B (~637MB)"; then
        if validate_model "$MODEL_FILE"; then
            echo "Successfully downloaded TinyLlama model!"
            echo ""
            echo "You can now run SuperTinyWasmLLM with:"
            echo "  echo '{\"prompt\": \"Hello world\", \"max_tokens\": 50}' | \\"
            echo "    wasmedge --dir .:. --nn-preload default:GGML:AUTO:$MODEL_FILE supertinywasmllm.wasm"
        else
            rm -f "$MODEL_FILE"
            exit 1
        fi
    else
        echo "Error: Failed to download default model"
        echo "You can try the smaller model with: $0 --small"
        exit 1
    fi
fi